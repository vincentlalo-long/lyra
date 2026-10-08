# Contributing to Lyra

Thank you for your interest in contributing to Lyra.

Lyra is a modular, pluggable terminal music player. Contributions—ranging from bug fixes and documentation improvements to new audio format decoders and modular plugins—are welcome.

---

## Repository Structure

```text
lyra/
├── MusicPlayer/        # TUI Player application (Rust / Ratatui)
├── lyra/               # Python core pipeline (audio/artwork/lyrics/cli)
├── plugins/            # Community and sample plugin scripts
│   ├── ocr_video_lyrics/  # Hardcoded video subtitle OCR extractor (in dev)
│   └── template_plugin/   # Starter boilerplate for new plugins
├── registry.json       # Manifest directory of community plugins
└── tests/              # Test suite
```

### Two Types of Plugins in Lyra

Lyra supports two plugin models:

1. **Compile-Time Core Features (Cargo Features)**:
   Deeply integrated into the Rust TUI engine (`download`, `mpris`, `notify`, `genre`). These can be enabled or compiled out at build time.
2. **Community & Script Plugins (`plugins/` & `registry.json`)**:
   Independent add-ons listed in `registry.json` and discoverable through the in-app Plugin Store (`Tab` -> Plugins -> Store).

---

## 1. Adding a Compile-Time Core Feature (Cargo Feature)

When creating a new built-in feature or modular subsystem, follow the convention established in `MusicPlayer/plugins.md`:

1. **Feature definition**: Add your feature to `MusicPlayer/Cargo.toml` under `[features]`:
   ```toml
   [features]
   default = ["download", "mpris", "notify", "genre"]
   myfeature = ["dep:optional_crate"]
   ```
   Always mark new external dependencies with `optional = true`.

2. **Module structure**:
   - Single-file: `MusicPlayer/src/myfeature.rs`
   - Multi-file (>200 lines): `MusicPlayer/src/myfeature/` containing `mod.rs`, `types.rs`, `view.rs`, etc.

3. **Gating in `main.rs`**:
   ```rust
   #[cfg(feature = "myfeature")]
   mod myfeature;
   ```

4. **App state integration**:
   - In `MusicPlayer/src/app/mod.rs`, gate fields and initialization with `#[cfg(feature = "myfeature")]`.
   - Use separate `#[cfg]` attributes for each `use` statement (attributes are not allowed inside `{...}` import groups).

5. **UI & ViewModes**:
   - If introducing a new view, add a `#[cfg(feature = "myfeature")]` variant to `ViewMode` in `MusicPlayer/src/app/types.rs`.
   - Gate matching arms in `MusicPlayer/src/ui/mod.rs` and `MusicPlayer/src/input/`.
   - In widget labels or strings, use `if cfg!(feature = "myfeature")` rather than placing `#[cfg]` inside macro expressions.

6. **Decoupling Rules**:
   - **Core must never import from a plugin**. Plugin code can depend on core utilities (`meta.rs`, `audio/`), but core code must remain completely agnostic of the plugin.
   - Use `pub(crate)` for internal items within a plugin module; expose only necessary public types at `mod.rs`.

---

## 2. Contributing a Community Plugin (`plugins/`)

Community plugins extend Lyra's capabilities via external scripts or stand-alone tools.

1. **Scaffold your plugin**:
   Copy [`plugins/template_plugin/`](plugins/template_plugin/) into `plugins/<your-plugin-id>/`.

2. **Implement functionality**:
   Write your entrypoint in `plugins/<your-plugin-id>/plugin.py` or executable script.

3. **Register in `registry.json`**:
   Add a JSON entry to [`registry.json`](registry.json) at the root of the repository:
   ```json
   {
     "id": "your-plugin-id",
     "name": "Your Plugin Name",
     "version": "v1.0.0",
     "author": "@your-github",
     "description": "Short explanation of features",
     "entry": "plugins/your-plugin-id",
     "status": "active"
   }
   ```
   Lyra's in-app Plugin Store dynamically reads `registry.json` to list available extensions for download and discovery.

4. **Document dependencies**:
   Include a `README.md` inside your plugin directory specifying required system libraries or Python packages.

---

## Coding Standards

### Zero Warnings Policy
All code must compile warning-free across all feature configurations:
```bash
# 1. Full features
cargo check --manifest-path MusicPlayer/Cargo.toml

# 2. Minimal build (no default features)
cargo check --manifest-path MusicPlayer/Cargo.toml --no-default-features

# 3. Individual feature test
cargo check --manifest-path MusicPlayer/Cargo.toml --no-default-features --features myfeature
```

### No Hardcoded Paths
Never hardcode personal or platform-specific home paths (e.g., `/home/username`). Use `dirs::home_dir()`, `dirs::config_dir()`, or the paths defined in `ConfigPathItem` (`~/.config/lyra/`).

### Test Coverage
- Unit tests for a plugin belong inside its own module using `#[cfg(test)]`.
- Tests must pass in both default and `--no-default-features` configurations:
  ```bash
  cargo test --manifest-path MusicPlayer/Cargo.toml
  cargo test --manifest-path MusicPlayer/Cargo.toml --no-default-features
  ```
- If modifying Python utilities:
  ```bash
  pytest
  ```

---

## Development Workflow

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/vincentlalo-long/lyra.git
   cd lyra
   ```

2. Create a topic branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. Implement your changes following the rules above.

4. Run formatting and verification:
   ```bash
   cargo fmt --manifest-path MusicPlayer/Cargo.toml -- --check
   cargo clippy --manifest-path MusicPlayer/Cargo.toml --all-targets -- -D warnings
   cargo test --manifest-path MusicPlayer/Cargo.toml
   cargo test --manifest-path MusicPlayer/Cargo.toml --no-default-features
   ```

5. Commit with a clear, concise commit message and open a Pull Request against `main`.
