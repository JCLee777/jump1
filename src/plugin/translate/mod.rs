//! AI-powered translation plugin and provider preferences.

pub mod view;

use std::collections::BTreeMap;

use gpui_kit::{App, AppContext as _, Entity, FocusHandle, Window};
use serde::{Deserialize, Serialize};

use crate::{
    network::ChatCompletionClient,
    plugin::{
        PluginEventHandler, PluginId,
        api::{Plugin, PluginKind, PluginManifest, PluginPermission},
        storage::PluginStorage,
    },
};

const PREFERENCES_FILE: &str = "translation.json";
pub const MAX_TRANSLATION_BYTES: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationDirection {
    ChineseToEnglish,
    EnglishToChinese,
}

impl TranslationDirection {
    pub fn detect(source: &str) -> Self {
        if source.chars().any(is_han) {
            Self::ChineseToEnglish
        } else {
            Self::EnglishToChinese
        }
    }

    fn target_language(self) -> &'static str {
        match self {
            Self::ChineseToEnglish => "English",
            Self::EnglishToChinese => "Simplified Chinese",
        }
    }
}

fn is_han(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
            | 0x30000..=0x3134F
    )
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    OpenAi,
    DeepSeek,
    Custom,
}

impl Provider {
    pub fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Self::OpenAi => ("https://api.openai.com/v1/chat/completions", "gpt-5-mini"),
            Self::DeepSeek => (
                "https://api.deepseek.com/chat/completions",
                "deepseek-flash",
            ),
            Self::Custom => ("", ""),
        }
    }

    pub fn storage_key(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::DeepSeek => "deepseek",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TranslationPreferences {
    pub provider: Provider,
    pub endpoint: String,
    pub model: String,
    pub api_keys: BTreeMap<String, String>,
}

impl Default for TranslationPreferences {
    fn default() -> Self {
        let provider = Provider::default();
        let (endpoint, model) = provider.defaults();
        Self {
            provider,
            endpoint: endpoint.to_owned(),
            model: model.to_owned(),
            api_keys: BTreeMap::new(),
        }
    }
}

pub fn load_preferences(storage: Option<&PluginStorage>) -> TranslationPreferences {
    let mut preferences = storage
        .and_then(|storage| storage.read(PREFERENCES_FILE))
        .and_then(|bytes| serde_json::from_slice::<TranslationPreferences>(&bytes).ok())
        .unwrap_or_default();
    let (endpoint, model) = preferences.provider.defaults();
    if preferences.endpoint.trim().is_empty() && !endpoint.is_empty() {
        preferences.endpoint = endpoint.to_owned();
    }
    if preferences.model.trim().is_empty() && !model.is_empty() {
        preferences.model = model.to_owned();
    }
    preferences
}

pub fn save_preferences(
    storage: Option<&PluginStorage>,
    preferences: &TranslationPreferences,
) -> bool {
    let Some(storage) = storage else { return false };
    serde_json::to_vec(preferences)
        .is_ok_and(|bytes| storage.write_atomic(PREFERENCES_FILE, &bytes))
}

pub fn test_connection(preferences: &TranslationPreferences, api_key: &str) -> Result<(), String> {
    let client = ChatCompletionClient::new()?;
    client
        .complete(
            &preferences.endpoint,
            &preferences.model,
            api_key,
            "You are checking an API connection. Reply with only: OK",
            "Reply with OK.",
        )
        .map(|_| ())
}

pub fn translate(
    preferences: &TranslationPreferences,
    api_key: &str,
    source: &str,
) -> Result<String, String> {
    if source.len() > MAX_TRANSLATION_BYTES {
        return Err("Input must be 32 KB or less".to_owned());
    }
    let target_language = TranslationDirection::detect(source).target_language();
    let prompt = format!(
        "Translate the text into {}. Preserve its meaning, tone, names, and formatting. Return only the translation, without explanations or quotation marks.",
        target_language
    );
    ChatCompletionClient::new()?.complete(
        &preferences.endpoint,
        &preferences.model,
        api_key,
        &prompt,
        source,
    )
}

pub struct TranslatePlugin {
    page: Entity<view::TranslatePage>,
    settings_page: Entity<view::TranslateSettingsPanel>,
}

impl TranslatePlugin {
    pub fn matches_launcher_query(query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || ["翻译", "translate", "translator", "translation"]
                .iter()
                .any(|keyword| keyword.contains(&query) || query.contains(keyword))
    }

    pub fn load(
        storage: Option<PluginStorage>,
        locale: crate::i18n::Locale,
        on_event: PluginEventHandler,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let preferences = load_preferences(storage.as_ref());
        let page_storage = storage.clone();
        let page =
            cx.new(|cx| view::TranslatePage::new(page_storage, locale, on_event, window, cx));
        let settings_storage = storage.clone();
        let settings_page = cx.new(|cx| {
            view::TranslateSettingsPanel::new(preferences, settings_storage, locale, window, cx)
        });
        Self {
            page,
            settings_page,
        }
    }

    pub fn page(&self) -> &Entity<view::TranslatePage> {
        &self.page
    }

    pub fn settings_page(&self) -> &Entity<view::TranslateSettingsPanel> {
        &self.settings_page
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.page.read(cx).focus_handle(cx)
    }

    pub fn import_text(&self, text: String, window: &mut Window, cx: &mut App) {
        self.page
            .update(cx, |page, cx| page.set_source_text(text, window, cx));
    }

    pub fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: PluginId::Translate,
            name: "Translate",
            version: "0.1.0",
            kind: PluginKind::Page,
            permissions: &[
                PluginPermission::ClipboardRead,
                PluginPermission::ClipboardWrite,
                PluginPermission::Network,
            ],
        }
    }
}

impl Plugin for TranslatePlugin {
    fn manifest(&self) -> PluginManifest {
        TranslatePlugin::manifest(self)
    }
}
