//! YAML editing, validation, formatting, and plugin-owned draft persistence.

mod view;

use gpui_kit::{App, AppContext as _, Entity, FocusHandle, Window, component::input::TabSize};

use crate::{
    i18n::{I18n, Locale},
    plugin::{
        PluginEventHandler, PluginId,
        api::{Plugin, PluginKind, PluginManifest, PluginPermission},
        settings::SettingDefinition,
        storage::PluginStorage,
    },
};

const DRAFT_FILE: &str = "draft.yaml";
const COMPARE_DRAFT_FILE: &str = "compare.yaml";
const SETTINGS_FILE: &str = "settings.conf";
const MAX_DRAFT_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_DRAFT: &str = "app:\n  name: Jump1\n  enabled: true\n";

pub struct YamlPlugin {
    page: Entity<view::YamlPage>,
    storage: Option<PluginStorage>,
    indent: usize,
}

impl YamlPlugin {
    pub fn matches_launcher_query(query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || [
                "yaml",
                "yml",
                "yaml编辑器",
                "yaml格式化",
                "格式化yaml",
                "yaml editor",
            ]
            .iter()
            .any(|keyword| keyword.contains(&query) || query.contains(keyword))
    }

    pub fn load(
        storage: Option<PluginStorage>,
        locale: Locale,
        on_event: PluginEventHandler,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let indent = storage
            .as_ref()
            .and_then(|storage| storage.read_text(SETTINGS_FILE))
            .and_then(|value| value.trim().parse().ok())
            .filter(|value| matches!(value, 2 | 4))
            .unwrap_or(2);
        let draft = storage
            .as_ref()
            .and_then(|storage| storage.read_text(DRAFT_FILE))
            .filter(|value| value.len() <= MAX_DRAFT_BYTES)
            .unwrap_or_else(|| DEFAULT_DRAFT.to_owned());
        let compare_draft = storage
            .as_ref()
            .and_then(|storage| storage.read_text(COMPARE_DRAFT_FILE))
            .filter(|value| value.len() <= MAX_DRAFT_BYTES)
            .unwrap_or_default();
        let page_storage = storage.clone();
        let page = cx.new(|cx| {
            view::YamlPage::new(
                draft,
                compare_draft,
                indent,
                locale,
                page_storage,
                on_event,
                window,
                cx,
            )
        });
        Self {
            page,
            storage,
            indent,
        }
    }

    pub fn page(&self) -> &Entity<view::YamlPage> {
        &self.page
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.page.read(cx).focus_handle(cx)
    }

    pub fn set_locale(&self, locale: Locale, cx: &mut App) {
        self.page.update(cx, |page, cx| page.set_locale(locale, cx));
    }

    pub fn import_text(&self, text: String, window: &mut Window, cx: &mut App) {
        if text.len() <= MAX_DRAFT_BYTES {
            self.page
                .update(cx, |page, cx| page.replace_text(text, window, cx));
        }
    }

    pub fn open_with_text(&self, text: String, window: &mut Window, cx: &mut App) {
        let Some(formatted) = format_yaml(&text) else {
            return;
        };
        self.page
            .update(cx, |page, cx| page.replace_text(formatted, window, cx));
    }

    pub fn setting_definitions(&self, i18n: &I18n) -> Vec<SettingDefinition> {
        vec![SettingDefinition {
            plugin: PluginId::Yaml,
            key: "indent",
            title: i18n.text(crate::i18n::Message::YamlIndent).into(),
            description: i18n
                .text(crate::i18n::Message::YamlIndentDescription)
                .into(),
            options: [2, 4]
                .into_iter()
                .map(|value| (value.to_string(), value.to_string()))
                .collect(),
            selected: self.indent.to_string(),
        }]
    }

    pub fn apply_setting(
        &mut self,
        key: &str,
        value: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if key != "indent" {
            return false;
        }
        let Ok(indent) = value.parse::<usize>() else {
            return false;
        };
        if !matches!(indent, 2 | 4) {
            return false;
        }
        self.indent = indent;
        if let Some(storage) = &self.storage {
            let _ = storage.write_text_atomic(SETTINGS_FILE, &indent.to_string());
        }
        self.page.update(cx, |page, cx| {
            page.set_indent(
                TabSize {
                    tab_size: indent,
                    hard_tabs: false,
                },
                window,
                cx,
            )
        });
        true
    }
}

fn format_yaml(text: &str) -> Option<String> {
    if text.len() > MAX_DRAFT_BYTES {
        return None;
    }
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).ok()?;
    if !matches!(
        value,
        serde_yaml_ng::Value::Mapping(_)
            | serde_yaml_ng::Value::Sequence(_)
            | serde_yaml_ng::Value::Tagged(_)
    ) {
        return None;
    }
    serde_yaml_ng::to_string(&value).ok()
}

impl Plugin for YamlPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: PluginId::Yaml,
            name: "YAML Editor",
            version: "0.1.0",
            kind: PluginKind::Page,
            permissions: &[
                PluginPermission::ClipboardRead,
                PluginPermission::ClipboardWrite,
            ],
        }
    }
}
