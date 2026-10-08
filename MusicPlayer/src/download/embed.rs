//! Embedded Python helper scripts for YouTube search/download, artwork scraping, and lyric fetching.
//!
//! Enables standalone binary execution without requiring the user to clone the repository.
//! When `lyra` is executed outside of the git workspace, it automatically unpacks these
//! scripts to `~/.local/share/lyra/scripts/` (or `~/.config/lyra/scripts/`).

use std::path::PathBuf;

pub struct EmbeddedFile {
    pub rel_path: &'static str,
    pub content: &'static str,
}

pub const EMBEDDED_FILES: &[EmbeddedFile] = &[
    EmbeddedFile {
        rel_path: "main.py",
        content: include_str!("../../../main.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/__init__.py",
        content: include_str!("../../../lyra/__init__.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/__main__.py",
        content: include_str!("../../../lyra/__main__.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/cli.py",
        content: include_str!("../../../lyra/cli.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/core.py",
        content: include_str!("../../../lyra/core.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/genres.py",
        content: include_str!("../../../lyra/genres.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/image.py",
        content: include_str!("../../../lyra/image.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/lyric.py",
        content: include_str!("../../../lyra/lyric.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/tagger.py",
        content: include_str!("../../../lyra/tagger.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/utils.py",
        content: include_str!("../../../lyra/utils.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/artwork/__init__.py",
        content: include_str!("../../../lyra/artwork/__init__.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/artwork/cache.py",
        content: include_str!("../../../lyra/artwork/cache.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/artwork/images.py",
        content: include_str!("../../../lyra/artwork/images.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/artwork/providers.py",
        content: include_str!("../../../lyra/artwork/providers.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/artwork/scoring.py",
        content: include_str!("../../../lyra/artwork/scoring.py"),
    },
    EmbeddedFile {
        rel_path: "lyra/artwork/textutils.py",
        content: include_str!("../../../lyra/artwork/textutils.py"),
    },
];

/// Extracts embedded scripts to `~/.local/share/lyra/scripts` if missing or outdated.
/// Returns the path to `main.py` if successful.
pub fn ensure_extracted_scripts() -> Option<PathBuf> {
    let base_dir = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".local");
                p.push("share");
                p
            })
        })
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".config");
                p
            })
        })?;

    let scripts_dir = base_dir.join("lyra").join("scripts");
    let main_py = scripts_dir.join("main.py");

    for file in EMBEDDED_FILES {
        let dest = scripts_dir.join(file.rel_path);
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let needs_write = match std::fs::read_to_string(&dest) {
            Ok(existing) => existing != file.content,
            Err(_) => true,
        };
        if needs_write {
            let _ = std::fs::write(&dest, file.content);
        }
    }

    if main_py.exists() {
        Some(main_py)
    } else {
        None
    }
}
