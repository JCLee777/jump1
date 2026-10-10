//! Settings plugin state for user-facing preferences.

pub mod view;

use crate::plugin::{PluginId, system::SystemChange};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsSection {
    #[default]
    General,
    Shortcuts,
    Plugin(PluginId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsChange {
    System(SystemChange),
    Plugin {
        plugin: PluginId,
        key: String,
        value: String,
    },
}

#[derive(Clone)]
pub struct SettingDefinition {
    pub plugin: PluginId,
    pub key: &'static str,
    pub title: String,
    pub description: String,
    pub options: Vec<(String, String)>,
    pub selected: String,
}

pub fn matches_launcher_query(query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || ["设置", "settings", "preferences", "偏好"]
            .iter()
            .any(|keyword| keyword.contains(&query) || query.contains(keyword))
}

pub struct SettingsPlugin {
    section: SettingsSection,
}

impl SettingsPlugin {
    pub fn new() -> Self {
        Self {
            section: SettingsSection::default(),
        }
    }

    pub fn section(&self) -> SettingsSection {
        self.section
    }

    pub fn select_section(&mut self, section: SettingsSection) {
        self.section = section;
    }
}

impl crate::plugin::api::Plugin for SettingsPlugin {
    fn manifest(&self) -> crate::plugin::api::PluginManifest {
        crate::plugin::api::PluginManifest {
            id: PluginId::Settings,
            name: "Settings",
            version: "0.1.0",
            kind: crate::plugin::api::PluginKind::Page,
            permissions: &[],
        }
    }
}
