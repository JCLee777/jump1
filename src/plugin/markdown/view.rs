use std::time::Duration;

use gpui_kit::{
    App, AppContext as _, Context, Entity, Focusable as _, HighlightStyle, Image, ImageFormat,
    ImageSource, InteractiveElement as _, IntoElement, ParentElement, Render, Resource,
    StatefulInteractiveElement as _, Styled, Subscription, Window,
    base::text::{TextView, TextViewState, TextViewStyle},
    component::input::{Editor, EditorState, InputEvent},
    div, px,
};

use crate::{
    i18n::{I18n, Locale, Message},
    icon::{Glyph, glyph},
    plugin::{
        ExternalOpenGuard, PluginEvent, PluginEventHandler, PluginId,
        compare::{ScrollSync, Side},
        storage::PluginStorage,
    },
    theme::AppTheme,
};

use super::{DRAFT_FILE, MAX_DRAFT_BYTES};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MarkdownViewMode {
    Edit,
    Split,
    Preview,
}

pub struct MarkdownPage {
    editor: Entity<EditorState>,
    preview: Entity<TextViewState>,
    scroll_sync: ScrollSync,
    draft: String,
    summary: String,
    storage: Option<PluginStorage>,
    locale: Locale,
    mode: MarkdownViewMode,
    revision: u64,
    notice: Option<Message>,
    on_event: PluginEventHandler,
    external_open_guard: ExternalOpenGuard,
    _editor_subscription: Subscription,
    _editor_scroll_subscription: Subscription,
    _preview_scroll_subscription: Subscription,
    _key_subscription: Subscription,
}

impl MarkdownPage {
    pub fn new(
        draft: String,
        locale: Locale,
        storage: Option<PluginStorage>,
        on_event: PluginEventHandler,
        external_open_guard: ExternalOpenGuard,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .line_number(true)
                .indent_guides(true)
                .soft_wrap(true)
                .default_value(draft.clone())
        });
        let preview = cx.new(|cx| TextViewState::markdown(&draft, cx));
        let preview_for_editor = preview.clone();
        let editor_scroll_subscription = cx.observe(&editor, move |this, editor, cx| {
            let y = clamp_preview_scroll_y(
                &preview_for_editor.read(cx),
                editor.read(cx).scroll_offset().y,
            );
            if let Some(y) =
                this.scroll_sync
                    .observe(Side::Left, y, this.mode == MarkdownViewMode::Split)
            {
                preview_for_editor.update(cx, |preview, cx| {
                    set_preview_scroll_y(preview, y, cx);
                });
            }
        });
        let editor_for_preview = editor.clone();
        let preview_scroll_subscription = cx.observe(&preview, move |this, preview, cx| {
            let y = preview
                .read(cx)
                .list_state()
                .scroll_px_offset_for_scrollbar()
                .y;
            if let Some(y) =
                this.scroll_sync
                    .observe(Side::Right, y, this.mode == MarkdownViewMode::Split)
            {
                editor_for_preview.update(cx, |editor, cx| {
                    let mut offset = editor.scroll_offset();
                    if offset.y != y {
                        offset.y = y;
                        editor.set_scroll_offset(offset, cx);
                    }
                });
            }
        });
        let editor_subscription = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            this.draft = editor.read(cx).value().to_string();
            this.summary = markdown_summary(&this.draft, &I18n::new(this.locale));
            if this.mode != MarkdownViewMode::Edit {
                let draft = this.draft.clone();
                this.preview
                    .update(cx, |preview, cx| preview.set_text(&draft, cx));
            }
            this.notice = None;
            this.revision = this.revision.saturating_add(1);
            let revision = this.revision;
            let timer = cx.background_executor().clone();
            cx.spawn(async move |this, cx| {
                timer.timer(Duration::from_millis(180)).await;
                let _ = this.update(cx, |this, cx| {
                    if this.revision == revision {
                        this.persist_draft();
                        cx.notify();
                    }
                });
            })
            .detach();
            cx.notify();
        });
        let editor_focus = editor.focus_handle(cx);
        let escape_handler = on_event.clone();
        let key_subscription = cx.intercept_keystrokes(move |event, window, cx| {
            if editor_focus.is_focused(window) && event.keystroke.key == "escape" {
                escape_handler(PluginEvent::Back, window, cx);
                cx.stop_propagation();
            }
        });

        Self {
            editor,
            preview,
            scroll_sync: ScrollSync::default(),
            summary: markdown_summary(&draft, &I18n::new(locale)),
            draft,
            storage,
            locale,
            mode: MarkdownViewMode::Split,
            revision: 0,
            notice: None,
            on_event,
            external_open_guard,
            _editor_subscription: editor_subscription,
            _editor_scroll_subscription: editor_scroll_subscription,
            _preview_scroll_subscription: preview_scroll_subscription,
            _key_subscription: key_subscription,
        }
    }

    pub fn focus_handle(&self, cx: &App) -> gpui_kit::FocusHandle {
        self.editor.focus_handle(cx)
    }

    pub fn set_locale(&mut self, locale: Locale, cx: &mut Context<Self>) {
        self.locale = locale;
        self.summary = markdown_summary(&self.draft, &I18n::new(locale));
        cx.notify();
    }

    pub fn replace_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if text.len() > MAX_DRAFT_BYTES {
            return;
        }
        self.draft = text.clone();
        self.summary = markdown_summary(&self.draft, &I18n::new(self.locale));
        self.notice = None;
        self.revision = self.revision.saturating_add(1);
        self.persist_draft();
        if self.mode != MarkdownViewMode::Edit {
            self.preview
                .update(cx, |preview, cx| preview.set_text(&text, cx));
        }
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        window.focus(&self.editor.focus_handle(cx), cx);
        cx.notify();
    }

    fn set_mode(&mut self, mode: MarkdownViewMode, window: &mut Window, cx: &mut Context<Self>) {
        self.mode = mode;
        if mode != MarkdownViewMode::Edit {
            let draft = self.draft.clone();
            self.preview
                .update(cx, |preview, cx| preview.set_text(&draft, cx));
        }
        if mode == MarkdownViewMode::Split {
            let y = self.editor.read(cx).scroll_offset().y;
            self.scroll_sync.follow(Side::Right);
            let y = clamp_preview_scroll_y(&self.preview.read(cx), y);
            self.preview
                .update(cx, |preview, cx| set_preview_scroll_y(preview, y, cx));
        }
        if mode != MarkdownViewMode::Preview {
            window.focus(&self.editor.focus_handle(cx), cx);
        }
        cx.notify();
    }

    fn copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).value().to_string();
        self.notice = Some(Message::MarkdownCopied);
        (self.on_event)(PluginEvent::WriteClipboard(text), window, cx);
        cx.notify();
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text(String::new(), window, cx);
    }

    fn import_clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let on_event = self.on_event.clone();
        window.defer(cx, move |window, cx| {
            on_event(PluginEvent::ImportClipboard(PluginId::Markdown), window, cx);
        });
    }

    fn persist_draft(&self) {
        if self.draft.len() <= MAX_DRAFT_BYTES {
            if let Some(storage) = &self.storage {
                let _ = storage.write_text_atomic(DRAFT_FILE, &self.draft);
            }
        }
    }
}

impl Drop for MarkdownPage {
    fn drop(&mut self) {
        self.persist_draft();
    }
}

impl Render for MarkdownPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::resolve(cx);
        let i18n = I18n::new(self.locale);
        let colors = theme.semantic.colors;
        let spacing = theme.semantic.spacing;
        let typography = &theme.semantic.typography;
        let page = cx.entity();
        let on_back = self.on_event.clone();

        let mut content = div()
            .flex_1()
            .min_h(px(0.))
            .p(spacing.md)
            .flex()
            .gap(spacing.md);
        if self.mode != MarkdownViewMode::Preview {
            content = content.child(editor_panel(&self.editor, &i18n, &theme));
        }
        if self.mode != MarkdownViewMode::Edit {
            content = content.child(preview_panel(
                self.preview.clone(),
                &i18n,
                &theme,
                self.external_open_guard.clone(),
            ));
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .font_family(typography.sans.clone())
            .bg(theme.materials.window)
            .text_color(colors.foreground)
            .child(
                div()
                    .px(spacing.xl)
                    .pt(theme.metrics.header_top)
                    .pb(spacing.md)
                    .flex()
                    .items_center()
                    .gap(spacing.md)
                    .child(
                        div()
                            .id("markdown-back")
                            .size(theme.metrics.result_icon_box)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme.semantic.radius.md)
                            .cursor_pointer()
                            .hover(move |this| this.bg(colors.muted))
                            .on_click(move |_, window, cx| on_back(PluginEvent::Back, window, cx))
                            .child(glyph(
                                Glyph::ArrowLeft,
                                theme.metrics.result_icon,
                                colors.primary,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(spacing.xxs)
                            .child(
                                div()
                                    .text_size(typography.sm.size)
                                    .line_height(typography.sm.line_height)
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child(i18n.text(Message::MarkdownName)),
                            )
                            .child(
                                div()
                                    .text_size(typography.xs.size)
                                    .line_height(typography.xs.line_height)
                                    .text_color(colors.muted_foreground)
                                    .child(i18n.text(Message::MarkdownDescription)),
                            ),
                    ),
            )
            .child(
                div()
                    .px(spacing.xl)
                    .pb(spacing.md)
                    .flex()
                    .items_center()
                    .gap(spacing.xs)
                    .child(
                        div()
                            .id("markdown-mode-tabs")
                            .flex()
                            .items_center()
                            .gap(spacing.xxs)
                            .p(spacing.xxs)
                            .rounded(theme.semantic.radius.md)
                            .border_1()
                            .border_color(colors.border)
                            .bg(theme.materials.window)
                            .child(mode_button(
                                "markdown-mode-edit",
                                i18n.text(Message::MarkdownEdit),
                                MarkdownViewMode::Edit,
                                self.mode,
                                page.clone(),
                                &theme,
                            ))
                            .child(mode_button(
                                "markdown-mode-split",
                                i18n.text(Message::MarkdownSplit),
                                MarkdownViewMode::Split,
                                self.mode,
                                page.clone(),
                                &theme,
                            ))
                            .child(mode_button(
                                "markdown-mode-preview",
                                i18n.text(Message::MarkdownPreview),
                                MarkdownViewMode::Preview,
                                self.mode,
                                page.clone(),
                                &theme,
                            )),
                    )
                    .child(div().flex_1())
                    .child(tool_button(
                        "markdown-import",
                        Glyph::Clipboard,
                        i18n.text(Message::MarkdownImportClipboard),
                        page.clone(),
                        |page, window, cx| page.import_clipboard(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "markdown-copy",
                        Glyph::Copy,
                        i18n.text(Message::MarkdownCopy),
                        page.clone(),
                        |page, window, cx| page.copy(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "markdown-clear",
                        Glyph::Trash,
                        i18n.text(Message::MarkdownClear),
                        page,
                        |page, window, cx| page.clear(window, cx),
                        &theme,
                    )),
            )
            .child(div().h(theme.metrics.divider).bg(colors.border))
            .child(content)
            .child(markdown_status(
                self.notice
                    .map(|message| i18n.text(message).to_owned())
                    .unwrap_or_else(|| self.summary.clone()),
                i18n.text(Message::MarkdownKeyboardHint),
                &theme,
            ))
    }
}

fn editor_panel(editor: &Entity<EditorState>, i18n: &I18n, theme: &AppTheme) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .gap(spacing.xs)
        .child(
            div()
                .text_size(typography.xs.size)
                .line_height(typography.xs.line_height)
                .text_color(colors.muted_foreground)
                .child(i18n.text(Message::MarkdownSource)),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .w_full()
                .rounded(theme.semantic.radius.lg)
                .overflow_hidden()
                .child(
                    Editor::new(editor)
                        .size_full()
                        .rounded(theme.semantic.radius.lg)
                        .border_1()
                        .border_color(colors.input)
                        .bg(theme.materials.surface),
                ),
        )
}

fn clamp_preview_scroll_y(preview: &TextViewState, y: gpui_kit::Pixels) -> gpui_kit::Pixels {
    y.clamp(-preview.list_state().max_offset_for_scrollbar().y, px(0.))
}

fn set_preview_scroll_y(
    preview: &mut TextViewState,
    y: gpui_kit::Pixels,
    cx: &mut Context<TextViewState>,
) {
    let list = preview.list_state();
    let current_y = list.scroll_px_offset_for_scrollbar().y;
    list.scroll_by(current_y - y);
    cx.notify();
}

fn preview_panel(
    preview: Entity<TextViewState>,
    i18n: &I18n,
    theme: &AppTheme,
    external_open_guard: ExternalOpenGuard,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let text_style = TextViewStyle::default()
        .with_foreground(colors.foreground)
        .with_muted_foreground(colors.muted_foreground)
        .with_link(colors.primary)
        .with_selection(colors.selection)
        .with_code_background(theme.materials.control)
        .with_border(colors.border)
        .with_inline_code(HighlightStyle {
            color: Some(colors.accent_foreground),
            background_color: Some(colors.accent),
            ..Default::default()
        })
        .with_dark(colors.foreground.l >= 0.5);
    div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .gap(spacing.xs)
        .child(
            div()
                .text_size(typography.xs.size)
                .line_height(typography.xs.line_height)
                .text_color(colors.muted_foreground)
                .child(i18n.text(Message::MarkdownRenderedPreview)),
        )
        .child(
            div()
                .id("markdown-preview-scroll")
                .flex_1()
                .min_h(px(0.))
                .w_full()
                .rounded(theme.semantic.radius.lg)
                .border_1()
                .border_color(colors.input)
                .bg(theme.materials.surface)
                .p(spacing.md)
                .overflow_hidden()
                .child(
                    TextView::new(&preview)
                        .image_source(markdown_image_source)
                        .style(text_style)
                        .scrollable(true)
                        .on_link_click(move |url, _, _, cx| {
                            if let Some(path) = markdown_local_path(url.as_ref()) {
                                external_open_guard.mark_opening();
                                if let Err(error) = crate::platform::open_path(&path) {
                                    eprintln!("failed to open Markdown file link: {error}");
                                }
                            } else if let Some(url) = markdown_link_url(url.as_ref()) {
                                external_open_guard.mark_opening();
                                cx.open_url(&url);
                            }
                        })
                        .size_full(),
                ),
        )
}

fn markdown_image_source(uri: &gpui_kit::SharedUri) -> ImageSource {
    let url = uri.as_ref();
    if let Some(path) = markdown_local_path(url) {
        return ImageSource::Resource(Resource::from(path));
    }

    if let Some(image) = markdown_data_url_image(url) {
        return ImageSource::Image(image);
    }

    ImageSource::Resource(Resource::Uri(uri.clone()))
}

fn markdown_data_url_image(url: &str) -> Option<std::sync::Arc<Image>> {
    let data_url = data_url::DataUrl::process(url).ok()?;
    let mime = data_url.mime_type();
    let format = ImageFormat::from_mime_type(&format!("{}/{}", mime.type_, mime.subtype))?;
    let (bytes, _) = data_url.decode_to_vec().ok()?;
    Some(std::sync::Arc::new(Image::from_bytes(format, bytes)))
}

fn markdown_local_path(url: &str) -> Option<std::path::PathBuf> {
    let url = url.trim();
    let (scheme, target) = url.split_once(':').unwrap_or(("", url));
    if scheme.eq_ignore_ascii_case("file") {
        let target = target.strip_prefix("//").unwrap_or(target);
        let path = if target.starts_with('/') {
            target.split(['?', '#']).next()?.to_owned()
        } else {
            let (host, path) = target.split_once('/')?;
            if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                return None;
            }
            format!("/{path}").split(['?', '#']).next()?.to_owned()
        };
        return Some(std::path::PathBuf::from(percent_decode_path(&path)?));
    }

    let path = std::path::PathBuf::from(percent_decode_path(url)?);
    path.is_absolute().then_some(path)
}

fn percent_decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((hex_value(high)? << 4) | hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn markdown_link_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(char::is_whitespace) {
        return None;
    }

    let authority = url.split(['/', '?', '#']).next().unwrap_or_default();
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    if let Some((hostname, port)) = host_port.rsplit_once(':')
        && port.parse::<u16>().is_ok()
        && is_markdown_hostname(hostname)
    {
        return Some(format!("https://{url}"));
    }

    if let Some((scheme, target)) = url.split_once(':') {
        let scheme = scheme.to_ascii_lowercase();
        let valid_scheme = !scheme.is_empty()
            && scheme.chars().enumerate().all(|(index, character)| {
                character.is_ascii_alphanumeric()
                    || (index > 0 && matches!(character, '+' | '.' | '-'))
            })
            && scheme.as_bytes()[0].is_ascii_alphabetic();
        if valid_scheme && !target.is_empty() && !matches!(scheme.as_str(), "javascript" | "data") {
            return Some(url.to_owned());
        }
    }

    if url.starts_with("//") {
        return Some(format!("https:{url}"));
    }

    if is_markdown_hostname(host_port) {
        Some(format!("https://{url}"))
    } else {
        None
    }
}

fn is_markdown_hostname(hostname: &str) -> bool {
    hostname.contains('.')
        || hostname.eq_ignore_ascii_case("localhost")
        || hostname.parse::<std::net::IpAddr>().is_ok()
}

fn mode_button(
    id: &'static str,
    label: &'static str,
    mode: MarkdownViewMode,
    selected_mode: MarkdownViewMode,
    page: Entity<MarkdownPage>,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let typography = &theme.semantic.typography;
    let selected = mode == selected_mode;
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let hover_background = theme.materials.hover;
    let spacing = theme.semantic.spacing;
    div()
        .id(id)
        .h(px(30.))
        .px(spacing.md)
        .flex()
        .items_center()
        .justify_center()
        .rounded(theme.semantic.radius.md)
        .bg(if selected {
            selected_background
        } else {
            theme.materials.window
        })
        .text_color(if selected {
            selected_foreground
        } else {
            colors.foreground
        })
        .cursor_pointer()
        .text_size(typography.xs.size)
        .line_height(typography.xs.line_height)
        .font_weight(if selected {
            gpui_kit::FontWeight::SEMIBOLD
        } else {
            gpui_kit::FontWeight::NORMAL
        })
        .hover(move |this| {
            this.bg(if selected {
                selected_background
            } else {
                hover_background
            })
        })
        .on_click(move |_, window, cx| {
            page.update(cx, |page, cx| page.set_mode(mode, window, cx));
        })
        .child(label)
}

fn tool_button(
    id: &'static str,
    icon: Glyph,
    label: &'static str,
    page: Entity<MarkdownPage>,
    action: impl Fn(&mut MarkdownPage, &mut Window, &mut Context<MarkdownPage>) + 'static,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    div()
        .id(id)
        .h(px(32.))
        .px(spacing.sm)
        .flex()
        .items_center()
        .gap(spacing.xs)
        .rounded(theme.semantic.radius.md)
        .border_1()
        .border_color(colors.border)
        .bg(theme.materials.surface)
        .cursor_pointer()
        .text_size(typography.xs.size)
        .line_height(typography.xs.line_height)
        .hover(move |this| this.bg(colors.muted))
        .on_click(move |_, window, cx| page.update(cx, |page, cx| action(page, window, cx)))
        .child(glyph(icon, px(14.), colors.primary))
        .child(label)
}

fn markdown_status(message: String, hint: &'static str, theme: &AppTheme) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    div()
        .h(theme.metrics.status_bar_height)
        .px(spacing.xl)
        .flex()
        .items_center()
        .justify_between()
        .border_t_1()
        .border_color(colors.border)
        .text_size(typography.xs.size)
        .line_height(typography.xs.line_height)
        .child(div().text_color(colors.muted_foreground).child(message))
        .child(div().text_color(colors.muted_foreground).child(hint))
}

fn markdown_summary(source: &str, i18n: &I18n) -> String {
    let lines = source.lines().count().max(1);
    let characters = source.chars().count();
    match i18n.locale() {
        Locale::ZhCn => format!("{lines} 行 · {characters} 个字符"),
        Locale::EnUs => format!("{lines} lines · {characters} characters"),
    }
}
