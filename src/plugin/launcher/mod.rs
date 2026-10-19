//! Default home plugin: application, path, web, and quick-action search.

mod search;
pub mod view;

pub use crate::plugin::protocol::{ApplicationEntry, CommandAction, ResultKind, SearchResult};
pub use search::search;

use crate::plugin::api::{Plugin, PluginKind, PluginManifest, PluginPermission};

const DEFAULT_RECENT_LIMIT: usize = 6;
const MAX_RECENT_APPLICATIONS: usize = 20;
const DEFAULT_SORT_ENTRIES_BY_USAGE: bool = true;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LauncherVisibilityItem {
    Clipboard,
    Settings,
    Json,
    Yaml,
    Translate,
    HomeFolder,
    DownloadsFolder,
    DocumentsFolder,
}

impl LauncherVisibilityItem {
    const ALL: [Self; 8] = [
        Self::Clipboard,
        Self::Settings,
        Self::Json,
        Self::Yaml,
        Self::Translate,
        Self::HomeFolder,
        Self::DownloadsFolder,
        Self::DocumentsFolder,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Clipboard => "plugin_clipboard",
            Self::Settings => "plugin_settings",
            Self::Json => "plugin_json",
            Self::Yaml => "plugin_yaml",
            Self::Translate => "plugin_translate",
            Self::HomeFolder => "quick_action_home",
            Self::DownloadsFolder => "quick_action_downloads",
            Self::DocumentsFolder => "quick_action_documents",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|item| item.key() == key)
    }

    fn for_plugin(plugin: crate::plugin::PluginId) -> Option<Self> {
        match plugin {
            crate::plugin::PluginId::Clipboard => Some(Self::Clipboard),
            crate::plugin::PluginId::Settings => Some(Self::Settings),
            crate::plugin::PluginId::Json => Some(Self::Json),
            crate::plugin::PluginId::Yaml => Some(Self::Yaml),
            crate::plugin::PluginId::Translate => Some(Self::Translate),
            _ => None,
        }
    }

    fn for_result(title: &crate::plugin::protocol::ResultTitle) -> Option<Self> {
        match title {
            crate::plugin::protocol::ResultTitle::OpenHome => Some(Self::HomeFolder),
            crate::plugin::protocol::ResultTitle::OpenDownloads => Some(Self::DownloadsFolder),
            crate::plugin::protocol::ResultTitle::OpenDocuments => Some(Self::DocumentsFolder),
            _ => None,
        }
    }

    fn for_result_title(title: &crate::plugin::protocol::ResultTitle) -> Option<Self> {
        use crate::plugin::protocol::ResultTitle;

        match title {
            ResultTitle::ClipboardPlugin => Some(Self::Clipboard),
            ResultTitle::SettingsPlugin => Some(Self::Settings),
            ResultTitle::JsonPlugin => Some(Self::Json),
            ResultTitle::YamlPlugin => Some(Self::Yaml),
            ResultTitle::TranslatePlugin => Some(Self::Translate),
            _ => Self::for_result(title),
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Clipboard => 0,
            Self::Settings => 1,
            Self::Json => 2,
            Self::Yaml => 3,
            Self::Translate => 4,
            Self::HomeFolder => 5,
            Self::DownloadsFolder => 6,
            Self::DocumentsFolder => 7,
        }
    }

    fn title(self, i18n: &crate::i18n::I18n) -> String {
        use crate::{i18n::Message, plugin::protocol::ResultTitle};

        match self {
            Self::Clipboard => i18n.text(Message::ClipboardName).into(),
            Self::Settings => i18n.text(Message::SettingsName).into(),
            Self::Json => i18n.text(Message::JsonName).into(),
            Self::Yaml => i18n.text(Message::YamlName).into(),
            Self::Translate => i18n.text(Message::TranslateName).into(),
            Self::HomeFolder => i18n.result_title(&ResultTitle::OpenHome),
            Self::DownloadsFolder => i18n.result_title(&ResultTitle::OpenDownloads),
            Self::DocumentsFolder => i18n.result_title(&ResultTitle::OpenDocuments),
        }
    }

    fn is_plugin(self) -> bool {
        matches!(
            self,
            Self::Clipboard | Self::Settings | Self::Json | Self::Yaml | Self::Translate
        )
    }
}

pub struct LauncherPlugin {
    query: String,
    apps: Vec<ApplicationEntry>,
    recent_paths: Vec<std::path::PathBuf>,
    recent_limit: usize,
    sort_entries_by_usage: bool,
    entry_usage: [u64; LauncherVisibilityItem::ALL.len()],
    hidden_entries: Vec<LauncherVisibilityItem>,
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
        let sort_entries_by_usage = storage
            .as_ref()
            .and_then(|storage| storage.read_text("entry-sort.conf"))
            .map(|value| match value.trim() {
                "default" => false,
                "usage" => true,
                _ => DEFAULT_SORT_ENTRIES_BY_USAGE,
            })
            .unwrap_or(DEFAULT_SORT_ENTRIES_BY_USAGE);
        let mut entry_usage = [0; LauncherVisibilityItem::ALL.len()];
        if let Some(value) = storage
            .as_ref()
            .and_then(|storage| storage.read_text("entry-usage.conf"))
        {
            for line in value.lines() {
                let Some((key, count)) = line.split_once('=') else {
                    continue;
                };
                let (Some(item), Ok(count)) =
                    (LauncherVisibilityItem::from_key(key), count.parse::<u64>())
                else {
                    continue;
                };
                entry_usage[item.index()] = count;
            }
        }
        let hidden_entries = storage
            .as_ref()
            .and_then(|storage| storage.read_text("hidden-entries.conf"))
            .map(|value| {
                value
                    .lines()
                    .filter_map(LauncherVisibilityItem::from_key)
                    .fold(Vec::new(), |mut entries, item| {
                        if !entries.contains(&item) {
                            entries.push(item);
                        }
                        entries
                    })
            })
            .unwrap_or_default();
        Self {
            query: String::new(),
            apps: Vec::new(),
            recent_paths,
            recent_limit,
            sort_entries_by_usage,
            entry_usage,
            hidden_entries,
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
        results.extend(
            search(&self.query, &self.apps)
                .into_iter()
                .filter(|result| {
                    LauncherVisibilityItem::for_result(&result.title)
                        .is_none_or(|item| self.is_entry_visible(item))
                }),
        );
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

    pub fn record_entry_usage(&mut self, action: &crate::plugin::protocol::CommandAction) {
        use crate::plugin::protocol::CommandAction;

        let item = match action {
            CommandAction::OpenPlugin(plugin) => LauncherVisibilityItem::for_plugin(*plugin),
            CommandAction::Path(path) => crate::platform::home_dir().and_then(|home| {
                if path == &home {
                    Some(LauncherVisibilityItem::HomeFolder)
                } else if path == &home.join("Downloads") {
                    Some(LauncherVisibilityItem::DownloadsFolder)
                } else if path == &home.join("Documents") {
                    Some(LauncherVisibilityItem::DocumentsFolder)
                } else {
                    None
                }
            }),
            _ => None,
        };
        let Some(item) = item else {
            return;
        };
        let count = &mut self.entry_usage[item.index()];
        *count = count.saturating_add(1);
        self.persist_entry_usage();
    }

    pub fn sort_home_entries(&self, results: &mut [SearchResult]) {
        if !self.sort_entries_by_usage || !self.query.trim().is_empty() {
            return;
        }

        let first_entry = results
            .iter()
            .take_while(|result| result.kind == ResultKind::RecentApplication)
            .count();
        results[first_entry..].sort_by_key(|result| {
            std::cmp::Reverse(
                LauncherVisibilityItem::for_result_title(&result.title)
                    .map(|item| self.entry_usage[item.index()])
                    .unwrap_or(0),
            )
        });
    }

    pub fn setting_definitions(
        &self,
        i18n: &crate::i18n::I18n,
    ) -> Vec<crate::plugin::settings::SettingDefinition> {
        let mut definitions = vec![crate::plugin::settings::SettingDefinition {
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
        }];
        definitions.push(crate::plugin::settings::SettingDefinition {
            plugin: crate::plugin::PluginId::Launcher,
            key: "entry_sort",
            title: i18n.text(crate::i18n::Message::LauncherEntrySort).into(),
            description: i18n
                .text(crate::i18n::Message::LauncherEntrySortDescription)
                .into(),
            options: [
                (
                    "usage".to_owned(),
                    i18n.text(crate::i18n::Message::LauncherSortByUsage)
                        .to_owned(),
                ),
                (
                    "default".to_owned(),
                    i18n.text(crate::i18n::Message::LauncherSortDefault)
                        .to_owned(),
                ),
            ]
            .into_iter()
            .collect(),
            selected: if self.sort_entries_by_usage {
                "usage".to_owned()
            } else {
                "default".to_owned()
            },
        });
        definitions.extend(LauncherVisibilityItem::ALL.into_iter().map(|item| {
            let description = if item.is_plugin() {
                crate::i18n::Message::LauncherPluginVisibilityDescription
            } else {
                crate::i18n::Message::LauncherQuickActionVisibilityDescription
            };
            crate::plugin::settings::SettingDefinition {
                plugin: crate::plugin::PluginId::Launcher,
                key: item.key(),
                title: item.title(i18n),
                description: i18n.text(description).into(),
                options: [
                    (
                        "show".to_owned(),
                        i18n.text(crate::i18n::Message::LauncherVisibilityShow)
                            .to_owned(),
                    ),
                    (
                        "hide".to_owned(),
                        i18n.text(crate::i18n::Message::LauncherVisibilityHide)
                            .to_owned(),
                    ),
                ]
                .into_iter()
                .collect(),
                selected: if self.is_entry_visible(item) {
                    "show".to_owned()
                } else {
                    "hide".to_owned()
                },
            }
        }));
        definitions
    }

    pub fn apply_setting(&mut self, key: &str, value: &str) -> bool {
        if key == "entry_sort" {
            self.sort_entries_by_usage = match value {
                "usage" => true,
                "default" => false,
                _ => return false,
            };
            self.persist_entry_sort();
            return true;
        }

        if key == "recent_limit" {
            let Ok(limit) = value.parse::<usize>() else {
                return false;
            };
            if !matches!(limit, 4 | 6 | 8 | 10) {
                return false;
            }
            self.recent_limit = limit;
            self.persist_settings();
            return true;
        }

        let Some(item) = LauncherVisibilityItem::from_key(key) else {
            return false;
        };
        match value {
            "show" => self.hidden_entries.retain(|hidden| *hidden != item),
            "hide" if !self.hidden_entries.contains(&item) => self.hidden_entries.push(item),
            "hide" => {}
            _ => return false,
        }
        self.persist_hidden_entries();
        true
    }

    pub fn is_plugin_entry_visible(&self, plugin: crate::plugin::PluginId) -> bool {
        LauncherVisibilityItem::for_plugin(plugin).is_none_or(|item| self.is_entry_visible(item))
    }

    fn is_entry_visible(&self, item: LauncherVisibilityItem) -> bool {
        !self.hidden_entries.contains(&item)
    }

    fn recent_results(&self) -> Vec<SearchResult> {
        self.recent_paths
            .iter()
            .filter_map(|path| self.apps.iter().find(|app| app.path == *path))
            .take(self.recent_limit)
            .map(|app| SearchResult {
                title: crate::plugin::protocol::ResultTitle::Application(app.name.clone()),
                detail: crate::plugin::protocol::ResultDetail::Literal(
                    if app.path.to_string_lossy().starts_with("shell:AppsFolder\\") {
                        String::new()
                    } else {
                        app.path.display().to_string()
                    },
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

    fn persist_hidden_entries(&self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let hidden = LauncherVisibilityItem::ALL
            .into_iter()
            .filter(|item| self.hidden_entries.contains(item))
            .map(LauncherVisibilityItem::key)
            .collect::<Vec<_>>()
            .join("\n");
        let _ = storage.write_text_atomic("hidden-entries.conf", &hidden);
    }

    fn persist_entry_sort(&self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let value = if self.sort_entries_by_usage {
            "usage"
        } else {
            "default"
        };
        let _ = storage.write_text_atomic("entry-sort.conf", value);
    }

    fn persist_entry_usage(&self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let usage = LauncherVisibilityItem::ALL
            .into_iter()
            .filter_map(|item| {
                let count = self.entry_usage[item.index()];
                (count > 0).then(|| format!("{}={count}", item.key()))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let _ = storage.write_text_atomic("entry-usage.conf", &usage);
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
