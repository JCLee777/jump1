//! Isolated, atomic storage for plugin-owned data.

use std::{fs, path::PathBuf};

use super::PluginId;

#[derive(Clone)]
pub struct PluginStorage {
    root: PathBuf,
}

impl PluginStorage {
    pub fn for_plugin(plugin: PluginId) -> Option<Self> {
        Some(Self {
            root: crate::platform::plugin_data_dir()?.join(plugin.slug()),
        })
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        fs::read(self.root.join(name)).ok()
    }

    pub fn read_text(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.root.join(name)).ok()
    }

    pub fn write_atomic(&self, name: &str, bytes: &[u8]) -> bool {
        if fs::create_dir_all(&self.root).is_err() {
            return false;
        }
        let target = self.root.join(name);
        let temporary = target.with_extension("tmp");
        if fs::write(&temporary, bytes).is_err() || fs::rename(&temporary, target).is_err() {
            let _ = fs::remove_file(temporary);
            return false;
        }
        true
    }

    pub fn write_text_atomic(&self, name: &str, value: &str) -> bool {
        self.write_atomic(name, value.as_bytes())
    }
}
