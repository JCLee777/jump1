//! JSON editing, validation, formatting, and plugin-owned draft persistence.

mod view;

use gpui_kit::{App, AppContext as _, Entity, FocusHandle, Window, component::input::TabSize};
use serde::Serialize as _;

use crate::{
    i18n::{I18n, Locale},
    plugin::{
        PluginEventHandler, PluginId,
        api::{Plugin, PluginKind, PluginManifest, PluginPermission},
        settings::SettingDefinition,
        storage::PluginStorage,
    },
};

const DRAFT_FILE: &str = "draft.json";
const COMPARE_DRAFT_FILE: &str = "compare.json";
const SETTINGS_FILE: &str = "settings.conf";
const MAX_DRAFT_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_DRAFT: &str = "{\n  \"name\": \"Jump1\",\n  \"enabled\": true\n}";

pub struct JsonPlugin {
    page: Entity<view::JsonPage>,
    storage: Option<PluginStorage>,
    indent: usize,
}

impl JsonPlugin {
    pub fn matches_launcher_query(query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || [
                "json",
                "json编辑器",
                "json格式化",
                "格式化json",
                "json editor",
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
            view::JsonPage::new(
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

    pub fn page(&self) -> &Entity<view::JsonPage> {
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
        let Some(formatted) = format_json(&text, self.indent) else {
            return;
        };
        self.page
            .update(cx, |page, cx| page.replace_text(formatted, window, cx));
    }

    pub fn setting_definitions(&self, i18n: &I18n) -> Vec<SettingDefinition> {
        vec![SettingDefinition {
            plugin: PluginId::Json,
            key: "indent",
            title: i18n.text(crate::i18n::Message::JsonIndent).into(),
            description: i18n
                .text(crate::i18n::Message::JsonIndentDescription)
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

fn format_json(text: &str, indent: usize) -> Option<String> {
    if text.len() > MAX_DRAFT_BYTES {
        return None;
    }
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let mut output = Vec::new();
    let indent = vec![b' '; indent];
    let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent);
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
    value.serialize(&mut serializer).ok()?;
    String::from_utf8(output).ok()
}

impl Plugin for JsonPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: PluginId::Json,
            name: "JSON Editor",
            version: "0.1.0",
            kind: PluginKind::Page,
            permissions: &[
                PluginPermission::ClipboardRead,
                PluginPermission::ClipboardWrite,
            ],
        }
    }
}
