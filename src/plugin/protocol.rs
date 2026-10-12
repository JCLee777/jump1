//! Shared protocol between plugins and the application host.

use std::path::PathBuf;

use super::PluginId;
use crate::{
    i18n::Locale,
    plugin::system::ThemeChoice,
    theme::{AccentColor, PanelSize},
};

#[derive(Clone, Debug)]
pub struct ApplicationEntry {
    pub name: String,
    pub search_terms: Vec<String>,
    pub path: PathBuf,
    pub icon_path: Option<PathBuf>,
}

#[derive(Clone)]
pub enum CommandAction {
    Application(ApplicationEntry),
    OpenPlugin(PluginId),
    Path(PathBuf),
    Url(String),
}

#[derive(Clone)]
pub struct SearchResult {
    pub title: ResultTitle,
    pub detail: ResultDetail,
    pub kind: ResultKind,
    pub icon_path: Option<PathBuf>,
    pub action: CommandAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultKind {
    Application,
    RecentApplication,
    Path,
    Plugin,
    QuickAction,
    Web,
}

#[derive(Clone, Debug)]
pub enum ResultTitle {
    Application(String),
    ClipboardPlugin,
    JsonPlugin,
    YamlPlugin,
    TranslatePlugin,
    SettingsPlugin,
    OpenHome,
    OpenDownloads,
    OpenDocuments,
    OpenPath(String),
    OpenUrl,
    SearchWeb(String),
}

#[derive(Clone, Debug)]
pub enum ResultDetail {
    ClipboardPlugin,
    JsonPlugin,
    YamlPlugin,
    TranslatePlugin,
    SettingsPlugin,
    Literal(String),
}

pub enum HostEffect {
    ApplyTheme(ThemeChoice),
    ApplyAccentColor(AccentColor),
    ApplyLocale(Locale),
    ResizePanel(PanelSize),
    LaunchAtLogin(Result<bool, String>),
}
