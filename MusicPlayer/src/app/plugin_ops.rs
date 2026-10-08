use std::path::PathBuf;
use crate::browser::FileBrowser;
use super::App;

impl App {
    /// Total selectable items in the main Plugin Manager view:
    /// (Installed plugins + Configurable paths + 1 Download Plugin button)
    pub fn total_plugin_items(&self) -> usize {
        self.plugins.len() + self.config_paths.len() + 1
    }

    /// Toggles enable/disable of the selected plugin, or triggers path edit, or opens the store.
    pub fn toggle_selected_plugin(&mut self) {
        let n_plugins = self.plugins.len();
        let n_paths = self.config_paths.len();

        if self.plugin_selected < n_plugins {
            let p = &mut self.plugins[self.plugin_selected];
            if p.is_removed {
                let name = p.name.clone();
                self.set_toast(format!("Plugin '{name}' is removed. Press 'r' to restore."));
                return;
            } else {
                p.enabled = !p.enabled;
                let name = p.name.clone();
                let status = if p.enabled { "Enabled" } else { "Disabled" };
                self.set_toast(format!("Plugin '{name}': {status}"));
            }
            let id = self.plugins[self.plugin_selected].id.clone();
            let enabled = self.plugins[self.plugin_selected].enabled;
            match id.as_str() {
                "notify" => self.notifications_enabled = enabled,
                "lyrics" => self.show_lyrics = enabled,
                _ => {}
            }
        } else if self.plugin_selected < n_plugins + n_paths {
            let path_idx = self.plugin_selected - n_plugins;
            self.start_editing_path(path_idx);
        } else {
            // Option "Download Plugin"
            self.show_plugin_store = true;
            self.store_selected = 0;
            self.set_toast("Opened Plugin Store (UI Preview)".to_string());
        }
    }

    /// Removes (uninstalls / deactivates) the selected plugin from active configuration
    pub fn remove_selected_plugin(&mut self) {
        if self.plugin_selected < self.plugins.len() {
            let p = &self.plugins[self.plugin_selected];
            if !p.is_builtin {
                let id = p.id.clone();
                let name = p.name.clone();
                self.plugins.remove(self.plugin_selected);
                if let Some(sp) = self.downloadable_plugins.iter_mut().find(|item| item.id == id) {
                    sp.is_installed = false;
                }
                let total = self.total_plugin_items();
                if self.plugin_selected >= total {
                    self.plugin_selected = total.saturating_sub(1);
                }
                self.set_toast(format!("Uninstalled & permanently removed '{name}'"));
                return;
            }
            let p_mut = &mut self.plugins[self.plugin_selected];
            p_mut.is_removed = true;
            p_mut.enabled = false;
            let name = p_mut.name.clone();
            self.set_toast(format!("Removed plugin: {name} (press 'r' to restore)"));
        } else {
            self.set_toast("Paths and store cannot be removed".to_string());
        }
    }

    /// Restores a removed plugin
    pub fn restore_selected_plugin(&mut self) {
        if self.plugin_selected < self.plugins.len() {
            let p = &mut self.plugins[self.plugin_selected];
            p.is_removed = false;
            p.enabled = true;
            let name = p.name.clone();
            self.set_toast(format!("Restored plugin: {name}"));
        }
    }

    /// Starts interactive editing of the currently selected configuration path
    pub fn start_editing_selected_path(&mut self) {
        let n_plugins = self.plugins.len();
        let n_paths = self.config_paths.len();
        if self.plugin_selected >= n_plugins && self.plugin_selected < n_plugins + n_paths {
            let path_idx = self.plugin_selected - n_plugins;
            self.start_editing_path(path_idx);
        } else {
            self.set_toast("Navigate to a path item to edit (press j/k then 'e')".to_string());
        }
    }

    /// Starts interactive editing of a configuration path
    pub fn start_editing_path(&mut self, path_idx: usize) {
        if let Some(item) = self.config_paths.get(path_idx) {
            self.editing_path_index = Some(path_idx);
            self.editing_path_input = item.path.clone();
            self.set_toast(format!("Editing {}: Type new path, Enter to save, Esc to cancel", item.label));
        }
    }

    /// Confirms and saves the edited configuration path
    pub fn confirm_editing_path(&mut self) {
        if let Some(idx) = self.editing_path_index.take() {
            let new_path = self.editing_path_input.trim().to_string();
            if !new_path.is_empty() {
                if let Some(item) = self.config_paths.get_mut(idx) {
                    item.path = new_path.clone();
                    let label = item.label.clone();
                    if item.id == "music_folder" {
                        let home = std::env::var("HOME").unwrap_or_default();
                        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        let expanded = crate::config::expand_path(&new_path, &home, &cwd);
                        self.music_folder = expanded.clone();
                        self.browser = FileBrowser::new(&expanded);
                    }
                    self.set_toast(format!("Updated {label} to: {new_path}"));
                }
            } else {
                self.set_toast("Path cannot be empty. Edit cancelled.".to_string());
            }
        }
    }

    /// Cancels editing the configuration path
    pub fn cancel_editing_path(&mut self) {
        if self.editing_path_index.take().is_some() {
            self.editing_path_input.clear();
            self.set_toast("Path edit cancelled".to_string());
        }
    }

    /// In Plugin Store: installs/downloads the selected community plugin (mock UI)
    pub fn install_store_plugin(&mut self) {
        if let Some(item) = self.downloadable_plugins.get_mut(self.store_selected) {
            item.is_installed = true;
            let name = item.name.clone();
            let version = item.version.clone();
            let id = item.id.clone();
            let desc = item.description.clone();
            if !self.plugins.iter().any(|p| p.id == id) {
                self.plugins.push(crate::plugin::ManagedPlugin {
                    id,
                    name: name.clone(),
                    version: version.clone(),
                    description: desc,
                    enabled: true,
                    is_removed: false,
                    is_builtin: false,
                });
            }
            self.set_toast(format!("[Mock UI] Downloaded & installed '{name}' {version}!"));
        }
    }

    /// Adjusts or toggles the selected setting in the Plugins view.
    pub fn adjust_selected_plugin(&mut self, _increase: bool) {
        if self.plugin_selected < self.plugins.len() {
            if !self.plugins[self.plugin_selected].is_removed {
                self.toggle_selected_plugin();
            }
        } else {
            self.toggle_selected_plugin();
        }
    }
}
