//! Built-in updater.
//!
//! The updater reads a small `latest.json` manifest published next to each
//! GitHub release, compares it with the running version, and can either
//! notify the user or replace the current binary (tarball installs only).
//!
//! Self-update is intentionally conservative:
//! - only Linux x86_64 release archives are selected;
//! - the manifest checksum is mandatory before anything is written;
//! - package-manager / cargo-managed binaries are never overwritten;
//! - the replacement is staged next to the current binary and atomically
//!   renamed, so a failed download cannot break the installed player.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

use crate::plugin::{cmp_versions, LYRA_VERSION};

pub const MANIFEST_URL: &str =
    "https://github.com/vincentlalo-long/lyra/releases/latest/download/latest.json";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UpdateAsset {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub assets: Vec<UpdateAsset>,
    /// Optional name -> sha256 map for simple manifests.
    #[serde(default)]
    pub sha256: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum UpdateEvent {
    Checked(Result<Option<UpdateManifest>, String>),
    Installed(Result<String, String>),
}

/// Starts a background check. `force` ignores the once-a-day cache and the
/// plugin enable/disable state (used by `lyra --check-update`).
pub fn spawn_check(tx: Sender<UpdateEvent>, force: bool) {
    std::thread::spawn(move || {
        let _ = tx.send(UpdateEvent::Checked(check_for_update(force)));
    });
}

/// Starts a background install of an already-fetched manifest.
pub fn spawn_install(tx: Sender<UpdateEvent>, manifest: UpdateManifest) {
    std::thread::spawn(move || {
        let _ = tx.send(UpdateEvent::Installed(install_update(&manifest)));
    });
}

/// Returns `Some(manifest)` when a newer release is available.
pub fn check_for_update(force: bool) -> Result<Option<UpdateManifest>, String> {
    if !force {
        if !is_enabled() || recently_checked() {
            return Ok(None);
        }
    }

    let body = fetch_text(MANIFEST_URL)?;
    let manifest: UpdateManifest = serde_json::from_str(body.trim())
        .map_err(|e| format!("invalid update manifest: {e}"))?;
    if manifest.version.trim().is_empty() {
        return Err("update manifest has no version".to_string());
    }
    mark_checked();

    if cmp_versions(LYRA_VERSION, &manifest.version) == std::cmp::Ordering::Less {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

/// Whether the updater plugin is enabled and the user has not opted out.
pub fn is_enabled() -> bool {
    if std::env::var_os("LYRA_NO_UPDATE_CHECK").is_some() {
        return false;
    }
    let plugins = crate::plugin::ManagedPlugin::load_and_merge_plugins();
    plugins
        .iter()
        .find(|p| p.id == "updater")
        .is_some_and(|p| p.enabled && !p.is_removed)
}

fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                let mut p = PathBuf::from(home);
                p.push(".cache");
                p
            })
        })?;
    Some(base.join("lyra").join("update-check"))
}

fn recently_checked() -> bool {
    let Some(path) = cache_path() else {
        return false;
    };
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    modified
        .elapsed()
        .map(|age| age.as_secs() < CHECK_INTERVAL_SECS)
        .unwrap_or(false)
}

fn mark_checked() {
    let Some(path) = cache_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, version_stamp());
}

fn version_stamp() -> String {
    format!("{}@{}", LYRA_VERSION, unix_now())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn fetch_text(url: &str) -> Result<String, String> {
    let curl = Command::new("curl")
        .args(["-fsSL", "--max-time", "20"])
        .arg(url)
        .output();
    if let Ok(out) = curl {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }
    let code = "import sys,urllib.request;print(urllib.request.urlopen(sys.argv[1],timeout=20).read().decode())";
    let out = Command::new("python3")
        .args(["-c", code, url])
        .output()
        .map_err(|e| format!("cannot fetch update manifest (need curl or python3): {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(format!(
            "cannot fetch update manifest: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn download_file(url: &str, out: &Path) -> Result<(), String> {
    if let Some(path) = url.strip_prefix("file://") {
        std::fs::copy(path, out)
            .map(|_| ())
            .map_err(|e| format!("cannot copy '{path}': {e}"))?;
        return Ok(());
    }
    let curl = Command::new("curl")
        .args(["-fsSL", "--max-time", "180", "-o"])
        .arg(out)
        .arg(url)
        .output();
    if let Ok(res) = curl {
        if res.status.success() {
            return Ok(());
        }
    }
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
            "download failed: {}",
            String::from_utf8_lossy(&res.stderr).trim()
        ))
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    if let Ok(out) = Command::new("sha256sum").arg(path).output() {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase());
        }
    }
    let code = "import sys,hashlib;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())";
    let out = Command::new("python3")
        .args(["-c", code])
        .arg(path)
        .output()
        .map_err(|_| "cannot verify checksum (need sha256sum or python3)".to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_lowercase())
}

fn is_full_build() -> bool {
    cfg!(feature = "download")
        && cfg!(feature = "mpris")
        && cfg!(feature = "notify")
        && cfg!(feature = "genre")
}

fn is_minimal_build() -> bool {
    !cfg!(feature = "download")
        && !cfg!(feature = "mpris")
        && !cfg!(feature = "notify")
        && !cfg!(feature = "genre")
}

fn target_asset(manifest: &UpdateManifest) -> Option<&UpdateAsset> {
    if !cfg!(target_os = "linux") || std::env::consts::ARCH != "x86_64" {
        return None;
    }
    let name = if is_full_build() {
        "lyra-linux-x86_64.tar.gz"
    } else if is_minimal_build() {
        "lyra-minimal-linux-x86_64.tar.gz"
    } else {
        return None;
    };
    manifest.assets.iter().find(|a| a.name == name)
}

fn asset_sha256(manifest: &UpdateManifest, asset: &UpdateAsset) -> Option<String> {
    if !asset.sha256.trim().is_empty() {
        return Some(asset.sha256.trim().to_lowercase());
    }
    manifest
        .sha256
        .get(&asset.name)
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
}

fn ensure_self_update_allowed(exe: &Path) -> Result<(), String> {
    let path = exe.to_string_lossy();
    let forced = std::env::var_os("LYRA_UPDATE_FORCE").is_some();
    let managed = path.starts_with("/usr/")
        || path.starts_with("/opt/")
        || path.starts_with("/snap/")
        || path.starts_with("/nix/store/")
        || path.contains("/.cargo/bin/");
    if managed && !forced {
        return Err(format!(
            "Lyra is managed by a package manager ({}). Update it with your package manager, or set LYRA_UPDATE_FORCE=1 to override.",
            exe.display()
        ));
    }

    let parent = exe.parent().unwrap_or_else(|| Path::new("."));
    let probe = parent.join(format!(".lyra-update-probe-{}", std::process::id()));
    std::fs::write(&probe, b"")
        .map_err(|e| format!("cannot write to '{}': {e}", parent.display()))?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| format!("cannot stat '{}': {e}", path.display()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("cannot chmod '{}': {e}", path.display()))?;
    }
    Ok(())
}

/// Downloads, verifies and atomically replaces the running binary.
/// Returns the newly installed version.
pub fn install_update(manifest: &UpdateManifest) -> Result<String, String> {
    if !cfg!(target_os = "linux") {
        return Err("self-update is only supported on Linux".to_string());
    }
    let asset = target_asset(manifest).ok_or_else(|| {
        "no compatible release asset for this build (custom builds must be updated manually)"
            .to_string()
    })?;
    let expected = asset_sha256(manifest, asset)
        .ok_or_else(|| "release manifest has no checksum; refusing to self-update".to_string())?;

    let exe = std::env::current_exe()
        .map_err(|e| format!("cannot locate current binary: {e}"))?
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_exe().unwrap_or_default());
    ensure_self_update_allowed(&exe)?;

    let work = std::env::temp_dir().join(format!("lyra-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| format!("cannot create temp dir: {e}"))?;

    let archive = work.join("lyra.tar.gz");
    download_file(&asset.url, &archive)?;
    let got = sha256_file(&archive)?;
    if got != expected {
        let _ = std::fs::remove_dir_all(&work);
        return Err(format!(
            "checksum mismatch: expected {expected}, got {got}"
        ));
    }

    let extracted = work.join("lyra");
    let tar = Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .arg("-C")
        .arg(&work)
        .output()
        .map_err(|e| format!("need 'tar' to extract update: {e}"))?;
    if !tar.status.success() || !extracted.is_file() {
        let _ = std::fs::remove_dir_all(&work);
        return Err(format!(
            "update archive does not contain a 'lyra' binary: {}",
            String::from_utf8_lossy(&tar.stderr).trim()
        ));
    }
    set_executable(&extracted)?;

    let staged = exe.with_file_name(format!(
        ".{}.update",
        exe.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "lyra".to_string())
    ));
    let _ = std::fs::remove_file(&staged);
    std::fs::copy(&extracted, &staged)
        .map_err(|e| format!("cannot stage update next to '{}': {e}", exe.display()))?;
    set_executable(&staged)?;

    let backup = exe.with_extension("old");
    let _ = std::fs::copy(&exe, &backup);
    std::fs::rename(&staged, &exe).map_err(|e| {
        let _ = std::fs::remove_file(&staged);
        format!("cannot replace '{}': {e}", exe.display())
    })?;
    let _ = std::fs::remove_dir_all(&work);

    Ok(manifest.version.clone())
}

/// Pretty CLI summary for `--check-update`.
pub fn print_check_result(manifest: &Option<UpdateManifest>) {
    match manifest {
        Some(m) => {
            println!(
                "Update available: v{} (installed v{})",
                m.version.trim_start_matches('v'),
                LYRA_VERSION
            );
            if !m.notes.trim().is_empty() {
                println!("{}", m.notes.trim());
            }
            println!("Run 'lyra --update' to install it.");
        }
        None => println!("Lyra v{LYRA_VERSION} is up to date."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str) -> UpdateManifest {
        UpdateManifest {
            version: version.to_string(),
            tag: format!("v{version}"),
            notes: String::new(),
            assets: vec![UpdateAsset {
                name: "lyra-linux-x86_64.tar.gz".to_string(),
                url: "file:///tmp/nope.tar.gz".to_string(),
                sha256: "abc".to_string(),
            }],
            sha256: BTreeMap::new(),
        }
    }

    #[test]
    fn manifest_parses_with_asset_checksums() {
        let json = r#"{
            "version": "0.3.3",
            "tag": "v0.3.3",
            "assets": [{"name":"lyra-linux-x86_64.tar.gz","url":"https://x/y.tar.gz","sha256":"deadbeef"}]
        }"#;
        let m: UpdateManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.version, "0.3.3");
        assert_eq!(m.assets[0].sha256, "deadbeef");
    }

    #[test]
    fn asset_checksum_falls_back_to_manifest_map() {
        let mut m = manifest("0.3.3");
        m.assets[0].sha256.clear();
        m.sha256.insert(
            "lyra-linux-x86_64.tar.gz".to_string(),
            "feedface".to_string(),
        );
        assert_eq!(
            asset_sha256(&m, &m.assets[0]).as_deref(),
            Some("feedface")
        );
    }

    #[test]
    fn managed_install_is_refused_without_force() {
        if std::env::var_os("LYRA_UPDATE_FORCE").is_some() {
            return;
        }
        let err = ensure_self_update_allowed(Path::new("/usr/bin/lyra")).unwrap_err();
        assert!(err.contains("package manager"));
    }

    #[test]
    fn version_comparison_is_reused_from_plugin() {
        assert_eq!(cmp_versions("0.3.2", "0.3.3"), std::cmp::Ordering::Less);
    }
}
