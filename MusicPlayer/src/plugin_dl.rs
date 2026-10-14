//! Real community-plugin installer & runner.
//!
//! The Plugin Store used to be mock-only (it flipped `is_installed` without
//! moving a single byte). This module performs an actual install:
//!
//! 1. `check_min_version` — refuses plugins needing a newer lyra.
//! 2. Source resolution — `url` (https/file) wins; otherwise a local copy
//!    from the manifest `entry` directory (repo checkout).
//! 3. sha256 verification when the manifest provides a hash.
//! 4. Extraction (`.tar.gz`) or single-file (`.py`) layout into
//!    `<data_root>/<id>/` with a `plugin.py` entrypoint + install receipt.
//! 5. `run_community_plugin` — executes an installed `plugin.py`'s
//!    `run(payload)` with a JSON payload over argv, JSON result on stdout.
//!
//! Installs run on a background thread and report through
//! [`InstallEvent`]s so the TUI never freezes mid-download.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::plugin::{check_min_version, DownloadablePlugin};

/// Events sent from the background installer thread to the main loop.
#[derive(Clone, Debug)]
pub enum InstallEvent {
    Progress { id: String, message: String },
    Done { id: String, name: String },
    Failed { id: String, message: String },
}

/// Where a plugin installs to: `<root>/<id>`.
pub fn install_dir_for(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

fn report(tx: &Option<std::sync::mpsc::Sender<InstallEvent>>, ev: InstallEvent) {
    if let Some(tx) = tx {
        let _ = tx.send(ev);
    }
}

/// Fetch `url` into `out`. Prefers `curl` (timeouts), falls back to
/// `python3 + urllib` (already a lyra dependency for the full TUI).
fn fetch_url(url: &str, out: &Path) -> Result<(), String> {
    if let Some(path) = url.strip_prefix("file://") {
        std::fs::copy(path, out)
            .map(|_| ())
            .map_err(|e| format!("cannot read local file '{path}': {e}"))?;
        return Ok(());
    }
    let curl = Command::new("curl")
        .args(["-fsSL", "--max-time", "90", "-o"])
        .arg(out)
        .arg(url)
        .output();
    match curl {
        Ok(o) if o.status.success() => return Ok(()),
        Ok(o) => {
            let detail = String::from_utf8_lossy(&o.stderr).trim().to_string();
            // Fall through to the python fallback before giving up.
            if python_fetch(url, out).is_ok() {
                return Ok(());
            }
            return Err(if detail.is_empty() {
                format!("download failed for '{url}' (curl)")
            } else {
                format!("download failed for '{url}': {detail}")
            });
        }
        Err(_) => {}
    }
    python_fetch(url, out)
}

fn python_fetch(url: &str, out: &Path) -> Result<(), String> {
    let code = "import sys,urllib.request;urllib.request.urlretrieve(sys.argv[1],sys.argv[2])";
    let res = Command::new("python3")
        .args(["-c", code, url])
        .arg(out)
        .output()
        .map_err(|e| format!("download failed (need curl or python3): {e}"))?;
    if res.status.success() && out.is_file() {
        Ok(())
    } else {
        Err(format!(
            "download failed for '{url}': {}",
            String::from_utf8_lossy(&res.stderr).trim()
        ))
    }
}

/// Verify hex sha256 via `sha256sum`, falling back to python hashlib.
fn verify_sha256(file: &Path, expected: &str) -> Result<(), String> {
    let exp = expected.trim().to_lowercase();
    if exp.is_empty() {
        return Ok(());
    }
    if let Ok(o) = Command::new("sha256sum").arg(file).output() {
        if o.status.success() {
            let got = String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            return if got == exp {
                Ok(())
            } else {
                Err(format!("checksum mismatch: expected {exp}, got {got}"))
            };
        }
    }
    let code = "import sys,hashlib;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())";
    let o = Command::new("python3")
        .args(["-c", code])
        .arg(file)
        .output()
        .map_err(|_| "cannot verify checksum (need sha256sum or python3)".to_string())?;
    let got = String::from_utf8_lossy(&o.stdout).trim().to_string();
    if got == exp {
        Ok(())
    } else {
        Err(format!("checksum mismatch: expected {exp}, got {got}"))
    }
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("cannot create '{dest:?}': {e}"))?;
    let entries =
        std::fs::read_dir(src).map_err(|e| format!("cannot list '{src:?}': {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot list '{src:?}': {e}"))?;
        let name = entry.file_name();
        if name == "__pycache__" {
            continue;
        }
        let from = entry.path();
        let to = dest.join(&name);
        let ft = entry.file_type().map_err(|e| format!("cannot stat '{from:?}': {e}"))?;
        if ft.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| format!("cannot copy '{from:?}': {e}"))?;
        }
    }
    Ok(())
}

/// Extracts a `.tar.gz` so that `plugin.py` ends up directly in `dest`
/// (a single top-level wrapper dir is stripped when present).
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("cannot create '{dest:?}': {e}"))?;
    let list = Command::new("tar")
        .args(["-tzf"])
        .arg(archive)
        .output()
        .map_err(|e| format!("need 'tar' to extract plugin: {e}"))?;
    if !list.status.success() {
        return Err("not a readable .tar.gz archive".to_string());
    }
    let names: Vec<String> = String::from_utf8_lossy(&list.stdout)
        .lines()
        .map(|l| l.trim().trim_start_matches("./").to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if names.is_empty() {
        return Err("empty plugin archive".to_string());
    }
    let first_segments: Vec<&str> = names
        .iter()
        .map(|n| n.split('/').next().unwrap_or(""))
        .collect();
    let single_root = first_segments.iter().all(|s| *s == first_segments[0])
        && names.iter().any(|n| n.contains('/'));
    let mut cmd = Command::new("tar");
    cmd.args(["-xzf"]).arg(archive).arg("-C").arg(dest);
    if single_root {
        cmd.args(["--strip-components=1"]);
    }
    let out = cmd.output().map_err(|e| format!("extract failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "extract failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

fn write_receipt(dest: &Path, plugin: &DownloadablePlugin) -> Result<(), String> {
    let receipt = serde_json::json!({
        "id": plugin.id,
        "name": plugin.name,
        "version": plugin.version,
        "installed_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    std::fs::write(dest.join(".lyra-plugin.json"), serde_json::to_string_pretty(&receipt).unwrap_or_default())
        .map_err(|e| format!("cannot write receipt: {e}"))
}

/// Blocking install of one plugin into `<root>/<id>`. Safe to run on a
/// background thread; progress goes through `tx`.
pub fn install_blocking(
    plugin: &DownloadablePlugin,
    root: &Path,
    tx: Option<std::sync::mpsc::Sender<InstallEvent>>,
) -> Result<PathBuf, String> {
    let id = plugin.id.clone();
    let prog = |m: &str| {
        report(
            &tx,
            InstallEvent::Progress { id: id.clone(), message: m.to_string() },
        )
    };

    check_min_version(&plugin.min_version)?;
    if plugin.id.trim().is_empty() || plugin.id.contains('/') || plugin.id.contains("..") {
        return Err("invalid plugin id".to_string());
    }
    let dest = install_dir_for(root, &plugin.id);

    // Source 1: explicit URL (remote or file://).
    if !plugin.url.trim().is_empty() {
        prog(&format!("Downloading {} ...", plugin.name));
        let tmp = root.join(format!(".{}.download", plugin.id));
        std::fs::create_dir_all(root).map_err(|e| format!("cannot create '{root:?}': {e}"))?;
        fetch_url(plugin.url.trim(), &tmp)?;
        verify_sha256(&tmp, &plugin.sha256)?;
        let is_tar = plugin.url.trim().ends_with(".tar.gz") || plugin.url.trim().ends_with(".tgz");
        let is_py = plugin.url.trim().ends_with(".py");
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| format!("cannot clean '{dest:?}': {e}"))?;
        }
        if is_tar {
            prog("Extracting archive ...");
            extract_tar_gz(&tmp, &dest)?;
        } else if is_py {
            std::fs::create_dir_all(&dest).map_err(|e| format!("cannot create '{dest:?}': {e}"))?;
            std::fs::copy(&tmp, dest.join("plugin.py"))
                .map_err(|e| format!("cannot stage plugin.py: {e}"))?;
        } else {
            let _ = std::fs::remove_file(&tmp);
            return Err("unsupported url: expected .tar.gz/.tgz or .py".to_string());
        }
        let _ = std::fs::remove_file(&tmp);
    } else {
        // Source 2: local copy from the manifest entry dir (repo checkout).
        let entry = plugin_entry_path(plugin);
        let Some(src) = entry else {
            return Err(format!(
                "'{}' has no download url and its '{}' folder is not on this machine; update lyra or fetch the plugin source manually",
                plugin.name, "entry"
            ));
        };
        prog("Copying local plugin files ...");
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| format!("cannot clean '{dest:?}': {e}"))?;
        }
        copy_dir(&src, &dest)?;
    }

    if !dest.join("plugin.py").is_file() {
        let _ = std::fs::remove_dir_all(&dest);
        return Err("installed plugin has no plugin.py entrypoint".to_string());
    }
    // Syntax-check the entrypoint before declaring victory.
    let check = Command::new("python3")
        .args(["-m", "py_compile"])
        .arg(dest.join("plugin.py"))
        .output();
    match check {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(format!(
                "plugin.py failed syntax check: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ));
        }
        Err(_) => {} // no python3: install anyway, run will report it
    }
    write_receipt(&dest, plugin)?;
    Ok(dest)
}

/// Best-effort removal of an installed plugin directory.
pub fn uninstall(plugin_id: &str, root: &Path) {
    if plugin_id.trim().is_empty() || plugin_id.contains('/') || plugin_id.contains("..") {
        return;
    }
    let _ = std::fs::remove_dir_all(install_dir_for(root, plugin_id));
}

/// Resolves the manifest `entry` (registry `entry` field) to a local dir.
/// The registry JSON deserializes into [`DownloadablePlugin`] without the
/// entry path, so callers pass it explicitly — see `plugin_entry_path`.
pub fn plugin_entry_path(plugin: &DownloadablePlugin) -> Option<PathBuf> {
    // Re-read the registry to find this plugin's `entry` field.
    let entry = find_registry_entry(&plugin.id)?;
    DownloadablePlugin::find_local_entry(&entry)
}

fn find_registry_entry(id: &str) -> Option<String> {
    let text = std::fs::read_to_string("registry.json")
        .or_else(|_| std::fs::read_to_string("../registry.json"))
        .or_else(|_| std::fs::read_to_string("../../registry.json"))
        .ok()
        .or_else(|| {
            std::env::var_os("HOME").and_then(|h| {
                let p = PathBuf::from(h).join("projects/lyra/registry.json");
                std::fs::read_to_string(p).ok()
            })
        })
        .unwrap_or_else(|| include_str!("../../registry.json").to_string());
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.as_array()?.iter().find_map(|item| {
        if item.get("id")?.as_str()? == id {
            item.get("entry")?.as_str().map(|s| s.to_string())
        } else {
            None
        }
    })
}

/// Runs an installed community plugin: executes `<dir>/plugin.py`'s
/// `run(payload)` with JSON on argv[1], parses JSON from stdout.
/// Returns the decoded JSON value on success.
pub fn run_community_plugin(
    plugin_id: &str,
    plugin_dir: &Path,
    payload_json: &str,
) -> Result<serde_json::Value, String> {
    let entry = plugin_dir.join("plugin.py");
    if !entry.is_file() {
        return Err(format!("'{plugin_id}' is not installed (missing plugin.py)"));
    }
    let shim = "import sys,json;sys.path.insert(0,sys.argv[1]);import plugin;print(json.dumps(plugin.run(json.loads(sys.argv[2]))))";
    let out = Command::new("python3")
        .args(["-c", shim])
        .arg(plugin_dir)
        .arg(payload_json)
        .output()
        .map_err(|_| "cannot run plugin (python3 not found)".to_string())?;
    if !out.status.success() {
        return Err(format!(
            "plugin '{plugin_id}' failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    serde_json::from_str(&text).map_err(|e| format!("plugin '{plugin_id}' returned invalid JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_plugin(id: &str) -> DownloadablePlugin {
        DownloadablePlugin {
            id: id.into(),
            name: id.into(),
            version: "v0.1.0".into(),
            author: "tests".into(),
            description: "test".into(),
            is_installed: false,
            url: String::new(),
            sha256: String::new(),
            min_version: String::new(),
        }
    }

    fn write_py(dir: &Path, body: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let mut f = std::fs::File::create(dir.join("plugin.py")).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn test_tar_gz_file_url_install_and_run() {
        let base = std::env::temp_dir().join(format!("lyra_plug_{}", std::process::id()));
        let src = base.join("srcpkg").join("myplug-1.0");
        write_py(&src, "def run(payload):\n    return {'status':'success','message':'hi '+str(payload.get('who','?')),'data':{}}\n");
        // pack a wrapper-dir archive like a GitHub release tarball
        let archive = base.join("myplug.tar.gz");
        let st = Command::new("tar")
            .args(["-czf"])
            .arg(&archive)
            .arg("-C")
            .arg(base.join("srcpkg"))
            .arg("myplug-1.0")
            .status()
            .unwrap();
        assert!(st.success());

        let mut p = test_plugin("myplug");
        p.url = format!("file://{}", archive.display());
        let root = base.join("data");
        let dest = install_blocking(&p, &root, None).expect("install must succeed");
        assert!(dest.join("plugin.py").is_file());
        assert!(dest.join(".lyra-plugin.json").is_file());

        let v = run_community_plugin("myplug", &dest, r#"{"who":"lyra"}"#).expect("run must succeed");
        assert_eq!(v.get("status").and_then(|s| s.as_str()), Some("success"));
        assert!(v.get("message").and_then(|s| s.as_str()).unwrap().contains("lyra"));

        uninstall("myplug", &root);
        assert!(!dest.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_min_version_gate_and_checksum() {
        let base = std::env::temp_dir().join(format!("lyra_plugv_{}", std::process::id()));
        let src = base.join("pkg");
        write_py(&src, "def run(payload):\n    return {}\n");
        let archive = base.join("p.tar.gz");
        assert!(Command::new("tar").args(["-czf"]).arg(&archive).arg("-C").arg(&base).arg("pkg").status().unwrap().success());

        // Future lyra requirement blocks install before any download.
        let mut p = test_plugin("future");
        p.url = format!("file://{}", archive.display());
        p.min_version = "v99.0.0".into();
        assert!(install_blocking(&p, &base.join("d"), None).is_err());

        // Wrong checksum blocks install.
        let mut q = test_plugin("badsum");
        q.url = format!("file://{}", archive.display());
        q.sha256 = "00".repeat(32);
        assert!(install_blocking(&q, &base.join("d2"), None).is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_local_entry_install_and_list() {
        // Mirrors our own bundled plugins (url empty, entry on disk).
        // cargo test runs with CWD = MusicPlayer/, so ../registry.json resolves.
        let base = std::env::temp_dir().join(format!("lyra_plugl_{}", std::process::id()));
        let mut p = test_plugin("vi-vietnamese-support");
        let Some(src) = plugin_entry_path(&p) else {
            let _ = std::fs::remove_dir_all(&base);
            return; // registry not reachable from this CWD; nothing to assert
        };
        assert!(src.join("plugin.py").is_file());
        let root = base.join("data");
        let dest = install_blocking(&p, &root, None).expect("local copy install must succeed");
        let v = run_community_plugin("vi-vietnamese-support", &dest, r#"{"action":"list"}"#)
            .expect("list probe must succeed");
        assert_eq!(v.get("status").and_then(|s| s.as_str()), Some("success"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn test_rejects_bad_ids_and_missing_entrypoint() {
        let base = std::env::temp_dir().join(format!("lyra_plugb_{}", std::process::id()));
        let mut p = test_plugin("../evil");
        assert!(install_blocking(&p, &base, None).is_err());
        p.id = "noentry".into();
        p.url = String::new(); // no url and (almost surely) no local entry dir
        // Either a local copy happens (dev checkout has no such entry) or a clear error.
        let res = install_blocking(&p, &base, None);
        assert!(res.is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
