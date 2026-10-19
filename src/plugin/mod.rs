//! Built-in plugin identities and implementations.

pub mod api;
pub mod clipboard;
pub mod compare;
pub mod host_view;
pub mod json;
pub mod launcher;
pub mod markdown;
pub mod protocol;
pub mod settings;
pub mod shortcuts;
pub mod storage;
pub mod system;
pub mod translate;
pub mod yaml;

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use gpui_kit::{App, ClipboardItem, FocusHandle, Window};

use self::{
    api::{Plugin, PluginKind, PluginLifecycle, PluginManifest},
    clipboard::{ClipboardPlugin, ClipboardPreviewImageSource},
    json::JsonPlugin,
    launcher::LauncherPlugin,
    markdown::MarkdownPlugin,
    protocol::{CommandAction, HostEffect, ResultDetail, ResultKind, ResultTitle, SearchResult},
    settings::{SettingDefinition, SettingsChange, SettingsPlugin, SettingsSection},
    shortcuts::{ShortcutBindingEvent, ShortcutPlugin},
    storage::PluginStorage,
    system::SystemPlugin,
    translate::TranslatePlugin,
    yaml::YamlPlugin,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginId {
    Launcher,
    Clipboard,
    Json,
    Yaml,
    Markdown,
    Translate,
    Settings,
    System,
    Shortcuts,
}

impl PluginId {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Launcher => "launcher",
            Self::Clipboard => "clipboard",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Markdown => "markdown",
            Self::Translate => "translate",
            Self::Settings => "settings",
            Self::System => "system",
            Self::Shortcuts => "shortcuts",
        }
    }
}

#[derive(Default)]
pub struct PluginRegistry {
    manifests: Vec<PluginManifest>,
    lifecycles: Vec<(PluginId, PluginLifecycle)>,
}

impl PluginRegistry {
    pub fn register(&mut self, manifest: PluginManifest) {
        if !self.contains(manifest.id) {
            self.lifecycles
                .push((manifest.id, PluginLifecycle::Registered));
            self.manifests.push(manifest);
        }
    }

    pub fn set_lifecycle(&mut self, id: PluginId, lifecycle: PluginLifecycle) {
        if let Some((_, state)) = self.lifecycles.iter_mut().find(|(plugin, _)| *plugin == id) {
            *state = lifecycle;
        }
    }

    pub fn lifecycle(&self, id: PluginId) -> Option<&PluginLifecycle> {
        self.lifecycles
            .iter()
            .find(|(plugin, _)| *plugin == id)
            .map(|(_, state)| state)
    }

    pub fn contains(&self, id: PluginId) -> bool {
        self.manifests.iter().any(|manifest| manifest.id == id)
    }

    pub fn manifest(&self, id: PluginId) -> Option<&PluginManifest> {
        self.manifests.iter().find(|manifest| manifest.id == id)
    }
}

pub enum PluginCommand {
    None,
    Action(CommandAction),
    PasteClipboard(clipboard::ClipboardSelection),
}

pub enum PluginEvent {
    Action(CommandAction),
    Back,
    PasteClipboard(usize),
    ClipboardSort(clipboard::ClipboardSortMode),
    ClipboardFilter(clipboard::ClipboardFilterMode),
    ToggleClipboardFavorite(usize),
    ClearClipboard,
    WriteClipboard(String),
    ImportClipboard(PluginId),
    Settings(SettingsChange),
    SettingsSection(SettingsSection),
    CheckForUpdates,
    DownloadUpdate {
        version: String,
        url: String,
    },
    InstallUpdate {
        version: String,
        package_path: std::path::PathBuf,
    },
    ShortcutBinding(ShortcutBindingEvent),
}

#[derive(Clone, Default)]
pub(crate) struct ExternalOpenGuard(Arc<Mutex<Option<Instant>>>);

impl ExternalOpenGuard {
    pub(crate) fn mark_opening(&self) {
        if let Ok(mut deadline) = self.0.lock() {
            *deadline = Some(Instant::now() + Duration::from_secs(5));
        }
    }

    pub(crate) fn should_preserve_panel(&self) -> bool {
        let Ok(mut deadline) = self.0.lock() else {
            return false;
        };
        if deadline.is_some_and(|deadline| Instant::now() < deadline) {
            true
        } else {
            *deadline = None;
            false
        }
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut deadline) = self.0.lock() {
            *deadline = None;
        }
    }
}

pub type PluginEventHandler = Rc<dyn for<'a, 'b> Fn(PluginEvent, &'a mut Window, &'b mut App)>;

pub struct PluginManager {
    registry: PluginRegistry,
    active: PluginId,
    launcher: LauncherPlugin,
    clipboard: ClipboardPlugin,
    json: JsonPlugin,
    yaml: YamlPlugin,
    markdown: MarkdownPlugin,
    translate: TranslatePlugin,
    settings: SettingsPlugin,
    system: SystemPlugin,
    shortcuts: ShortcutPlugin,
}

impl PluginManager {
    pub fn load(
        preferences: crate::state::Preferences,
        on_event: PluginEventHandler,
        external_open_guard: ExternalOpenGuard,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let launcher = LauncherPlugin::load(PluginStorage::for_plugin(PluginId::Launcher));
        let clipboard = ClipboardPlugin::load(PluginStorage::for_plugin(PluginId::Clipboard));
        let json = JsonPlugin::load(
            PluginStorage::for_plugin(PluginId::Json),
            preferences.locale,
            on_event.clone(),
            window,
            cx,
        );
        let yaml = YamlPlugin::load(
            PluginStorage::for_plugin(PluginId::Yaml),
            preferences.locale,
            on_event.clone(),
            window,
            cx,
        );
        let translate = TranslatePlugin::load(
            PluginStorage::for_plugin(PluginId::Translate),
            preferences.locale,
            on_event.clone(),
            window,
            cx,
        );
        let markdown = MarkdownPlugin::load(
            PluginStorage::for_plugin(PluginId::Markdown),
            preferences.locale,
            on_event,
            external_open_guard,
            window,
            cx,
        );
        let settings = SettingsPlugin::new();
        let system = SystemPlugin::new(
            preferences.theme,
            preferences.accent_color,
            preferences.locale,
            crate::platform::launch_at_login_enabled(),
            preferences.panel_size,
        );
        let shortcuts = ShortcutPlugin::load(PluginStorage::for_plugin(PluginId::Shortcuts));
        let mut registry = PluginRegistry::default();
        registry.register(launcher.manifest());
        registry.register(clipboard.manifest());
        registry.register(json.manifest());
        registry.register(yaml.manifest());
        registry.register(markdown.manifest());
        registry.register(translate.manifest());
        registry.register(settings.manifest());
        registry.register(system.manifest());
        registry.register(shortcuts.manifest());
        let mut manager = Self {
            registry,
            active: PluginId::Launcher,
            launcher,
            clipboard,
            json,
            yaml,
            markdown,
            translate,
            settings,
            system,
            shortcuts,
        };
        manager.load_plugins();
        let _ = manager.activate_plugin(PluginId::System);
        let _ = manager.activate_plugin(PluginId::Clipboard);
        let _ = manager.activate_plugin(PluginId::Shortcuts);
        let _ = manager.activate_plugin(PluginId::Launcher);
        manager
    }

    pub fn active(&self) -> PluginId {
        self.active
    }

    pub fn input_placeholder(&self, i18n: &crate::i18n::I18n) -> &'static str {
        match self.active {
            PluginId::Launcher => i18n.text(crate::i18n::Message::SearchPlaceholder),
            PluginId::Clipboard => i18n.text(crate::i18n::Message::ClipboardSearchPlaceholder),
            PluginId::Json => i18n.text(crate::i18n::Message::JsonName),
            PluginId::Yaml => i18n.text(crate::i18n::Message::YamlName),
            PluginId::Markdown => i18n.text(crate::i18n::Message::MarkdownName),
            PluginId::Translate => i18n.text(crate::i18n::Message::TranslateName),
            PluginId::Settings | PluginId::System | PluginId::Shortcuts => {
                i18n.text(crate::i18n::Message::SettingsName)
            }
        }
    }

    pub fn launcher(&self) -> &LauncherPlugin {
        &self.launcher
    }

    pub fn clipboard(&self) -> &ClipboardPlugin {
        &self.clipboard
    }

    pub fn open_clipboard_preview(&mut self, index: usize) {
        self.clipboard.open_preview(index);
    }

    pub fn close_clipboard_preview(&mut self) {
        self.clipboard.close_preview();
    }

    pub fn update_clipboard_preview(&mut self, index: usize) {
        self.clipboard.update_preview_image(index);
    }

    pub(crate) fn track_clipboard_preview_asset(&mut self, source: ClipboardPreviewImageSource) {
        self.clipboard.track_preview_asset(source);
    }

    pub fn leave_clipboard_page(&mut self, current_window: Option<&mut Window>, cx: &mut App) {
        self.clipboard.leave_page(current_window, cx);
    }

    pub fn json(&self) -> &JsonPlugin {
        &self.json
    }

    pub fn yaml(&self) -> &YamlPlugin {
        &self.yaml
    }

    pub fn markdown(&self) -> &MarkdownPlugin {
        &self.markdown
    }

    pub fn translate(&self) -> &TranslatePlugin {
        &self.translate
    }

    pub fn system(&self) -> &SystemPlugin {
        &self.system
    }

    pub fn settings(&self) -> &SettingsPlugin {
        &self.settings
    }

    pub fn shortcuts(&self) -> &ShortcutPlugin {
        &self.shortcuts
    }

    pub fn shortcuts_mut(&mut self) -> &mut ShortcutPlugin {
        &mut self.shortcuts
    }

    pub fn set_apps(&mut self, apps: Vec<protocol::ApplicationEntry>) {
        self.launcher.set_apps(apps);
    }

    pub fn select_settings_section(&mut self, section: SettingsSection) {
        self.settings.select_section(section);
    }

    pub fn set_query(&mut self, query: String) {
        match self.active {
            PluginId::Launcher => self.launcher.set_query(query),
            PluginId::Clipboard => self.clipboard.set_query(query),
            PluginId::Json => {}
            PluginId::Yaml => {}
            PluginId::Markdown => {}
            PluginId::Translate => {}
            PluginId::Settings => {}
            PluginId::System => {}
            PluginId::Shortcuts => {}
        }
    }

    pub fn clear_queries(&mut self) {
        self.launcher.clear_query();
        self.clipboard.clear_query();
    }

    pub fn current_count(&self) -> usize {
        match self.active {
            PluginId::Launcher => self.results().len(),
            PluginId::Clipboard => self.clipboard.current_entries().len(),
            PluginId::Json => 0,
            PluginId::Yaml => 0,
            PluginId::Markdown => 0,
            PluginId::Translate => 0,
            PluginId::Settings => 0,
            PluginId::System => 0,
            PluginId::Shortcuts => 0,
        }
    }

    pub fn results(&self) -> Vec<SearchResult> {
        let mut results = self.launcher.results();
        let mut plugin_results = Vec::new();
        let keep_plugin_searchable = !self.launcher.query().trim().is_empty();
        if self.registry.contains(PluginId::Clipboard)
            && (keep_plugin_searchable
                || self.launcher.is_plugin_entry_visible(PluginId::Clipboard))
            && ClipboardPlugin::matches_launcher_query(self.launcher.query())
        {
            plugin_results.push(SearchResult {
                title: ResultTitle::ClipboardPlugin,
                detail: ResultDetail::ClipboardPlugin,
                kind: ResultKind::Plugin,
                icon_path: None,
                action: CommandAction::OpenPlugin(PluginId::Clipboard),
            });
        }
        if self.registry.contains(PluginId::Settings)
            && (keep_plugin_searchable || self.launcher.is_plugin_entry_visible(PluginId::Settings))
            && crate::plugin::settings::matches_launcher_query(self.launcher.query())
        {
            plugin_results.push(SearchResult {
                title: ResultTitle::SettingsPlugin,
                detail: ResultDetail::SettingsPlugin,
                kind: ResultKind::Plugin,
                icon_path: None,
                action: CommandAction::OpenPlugin(PluginId::Settings),
            });
        }
        if self.registry.contains(PluginId::Json)
            && (keep_plugin_searchable || self.launcher.is_plugin_entry_visible(PluginId::Json))
            && JsonPlugin::matches_launcher_query(self.launcher.query())
        {
            plugin_results.push(SearchResult {
                title: ResultTitle::JsonPlugin,
                detail: ResultDetail::JsonPlugin,
                kind: ResultKind::Plugin,
                icon_path: None,
                action: CommandAction::OpenPlugin(PluginId::Json),
            });
        }
        if self.registry.contains(PluginId::Yaml)
            && (keep_plugin_searchable || self.launcher.is_plugin_entry_visible(PluginId::Yaml))
            && YamlPlugin::matches_launcher_query(self.launcher.query())
        {
            plugin_results.push(SearchResult {
                title: ResultTitle::YamlPlugin,
                detail: ResultDetail::YamlPlugin,
                kind: ResultKind::Plugin,
                icon_path: None,
                action: CommandAction::OpenPlugin(PluginId::Yaml),
            });
        }
        if self.registry.contains(PluginId::Markdown)
            && (keep_plugin_searchable || self.launcher.is_plugin_entry_visible(PluginId::Markdown))
            && MarkdownPlugin::matches_launcher_query(self.launcher.query())
        {
            plugin_results.push(SearchResult {
                title: ResultTitle::MarkdownPlugin,
                detail: ResultDetail::MarkdownPlugin,
                kind: ResultKind::Plugin,
                icon_path: None,
                action: CommandAction::OpenPlugin(PluginId::Markdown),
            });
        }
        if self.registry.contains(PluginId::Translate)
            && (keep_plugin_searchable
                || self.launcher.is_plugin_entry_visible(PluginId::Translate))
            && TranslatePlugin::matches_launcher_query(self.launcher.query())
        {
            plugin_results.push(SearchResult {
                title: ResultTitle::TranslatePlugin,
                detail: ResultDetail::TranslatePlugin,
                kind: ResultKind::Plugin,
                icon_path: None,
                action: CommandAction::OpenPlugin(PluginId::Translate),
            });
        }
        let insertion = results
            .iter()
            .position(|result| result.kind == ResultKind::Web)
            .unwrap_or(results.len());
        results.splice(insertion..insertion, plugin_results);
        self.launcher.sort_home_entries(&mut results);
        results
    }

    pub fn activate(&mut self, index: usize) -> PluginCommand {
        match self.active {
            PluginId::Launcher => self
                .results()
                .get(index)
                .map(|result| result.action.clone())
                .map(PluginCommand::Action)
                .unwrap_or(PluginCommand::None),
            PluginId::Clipboard => self
                .clipboard
                .activate_entry(index)
                .map(PluginCommand::PasteClipboard)
                .unwrap_or(PluginCommand::None),
            PluginId::Json => PluginCommand::None,
            PluginId::Yaml => PluginCommand::None,
            PluginId::Markdown => PluginCommand::None,
            PluginId::Translate => PluginCommand::None,
            PluginId::Settings => PluginCommand::None,
            PluginId::System => PluginCommand::None,
            PluginId::Shortcuts => PluginCommand::None,
        }
    }

    pub fn open(&mut self, plugin: PluginId, current_window: Option<&mut Window>, cx: &mut App) {
        if !self.registry.contains(plugin) || self.active == plugin {
            return;
        }
        let previous = self.active;
        if !self.runs_in_background(previous) {
            self.suspend_plugin(previous);
        }
        let ready = matches!(
            self.registry.lifecycle(plugin),
            Some(PluginLifecycle::Active)
        ) || self.activate_plugin(plugin);
        if ready {
            if previous == PluginId::Clipboard && plugin != PluginId::Clipboard {
                self.clipboard.leave_page(current_window, cx);
            }
            self.active = plugin;
        } else if !self.runs_in_background(previous) {
            let _ = self.activate_plugin(previous);
        }
    }

    fn runs_in_background(&self, plugin: PluginId) -> bool {
        self.registry.manifest(plugin).is_some_and(|manifest| {
            matches!(manifest.kind, PluginKind::Service | PluginKind::Hybrid)
        })
    }

    fn load_plugins(&mut self) {
        for plugin in [
            PluginId::Launcher,
            PluginId::Clipboard,
            PluginId::Json,
            PluginId::Yaml,
            PluginId::Markdown,
            PluginId::Translate,
            PluginId::Settings,
            PluginId::System,
            PluginId::Shortcuts,
        ] {
            let result = match plugin {
                PluginId::Launcher => self.launcher.on_load(),
                PluginId::Clipboard => self.clipboard.on_load(),
                PluginId::Json => self.json.on_load(),
                PluginId::Yaml => self.yaml.on_load(),
                PluginId::Markdown => self.markdown.on_load(),
                PluginId::Translate => self.translate.on_load(),
                PluginId::Settings => self.settings.on_load(),
                PluginId::System => self.system.on_load(),
                PluginId::Shortcuts => self.shortcuts.on_load(),
            };
            self.record_lifecycle_result(plugin, result, PluginLifecycle::Loaded);
        }
    }

    fn activate_plugin(&mut self, plugin: PluginId) -> bool {
        if matches!(
            self.registry.lifecycle(plugin),
            Some(PluginLifecycle::Failed(_)) | Some(PluginLifecycle::Stopped)
        ) {
            return false;
        }
        let result = match plugin {
            PluginId::Launcher => self.launcher.on_activate(),
            PluginId::Clipboard => self.clipboard.on_activate(),
            PluginId::Json => self.json.on_activate(),
            PluginId::Yaml => self.yaml.on_activate(),
            PluginId::Markdown => self.markdown.on_activate(),
            PluginId::Translate => self.translate.on_activate(),
            PluginId::Settings => self.settings.on_activate(),
            PluginId::System => self.system.on_activate(),
            PluginId::Shortcuts => self.shortcuts.on_activate(),
        };
        let activated = result.is_ok();
        self.record_lifecycle_result(plugin, result, PluginLifecycle::Active);
        activated
    }

    fn suspend_plugin(&mut self, plugin: PluginId) {
        let result = match plugin {
            PluginId::Launcher => self.launcher.on_suspend(),
            PluginId::Clipboard => self.clipboard.on_suspend(),
            PluginId::Json => self.json.on_suspend(),
            PluginId::Yaml => self.yaml.on_suspend(),
            PluginId::Markdown => self.markdown.on_suspend(),
            PluginId::Translate => self.translate.on_suspend(),
            PluginId::Settings => self.settings.on_suspend(),
            PluginId::System => self.system.on_suspend(),
            PluginId::Shortcuts => self.shortcuts.on_suspend(),
        };
        self.record_lifecycle_result(plugin, result, PluginLifecycle::Suspended);
    }

    fn record_lifecycle_result(
        &mut self,
        plugin: PluginId,
        result: Result<(), String>,
        success: PluginLifecycle,
    ) {
        self.registry.set_lifecycle(
            plugin,
            result.map_or_else(PluginLifecycle::Failed, |_| success),
        );
    }

    pub fn capture_clipboard(&mut self, item: ClipboardItem) -> bool {
        if matches!(
            self.registry.lifecycle(PluginId::Clipboard),
            Some(PluginLifecycle::Active)
        ) {
            self.clipboard.capture(item)
        } else {
            false
        }
    }

    pub fn set_clipboard_sort(&mut self, mode: clipboard::ClipboardSortMode) {
        self.clipboard.set_sort_mode(mode);
    }

    pub fn set_clipboard_filter(&mut self, mode: clipboard::ClipboardFilterMode) {
        self.clipboard.set_filter_mode(mode);
    }

    pub fn toggle_clipboard_favorite(&mut self, index: usize) -> bool {
        self.clipboard.toggle_favorite(index)
    }

    pub fn clear_clipboard_history(&mut self, current_clipboard: Option<ClipboardItem>) {
        self.clipboard.clear(current_clipboard);
    }

    pub fn ignore_clipboard_item(&mut self, item: Option<ClipboardItem>) {
        self.clipboard.ignore_item(item);
    }

    pub fn setting_definitions(&self, i18n: &crate::i18n::I18n) -> Vec<SettingDefinition> {
        let mut definitions = self.launcher.setting_definitions(i18n);
        definitions.extend(self.clipboard.setting_definitions(i18n));
        definitions.extend(self.json.setting_definitions(i18n));
        definitions.extend(self.yaml.setting_definitions(i18n));
        definitions
    }

    pub fn record_application_usage(&mut self, application: &protocol::ApplicationEntry) {
        self.launcher.record_application(application);
    }

    pub fn record_launcher_entry_usage(&mut self, action: &protocol::CommandAction) {
        self.launcher.record_entry_usage(action);
    }

    pub fn apply_setting(
        &mut self,
        change: &SettingsChange,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<HostEffect> {
        match change {
            SettingsChange::System(change) => match change {
                crate::plugin::system::SystemChange::Theme(theme) => {
                    self.system.apply(*change);
                    self.save_system_preferences();
                    vec![HostEffect::ApplyTheme(*theme)]
                }
                crate::plugin::system::SystemChange::AccentColor(accent) => {
                    self.system.apply(*change);
                    self.save_system_preferences();
                    vec![HostEffect::ApplyAccentColor(*accent)]
                }
                crate::plugin::system::SystemChange::Locale(locale) => {
                    self.system.apply(*change);
                    self.save_system_preferences();
                    self.json.set_locale(*locale, cx);
                    self.yaml.set_locale(*locale, cx);
                    self.markdown.set_locale(*locale, cx);
                    vec![HostEffect::ApplyLocale(*locale)]
                }
                crate::plugin::system::SystemChange::PanelSizePreset(_)
                | crate::plugin::system::SystemChange::CustomPanelSize { .. } => {
                    self.system.apply(*change);
                    self.save_system_preferences();
                    vec![HostEffect::ResizePanel(self.system.panel_size())]
                }
                crate::plugin::system::SystemChange::LaunchAtLogin(enabled) => {
                    let result = crate::platform::set_launch_at_login(*enabled);
                    if result.is_ok() {
                        self.system.apply(*change);
                    }
                    vec![HostEffect::LaunchAtLogin(result.map(|_| *enabled))]
                }
            },
            SettingsChange::Plugin { plugin, key, value } => match plugin {
                PluginId::Launcher => {
                    self.launcher.apply_setting(key, value);
                    Vec::new()
                }
                PluginId::Clipboard => {
                    self.clipboard.apply_setting(key, value);
                    Vec::new()
                }
                PluginId::Json => {
                    self.json.apply_setting(key, value, window, cx);
                    Vec::new()
                }
                PluginId::Yaml => {
                    self.yaml.apply_setting(key, value, window, cx);
                    Vec::new()
                }
                _ => Vec::new(),
            },
        }
    }

    fn save_system_preferences(&self) {
        crate::state::Preferences {
            theme: self.system.theme(),
            accent_color: self.system.accent_color(),
            locale: self.system.locale(),
            panel_size: self.system.panel_size(),
        }
        .save();
    }

    pub fn active_focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        match self.active {
            PluginId::Json => Some(self.json.focus_handle(cx)),
            PluginId::Yaml => Some(self.yaml.focus_handle(cx)),
            PluginId::Markdown => Some(self.markdown.focus_handle(cx)),
            PluginId::Translate => Some(self.translate.focus_handle(cx)),
            _ => None,
        }
    }

    pub fn import_clipboard(
        &self,
        plugin: PluginId,
        text: String,
        window: &mut Window,
        cx: &mut App,
    ) {
        if plugin == PluginId::Json {
            self.json.import_text(text, window, cx);
        } else if plugin == PluginId::Yaml {
            self.yaml.import_text(text, window, cx);
        } else if plugin == PluginId::Markdown {
            self.markdown.import_text(text, window, cx);
        } else if plugin == PluginId::Translate {
            self.translate.import_text(text, window, cx);
        }
    }

    pub fn prepare_open(
        &self,
        plugin: PluginId,
        clipboard_text: Option<String>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if plugin == PluginId::Json {
            if let Some(text) = clipboard_text {
                self.json.open_with_text(text, window, cx);
            }
        } else if plugin == PluginId::Yaml {
            if let Some(text) = clipboard_text {
                self.yaml.open_with_text(text, window, cx);
            }
        } else if plugin == PluginId::Translate {
            if let Some(text) = clipboard_text.filter(|text| !text.trim().is_empty()) {
                self.translate.import_text(text, window, cx);
            }
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for plugin in [
            PluginId::Launcher,
            PluginId::Clipboard,
            PluginId::Json,
            PluginId::Yaml,
            PluginId::Markdown,
            PluginId::Translate,
            PluginId::Settings,
            PluginId::System,
            PluginId::Shortcuts,
        ] {
            let result = match plugin {
                PluginId::Launcher => self.launcher.on_shutdown(),
                PluginId::Clipboard => self.clipboard.on_shutdown(),
                PluginId::Json => self.json.on_shutdown(),
                PluginId::Yaml => self.yaml.on_shutdown(),
                PluginId::Markdown => self.markdown.on_shutdown(),
                PluginId::Translate => self.translate.on_shutdown(),
                PluginId::Settings => self.settings.on_shutdown(),
                PluginId::System => self.system.on_shutdown(),
                PluginId::Shortcuts => self.shortcuts.on_shutdown(),
            };
            self.record_lifecycle_result(plugin, result, PluginLifecycle::Stopped);
        }
    }
}
