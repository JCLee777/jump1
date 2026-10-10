//! Default home plugin: application, path, web, and quick-action search.

mod search;
pub mod view;

pub use crate::plugin::protocol::{ApplicationEntry, CommandAction, ResultKind, SearchResult};
pub use search::search;

use crate::plugin::api::{Plugin, PluginKind, PluginManifest, PluginPermission};

const DEFAULT_RECENT_LIMIT: usize = 6;
const MAX_RECENT_APPLICATIONS: usize = 20;

pub struct LauncherPlugin {
    query: String,
    apps: Vec<ApplicationEntry>,
    recent_paths: Vec<std::path::PathBuf>,
    recent_limit: usize,
    storage: Option<crate::plugin::storage::PluginStorage>,
}

impl LauncherPlugin {
    pub fn load(storage: Option<crate::plugin::storage::PluginStorage>) -> Self {
        let recent_limit = storage
            .as_ref()
            .and_then(|storage| storage.read_text("settings.conf"))
            .and_then(|value| value.trim().parse().ok())
            .filter(|value| matches!(value, 4 | 6 | 8 | 10))
            .unwrap_or(DEFAULT_RECENT_LIMIT);
        let recent_paths = storage
            .as_ref()
            .and_then(|storage| storage.read("recent.bin"))
            .map(|bytes| decode_recent_paths(&bytes))
            .unwrap_or_default();
        Self {
            query: String::new(),
            apps: Vec::new(),
            recent_paths,
            recent_limit,
            storage,
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
    }

    pub fn results(&self) -> Vec<SearchResult> {
        let mut results = Vec::new();
        if self.query.trim().is_empty() {
            results.extend(self.recent_results());
        }
        results.extend(search(&self.query, &self.apps));
        results
    }

    pub fn app_count(&self) -> usize {
        self.apps.len()
    }

    pub fn set_apps(&mut self, apps: Vec<ApplicationEntry>) {
        self.apps = apps;
    }

    pub fn record_application(&mut self, application: &ApplicationEntry) {
        self.recent_paths.retain(|path| path != &application.path);
        self.recent_paths.insert(0, application.path.clone());
        self.recent_paths.truncate(MAX_RECENT_APPLICATIONS);
        self.persist_recent();
    }

    pub fn setting_definitions(
        &self,
        i18n: &crate::i18n::I18n,
    ) -> Vec<crate::plugin::settings::SettingDefinition> {
        vec![crate::plugin::settings::SettingDefinition {
            plugin: crate::plugin::PluginId::Launcher,
            key: "recent_limit",
            title: i18n.text(crate::i18n::Message::LauncherRecentLimit).into(),
            description: i18n
                .text(crate::i18n::Message::LauncherRecentLimitDescription)
                .into(),
            options: [4, 6, 8, 10]
                .into_iter()
                .map(|value| (value.to_string(), value.to_string()))
                .collect(),
            selected: self.recent_limit.to_string(),
        }]
    }

    pub fn apply_setting(&mut self, key: &str, value: &str) -> bool {
        if key != "recent_limit" {
            return false;
        }
        let Ok(limit) = value.parse::<usize>() else {
            return false;
        };
        if !matches!(limit, 4 | 6 | 8 | 10) {
            return false;
        }
        self.recent_limit = limit;
        self.persist_settings();
        true
    }

    fn recent_results(&self) -> Vec<SearchResult> {
        self.recent_paths
            .iter()
            .filter_map(|path| self.apps.iter().find(|app| app.path == *path))
            .take(self.recent_limit)
            .map(|app| SearchResult {
                title: crate::plugin::protocol::ResultTitle::Application(app.name.clone()),
                detail: crate::plugin::protocol::ResultDetail::Literal(
                    app.path.display().to_string(),
                ),
                kind: ResultKind::RecentApplication,
                icon_path: app.icon_path.clone(),
                action: CommandAction::Application(app.clone()),
            })
            .collect()
    }

    fn persist_recent(&self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let bytes = self
            .recent_paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\0");
        let _ = storage.write_atomic("recent.bin", bytes.as_bytes());
    }

    fn persist_settings(&self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let _ = storage.write_text_atomic("settings.conf", &self.recent_limit.to_string());
    }
}

fn decode_recent_paths(bytes: &[u8]) -> Vec<std::path::PathBuf> {
    String::from_utf8_lossy(bytes)
        .split('\0')
        .filter(|path| !path.is_empty())
        .take(MAX_RECENT_APPLICATIONS)
        .map(std::path::PathBuf::from)
        .collect()
}

impl Plugin for LauncherPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: crate::plugin::PluginId::Launcher,
            name: "Launcher",
            version: "0.1.0",
            kind: PluginKind::Page,
            permissions: &[
                PluginPermission::ProcessLaunch,
                PluginPermission::Filesystem,
            ],
        }
    }
}
