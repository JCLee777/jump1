//! Clipboard plugin page. Navigation stays in the application shell.

use std::{rc::Rc, sync::Arc};

use gpui_kit::{
    component::input::{Input, InputState},
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
    preview_image: Option<Arc<Image>>,
    selected: usize,
    status: Option<String>,
    i18n: &I18n,
    theme: &AppTheme,
    on_back: BackHandler,
    on_activate: ActivateHandler,
    on_sort: SortHandler,
    on_clear: ClearHandler,
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
                    .child(preview_panel(selected, &entry, preview_image, i18n, theme))
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
    image: Option<Arc<Image>>,
    i18n: &I18n,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;

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
                        .child(entry_title(entry)),
                ),
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
    image: Option<Arc<Image>>,
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
                        .child(text[..end].to_owned()),
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
            .map(|image| {
                img(image)
                    .w_full()
                    .h(theme.metrics.clipboard_preview_image_height)
                    .into_any_element()
            })
            .unwrap_or_else(|| {
                glyph(Glyph::Image, theme.metrics.result_icon, colors.primary).into_any_element()
            }),
        ClipboardContent::Files(paths) => {
            let image_path = (paths.len() == 1 && entry.format == ClipboardFormat::Image)
                .then(|| paths[0].clone());
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(spacing.md)
                .when_some(image_path, |this, path| {
                    this.child(
                        img(path)
                            .w_full()
                            .h(theme.metrics.clipboard_preview_image_height),
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
    let title = entry_title(&entry);
    let detail =
        i18n.clipboard_entry_detail(entry.content.size(), entry.captured_at, entry.use_count);
    let icon = entry_icon(&entry);
    let format_label = entry.format.label();

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

fn entry_title(entry: &ClipboardEntry) -> String {
    match &entry.content {
        ClipboardContent::Text(text) => preview(text),
        ClipboardContent::Image { format, .. } => {
            format!("{} image", format.extension().to_uppercase())
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
