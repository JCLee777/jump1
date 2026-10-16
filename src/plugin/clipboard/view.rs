//! Clipboard plugin page. Navigation stays in the application shell.

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use gpui_kit::{
    component::input::{Input, InputState},
    component::{
        Rope,
        highlighter::{LanguageRegistry, SyntaxHighlighter},
    },
    prelude::*,
    *,
};

use crate::{
    i18n::{I18n, Message},
    icon::{Glyph, glyph},
    theme::AppTheme,
};

use super::{ClipboardContent, ClipboardEntry, ClipboardFormat, ClipboardSortMode};

pub type BackHandler = Rc<dyn for<'a, 'b> Fn(&'a mut Window, &'b mut App)>;
pub type ActivateHandler = Rc<dyn for<'a, 'b> Fn(usize, &'a mut Window, &'b mut App)>;
pub type SortHandler = Rc<dyn for<'a, 'b> Fn(ClipboardSortMode, &'a mut Window, &'b mut App)>;
pub type ClearHandler = Rc<dyn for<'a, 'b> Fn(&'a mut Window, &'b mut App)>;
pub type OpenUrlHandler = Rc<dyn for<'a, 'b> Fn(String, &'a mut Window, &'b mut App)>;

thread_local! {
    static CLIPBOARD_HIGHLIGHTERS: RefCell<HashMap<String, SyntaxHighlighter>> = RefCell::new(HashMap::new());
}

const MAX_PREVIEW_CHARACTERS: usize = 16_000;
const ENTRY_TITLE_PREVIEW_CHARACTERS: usize = 96;

#[allow(clippy::too_many_arguments)]
pub fn render(
    input: &Entity<InputState>,
    scroll: &UniformListScrollHandle,
    entries: Vec<&ClipboardEntry>,
    total_entries: usize,
    sort_mode: ClipboardSortMode,
    preview_open: bool,
    preview_image: Option<Arc<RenderImage>>,
    selected: usize,
    status: Option<String>,
    i18n: &I18n,
    theme: &AppTheme,
    on_back: BackHandler,
    on_activate: ActivateHandler,
    on_sort: SortHandler,
    on_clear: ClearHandler,
    on_open_url: OpenUrlHandler,
) -> AnyElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let entries: Vec<_> = entries.into_iter().cloned().collect();
    let visible_entries = entries.len();
    let preview_entry = preview_open
        .then(|| entries.get(selected).cloned())
        .flatten();
    let preview_is_open = preview_entry.is_some();
    let row_entries = Rc::new(entries);
    let rows = if visible_entries == 0 {
        div()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .px(spacing.lg)
            .py(spacing.md)
            .child(empty_state(i18n.text(Message::ClipboardEmpty), theme))
            .into_any_element()
    } else {
        let row_i18n = i18n.clone();
        let row_activate = on_activate.clone();
        let item_height = theme.metrics.result_row_height + spacing.xs;
        uniform_list("clipboard-results", visible_entries, move |range, _, cx| {
            let row_theme = AppTheme::resolve(cx);
            let rows: Vec<_> = range
                .map(|index| {
                    div().w_full().h(item_height).child(clipboard_row(
                        index,
                        index == selected,
                        &row_entries[index],
                        &row_i18n,
                        &row_theme,
                        row_activate.clone(),
                    ))
                })
                .collect();
            rows
        })
        .track_scroll(scroll)
        .flex_1()
        .min_w(px(0.))
        .min_h(px(0.))
        .px(spacing.lg)
        .py(spacing.md)
        .into_any_element()
    };

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
                .pb(theme.metrics.header_bottom)
                .flex()
                .flex_col()
                .gap(spacing.lg)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(spacing.md)
                        .child(
                            div()
                                .id("clipboard-back")
                                .size(theme.metrics.result_icon_box)
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(theme.semantic.radius.md)
                                .cursor_pointer()
                                .hover(move |this| this.bg(theme.materials.hover))
                                .on_click(move |_, window, cx| on_back(window, cx))
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
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(i18n.text(Message::ClipboardName)),
                                )
                                .child(
                                    div()
                                        .text_size(typography.xs.size)
                                        .line_height(typography.xs.line_height)
                                        .text_color(colors.muted_foreground)
                                        .child(i18n.text(Message::ClipboardDescription)),
                                ),
                        )
                        .child(
                            div()
                                .id("clipboard-clear")
                                .px(spacing.md)
                                .py(spacing.sm)
                                .rounded(theme.semantic.radius.md)
                                .cursor_pointer()
                                .text_size(typography.xs.size)
                                .text_color(colors.muted_foreground)
                                .bg(theme.materials.control)
                                .hover(move |this| this.bg(theme.materials.hover))
                                .on_click(move |_, window, cx| on_clear(window, cx))
                                .child(i18n.text(Message::ClipboardClear)),
                        ),
                )
                .child(sort_tabs(sort_mode, i18n, theme, on_sort))
                .child(search_input(input, theme)),
        )
        .child(div().h(theme.metrics.divider).bg(colors.border))
        .child(div().flex_1().min_h(px(0.)).flex().child(rows).when_some(
            preview_entry,
            |this, entry| {
                this.child(div().w(theme.metrics.divider).bg(colors.border))
                    .child(preview_panel(
                        selected,
                        &entry,
                        preview_image,
                        i18n,
                        theme,
                        on_open_url,
                    ))
            },
        ))
        .child(status_bar(
            status.unwrap_or_else(|| i18n.clipboard_summary(visible_entries, total_entries)),
            i18n.text(if preview_is_open {
                Message::ClipboardPreviewKeyboardHint
            } else {
                Message::ClipboardKeyboardHint
            }),
            theme,
        ))
        .into_any_element()
}

fn preview_panel(
    selected: usize,
    entry: &ClipboardEntry,
    image: Option<Arc<RenderImage>>,
    i18n: &I18n,
    theme: &AppTheme,
    on_open_url: OpenUrlHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let url = match &entry.content {
        ClipboardContent::Text(text) => browser_url(text),
        ClipboardContent::Image { .. } | ClipboardContent::Files(_) => None,
    };
    let hover_background = theme.materials.hover;

    div()
        .id("clipboard-preview")
        .flex_1()
        .min_w(px(0.))
        .min_h(px(0.))
        .flex()
        .flex_col()
        .bg(theme.materials.surface)
        .child(
            div()
                .px(spacing.lg)
                .py(spacing.sm)
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(typography.sm.size)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_ellipsis()
                        .child(entry_title(entry, i18n)),
                )
                .when_some(url, |this, url| {
                    let open_url = on_open_url.clone();
                    this.child(
                        div()
                            .id("clipboard-open-url")
                            .flex()
                            .items_center()
                            .gap(spacing.xs)
                            .px(spacing.sm)
                            .py(spacing.xs)
                            .rounded(theme.semantic.radius.sm)
                            .cursor_pointer()
                            .text_size(typography.xs.size)
                            .text_color(colors.primary)
                            .hover(move |this| this.bg(hover_background))
                            .on_click(move |_, window, cx| open_url(url.clone(), window, cx))
                            .child(glyph(Glyph::Web, theme.metrics.result_icon, colors.primary)),
                    )
                }),
        )
        .child(div().h(theme.metrics.divider).bg(colors.border))
        .child(
            div()
                .id(format!(
                    "clipboard-preview-content-{selected}-{}",
                    entry.captured_at
                ))
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .p(spacing.lg)
                .child(preview_content(entry, image, i18n, theme)),
        )
}

fn preview_content(
    entry: &ClipboardEntry,
    image: Option<Arc<RenderImage>>,
    i18n: &I18n,
    theme: &AppTheme,
) -> AnyElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    match &entry.content {
        ClipboardContent::Text(text) => {
            let end = text
                .char_indices()
                .nth(MAX_PREVIEW_CHARACTERS)
                .map_or(text.len(), |(offset, _)| offset);
            let preview_text = &text[..end];
            let highlights = syntax_highlights(preview_text, entry.format, theme);
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(spacing.md)
                .child(
                    div()
                        .font_family(typography.mono.clone())
                        .text_size(typography.sm.size)
                        .line_height(typography.sm.line_height)
                        .child(
                            StyledText::new(preview_text.to_owned()).with_highlights(highlights),
                        ),
                )
                .when(end < text.len(), |this| {
                    this.child(
                        div()
                            .text_size(typography.xs.size)
                            .text_color(colors.muted_foreground)
                            .child(i18n.text(Message::ClipboardPreviewTruncated)),
                    )
                })
                .into_any_element()
        }
        ClipboardContent::Image { .. } => image
            .map(|image| preview_image(image, theme))
            .unwrap_or_else(|| empty_image_slot(theme)),
        ClipboardContent::Files(paths) => {
            let has_image_preview = paths.len() == 1 && entry.format == ClipboardFormat::Image;
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(spacing.md)
                .when(has_image_preview, |this| {
                    this.child(
                        image
                            .map(|image| preview_image(image, theme))
                            .unwrap_or_else(|| empty_image_slot(theme)),
                    )
                })
                .children(paths.iter().take(100).map(|path| {
                    div()
                        .flex()
                        .items_start()
                        .gap(spacing.sm)
                        .child(glyph(
                            Glyph::File,
                            theme.metrics.result_icon,
                            colors.primary,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .text_size(typography.xs.size)
                                .child(path.to_string_lossy().to_string()),
                        )
                }))
                .when(paths.len() > 100, |this| {
                    this.child(
                        div()
                            .text_size(typography.xs.size)
                            .text_color(colors.muted_foreground)
                            .child(i18n.text(Message::ClipboardPreviewMoreFiles)),
                    )
                })
                .into_any_element()
        }
    }
}

fn syntax_highlights(
    text: &str,
    format: ClipboardFormat,
    theme: &AppTheme,
) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
    let Some(language) = preview_language(text, format) else {
        return Vec::new();
    };
    if LanguageRegistry::singleton().language(language).is_none() {
        if language == "sql" {
            return sql_highlights(text, theme);
        }
        return Vec::new();
    }

    CLIPBOARD_HIGHLIGHTERS.with(|highlighters| {
        let mut highlighters = highlighters.borrow_mut();
        let highlighter = highlighters
            .entry(language.to_owned())
            .or_insert_with(|| SyntaxHighlighter::new(language));
        let rope = Rope::from_str(text);
        highlighter.update(None, &rope, None);
        highlighter.styles(&(0..text.len()), theme.highlight_theme.as_ref())
    })
}

fn sql_highlights(text: &str, theme: &AppTheme) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
    let syntax = &theme.highlight_theme.style.syntax;
    let keyword_style = syntax.style("keyword").unwrap_or_default();
    let string_style = syntax.style("string").unwrap_or_default();
    let number_style = syntax.style("number").unwrap_or_default();
    let comment_style = syntax.style("comment").unwrap_or_default();
    let function_style = syntax.style("function").unwrap_or_default();
    let boolean_style = syntax.style("boolean").unwrap_or(keyword_style);
    let bytes = text.as_bytes();
    let mut highlights = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let start = index;
        let style = if bytes[index] == b'-' && bytes.get(index + 1) == Some(&b'-') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            Some(comment_style)
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            Some(comment_style)
        } else if bytes[index] == b'\'' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\'' {
                    index += 1;
                    if bytes.get(index) == Some(&b'\'') {
                        index += 1;
                        continue;
                    }
                    break;
                }
                index += 1;
            }
            Some(string_style)
        } else if bytes[index].is_ascii_digit() {
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit() || matches!(bytes[index], b'.' | b'_'))
            {
                index += 1;
            }
            Some(number_style)
        } else if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let token = text[start..index].to_ascii_lowercase();
            if matches!(token.as_str(), "true" | "false") {
                Some(boolean_style)
            } else if matches!(
                token.as_str(),
                "all"
                    | "alter"
                    | "and"
                    | "as"
                    | "asc"
                    | "between"
                    | "by"
                    | "case"
                    | "create"
                    | "cross"
                    | "delete"
                    | "desc"
                    | "distinct"
                    | "drop"
                    | "else"
                    | "end"
                    | "except"
                    | "exists"
                    | "foreign"
                    | "from"
                    | "full"
                    | "group"
                    | "having"
                    | "in"
                    | "index"
                    | "inner"
                    | "insert"
                    | "intersect"
                    | "into"
                    | "is"
                    | "join"
                    | "key"
                    | "left"
                    | "like"
                    | "limit"
                    | "not"
                    | "null"
                    | "offset"
                    | "on"
                    | "or"
                    | "order"
                    | "outer"
                    | "primary"
                    | "references"
                    | "right"
                    | "select"
                    | "set"
                    | "table"
                    | "then"
                    | "union"
                    | "unique"
                    | "update"
                    | "values"
                    | "when"
                    | "where"
            ) {
                Some(keyword_style)
            } else if bytes.get(index) == Some(&b'(') {
                Some(function_style)
            } else {
                None
            }
        } else {
            index += 1;
            None
        };

        if let Some(style) = style {
            highlights.push((start..index, style));
        }
    }

    highlights
}

fn preview_language(text: &str, format: ClipboardFormat) -> Option<&'static str> {
    let known_language = match format {
        ClipboardFormat::Json => Some("json"),
        ClipboardFormat::Yaml => Some("yaml"),
        ClipboardFormat::Bash => Some("bash"),
        ClipboardFormat::Sql => Some("sql"),
        ClipboardFormat::Html | ClipboardFormat::Xml => Some("html"),
        ClipboardFormat::Markdown => Some("markdown"),
        ClipboardFormat::Code | ClipboardFormat::PlainText => None,
        ClipboardFormat::Http
        | ClipboardFormat::Email
        | ClipboardFormat::Jwt
        | ClipboardFormat::Pdf
        | ClipboardFormat::Word
        | ClipboardFormat::Spreadsheet
        | ClipboardFormat::Presentation
        | ClipboardFormat::Image
        | ClipboardFormat::Audio
        | ClipboardFormat::Video
        | ClipboardFormat::Archive
        | ClipboardFormat::Folder
        | ClipboardFormat::File => return None,
    };
    known_language.or_else(|| infer_code_language(text))
}

fn infer_code_language(text: &str) -> Option<&'static str> {
    let trimmed = text.trim_start();
    if trimmed.starts_with("#!") || trimmed.starts_with("set -") || trimmed.starts_with("export ") {
        Some("bash")
    } else if trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("impl ")
    {
        Some("rust")
    } else if trimmed.starts_with("def ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("from ")
        || trimmed.starts_with("import ")
            && !trimmed.starts_with("import {")
            && !trimmed.contains(" from ")
        || trimmed.starts_with("print(")
    {
        Some("python")
    } else if trimmed.starts_with("package ") || trimmed.starts_with("func ") {
        Some("go")
    } else if trimmed.starts_with("#include ") {
        Some("c")
    } else if trimmed.starts_with("interface ")
        || trimmed.starts_with("type ") && trimmed.contains(" = ")
    {
        Some("typescript")
    } else if trimmed.starts_with("const ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("import {")
        || trimmed.contains(" from ") && trimmed.starts_with("import ")
        || trimmed.starts_with("console.")
        || trimmed.contains(" => ")
    {
        Some("javascript")
    } else if trimmed.starts_with("[package]") || trimmed.starts_with("[dependencies]") {
        Some("toml")
    } else if trimmed.starts_with("@media ")
        || trimmed.starts_with("@import ")
        || trimmed.starts_with(":root")
        || trimmed.starts_with(".") && trimmed.contains('{')
        || trimmed.starts_with('#') && trimmed.contains('{')
    {
        Some("css")
    } else if trimmed.starts_with('<') && trimmed.contains('>') {
        Some("html")
    } else {
        None
    }
}

fn browser_url(text: &str) -> Option<String> {
    let value = text.trim();
    let normalized = if value.starts_with("www.") {
        format!("https://{value}")
    } else {
        value.to_owned()
    };
    let url = reqwest::Url::parse(&normalized).ok()?;
    matches!(url.scheme(), "http" | "https").then_some(normalized)
}

fn preview_image(image: Arc<RenderImage>, theme: &AppTheme) -> AnyElement {
    img(image)
        .w_full()
        .h(theme.metrics.clipboard_preview_image_height)
        .into_any_element()
}

fn empty_image_slot(theme: &AppTheme) -> AnyElement {
    div()
        .w_full()
        .h(theme.metrics.clipboard_preview_image_height)
        .into_any_element()
}

fn clipboard_row(
    index: usize,
    selected: bool,
    entry: &ClipboardEntry,
    i18n: &I18n,
    theme: &AppTheme,
    on_activate: ActivateHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let radius = theme.semantic.radius;
    let typography = &theme.semantic.typography;
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let selected_control_background = theme.materials.selection_control;
    let hover_background = theme.materials.hover;
    let control_background = theme.materials.control;
    let title = entry_title(&entry, i18n);
    let detail =
        i18n.clipboard_entry_detail(entry.content.size(), entry.captured_at, entry.use_count);
    let icon = entry_icon(&entry);
    let format_label = i18n.clipboard_format_label(entry.format);

    div()
        .id(format!("clipboard-{index}"))
        .w_full()
        .h(theme.metrics.result_row_height)
        .flex()
        .items_center()
        .gap(spacing.md)
        .rounded(radius.md)
        .px(spacing.md)
        .py(spacing.sm)
        .cursor_pointer()
        .bg(if selected {
            selected_background
        } else {
            theme.materials.surface
        })
        .text_color(if selected {
            selected_foreground
        } else {
            colors.foreground
        })
        .hover(move |this| {
            this.bg(if selected {
                selected_background
            } else {
                hover_background
            })
        })
        .on_click(move |_, window, cx| on_activate(index, window, cx))
        .child(
            div()
                .size(theme.metrics.result_icon_box)
                .flex()
                .items_center()
                .justify_center()
                .rounded(radius.md)
                .bg(if selected {
                    selected_control_background
                } else {
                    control_background
                })
                .child(glyph(
                    icon,
                    theme.metrics.result_icon,
                    if selected {
                        selected_foreground
                    } else {
                        colors.primary
                    },
                )),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .gap(spacing.xxs)
                .child(
                    div()
                        .text_size(typography.sm.size)
                        .line_height(typography.sm.line_height)
                        .font_weight(FontWeight::MEDIUM)
                        .text_ellipsis()
                        .child(title),
                )
                .child(
                    div()
                        .text_size(typography.xs.size)
                        .line_height(typography.xs.line_height)
                        .text_color(if selected {
                            selected_foreground
                        } else {
                            colors.muted_foreground
                        })
                        .text_ellipsis()
                        .child(detail),
                ),
        )
        .child(
            div()
                .px(spacing.sm)
                .py(spacing.xxs)
                .rounded(radius.sm)
                .bg(if selected {
                    selected_control_background
                } else {
                    control_background
                })
                .text_size(typography.xs.size)
                .text_color(if selected {
                    selected_foreground
                } else {
                    colors.muted_foreground
                })
                .child(format_label),
        )
}

fn sort_tabs(
    selected: ClipboardSortMode,
    i18n: &I18n,
    theme: &AppTheme,
    on_sort: SortHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let selected_background = theme.materials.selection;
    let hover_background = theme.materials.hover;
    div().flex().gap(spacing.xs).children(
        [
            (
                ClipboardSortMode::Recent,
                i18n.text(Message::ClipboardSortRecent),
            ),
            (
                ClipboardSortMode::Frequent,
                i18n.text(Message::ClipboardSortFrequent),
            ),
        ]
        .into_iter()
        .map(move |(mode, label)| {
            let handler = on_sort.clone();
            div()
                .id(format!("clipboard-sort-{mode:?}"))
                .px(spacing.md)
                .py(spacing.sm)
                .rounded(theme.semantic.radius.md)
                .cursor_pointer()
                .bg(if mode == selected {
                    selected_background
                } else {
                    transparent_black()
                })
                .text_color(if mode == selected {
                    theme.materials.selection_foreground
                } else {
                    colors.muted_foreground
                })
                .hover(move |this| {
                    this.bg(if mode == selected {
                        selected_background
                    } else {
                        hover_background
                    })
                })
                .on_click(move |_, window, cx| handler(mode, window, cx))
                .child(label)
        }),
    )
}

fn entry_icon(entry: &ClipboardEntry) -> Glyph {
    match entry.format {
        ClipboardFormat::Image => Glyph::Image,
        ClipboardFormat::Audio => Glyph::Audio,
        ClipboardFormat::Folder => Glyph::Folder,
        ClipboardFormat::Json => Glyph::Json,
        ClipboardFormat::Yaml => Glyph::Yaml,
        ClipboardFormat::Http => Glyph::Web,
        ClipboardFormat::PlainText
        | ClipboardFormat::Bash
        | ClipboardFormat::Sql
        | ClipboardFormat::Html
        | ClipboardFormat::Xml
        | ClipboardFormat::Markdown
        | ClipboardFormat::Email
        | ClipboardFormat::Jwt => Glyph::Clipboard,
        _ => Glyph::File,
    }
}

fn entry_title(entry: &ClipboardEntry, i18n: &I18n) -> String {
    match &entry.content {
        ClipboardContent::Text(text) => preview(text),
        ClipboardContent::Image { format, .. } => {
            format!(
                "{} {}",
                format.extension().to_uppercase(),
                i18n.clipboard_image_suffix()
            )
        }
        ClipboardContent::Files(paths) => {
            let names = paths
                .iter()
                .take(3)
                .map(|path| {
                    path.file_name()
                        .unwrap_or(path.as_os_str())
                        .to_string_lossy()
                })
                .collect::<Vec<_>>()
                .join(", ");
            if paths.len() > 3 {
                format!("{names} +{}", paths.len() - 3)
            } else {
                names
            }
        }
    }
}

fn search_input(input: &Entity<InputState>, theme: &AppTheme) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    div()
        .h(theme.metrics.search_height)
        .flex()
        .items_center()
        .gap(spacing.md)
        .rounded(theme.semantic.radius.lg)
        .border_1()
        .border_color(colors.input)
        .bg(theme.materials.surface)
        .px(spacing.md)
        .child(glyph(
            Glyph::Search,
            theme.metrics.search_icon,
            colors.muted_foreground,
        ))
        .child(
            div()
                .flex_1()
                .child(Input::new(input).w_full().bordered(false)),
        )
}

fn empty_state(message: &'static str, theme: &AppTheme) -> impl IntoElement {
    let typography = &theme.semantic.typography;
    div()
        .p(theme.semantic.spacing.xl)
        .text_size(typography.sm.size)
        .line_height(typography.sm.line_height)
        .text_color(theme.semantic.colors.muted_foreground)
        .child(message)
}

fn status_bar(status: String, keyboard_hint: &'static str, theme: &AppTheme) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let typography = &theme.semantic.typography;
    div()
        .h(theme.metrics.status_bar_height)
        .px(theme.semantic.spacing.xl)
        .flex()
        .items_center()
        .justify_between()
        .border_t_1()
        .border_color(colors.border)
        .text_size(typography.xs.size)
        .line_height(typography.xs.line_height)
        .text_color(colors.muted_foreground)
        .child(status)
        .child(
            div()
                .px(theme.semantic.spacing.sm)
                .py(theme.semantic.spacing.xxs)
                .rounded(theme.semantic.radius.md)
                .bg(theme.materials.control)
                .child(keyboard_hint),
        )
}

fn preview(text: &str) -> String {
    let mut preview = String::with_capacity(ENTRY_TITLE_PREVIEW_CHARACTERS * 4 + 3);
    let mut character_count = 0;
    let mut has_previous_word = false;

    for word in text.split_whitespace() {
        if has_previous_word {
            if character_count == ENTRY_TITLE_PREVIEW_CHARACTERS {
                preview.push('…');
                return preview;
            }
            preview.push(' ');
            character_count += 1;
        }

        let mut characters = word.chars();
        for character in characters
            .by_ref()
            .take(ENTRY_TITLE_PREVIEW_CHARACTERS - character_count)
        {
            preview.push(character);
            character_count += 1;
        }
        if characters.next().is_some() {
            preview.push('…');
            return preview;
        }
        has_previous_word = true;
    }

    preview
}
