//! Markdown editing, preview, and plugin-owned draft persistence.

mod view;

use gpui_kit::{App, AppContext as _, Entity, FocusHandle, Window};

use crate::i18n::Locale;
use crate::plugin::{
    ExternalOpenGuard, PluginEventHandler, PluginId,
    api::{Plugin, PluginKind, PluginManifest, PluginPermission},
    storage::PluginStorage,
};

const DRAFT_FILE: &str = "draft.md";
const MAX_DRAFT_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_DRAFT: &str = "# Markdown\n\nEdit Markdown on the left and preview it on the right.\n\n- Supports headings, lists, links, tables, and code blocks\n- Drafts are saved locally\n\n```rust\nfn main() {\n    println!(\"Hello, Jump1!\");\n}\n```\n";

pub struct MarkdownPlugin {
    page: Entity<view::MarkdownPage>,
}

impl MarkdownPlugin {
    pub fn matches_launcher_query(query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || [
                "markdown",
                "md",
                "markdown编辑器",
                "markdown预览",
                "markdown editor",
            ]
            .iter()
            .any(|keyword| keyword.contains(&query) || query.contains(keyword))
    }

    pub fn load(
        storage: Option<PluginStorage>,
        locale: Locale,
        on_event: PluginEventHandler,
        external_open_guard: ExternalOpenGuard,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let draft = storage
            .as_ref()
            .and_then(|storage| storage.read_text(DRAFT_FILE))
            .filter(|value| value.len() <= MAX_DRAFT_BYTES)
            .unwrap_or_else(|| DEFAULT_DRAFT.to_owned());
        let page = cx.new(|cx| {
            view::MarkdownPage::new(
                draft,
                locale,
                storage,
                on_event,
                external_open_guard,
                window,
                cx,
            )
        });
        Self { page }
    }

    pub fn page(&self) -> &Entity<view::MarkdownPage> {
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
}

impl Plugin for MarkdownPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: PluginId::Markdown,
            name: "Markdown",
            version: "0.1.0",
            kind: PluginKind::Page,
            permissions: &[
                PluginPermission::ClipboardRead,
                PluginPermission::ClipboardWrite,
                PluginPermission::Filesystem,
            ],
        }
    }
}
