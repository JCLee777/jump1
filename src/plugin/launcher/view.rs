//! Launcher plugin page. The host owns the route and action callback.

use std::rc::Rc;

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

use super::{CommandAction, LauncherPlugin, ResultKind, SearchResult};

pub type ActivateHandler = Rc<dyn for<'a, 'b> Fn(CommandAction, &'a mut Window, &'b mut App)>;

pub fn render(
    input: &Entity<InputState>,
    scroll: &ScrollHandle,
    launcher: &LauncherPlugin,
    results: Vec<SearchResult>,
    selected: usize,
    status: Option<String>,
    i18n: &I18n,
    theme: &AppTheme,
    on_activate: ActivateHandler,
) -> AnyElement {
    let colors = theme.semantic.colors;
    let typography = &theme.semantic.typography;
    let recent_items: Vec<_> = results
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, result)| result.kind == ResultKind::RecentApplication)
        .collect();
    let recent_cards: Vec<_> = recent_items
        .iter()
        .map(|(index, result)| {
            recent_card(
                *index,
                *index == selected,
                result.clone(),
                i18n,
                theme,
                on_activate.clone(),
            )
            .into_any_element()
        })
        .collect();
    let rows: Vec<_> = results
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, result)| result.kind != ResultKind::RecentApplication)
        .map(|(index, result)| {
            result_row(
                index,
                index == selected,
                result,
                i18n,
                theme,
                on_activate.clone(),
            )
        })
        .collect();

    div()
        .size_full()
        .flex()
        .flex_col()
        .font_family(typography.sans.clone())
        .bg(theme.materials.window)
        .text_color(colors.foreground)
        .child(
            div()
                .px(theme.semantic.spacing.xl)
                .py(theme.semantic.spacing.lg)
                .child(search_input(input, theme)),
        )
        .child(div().h(theme.metrics.divider).bg(colors.border))
        .child(
            result_list(scroll, theme)
                .when(!recent_cards.is_empty(), |this| {
                    this.child(recent_section(
                        recent_cards,
                        i18n.text(Message::RecentApplications),
                        theme,
                    ))
                })
                .when(rows.is_empty() && recent_items.is_empty(), |this| {
                    this.child(empty_state(i18n.text(Message::EmptyResults), theme))
                })
                .children(rows),
        )
        .child(status_bar(
            status.unwrap_or_else(|| i18n.result_summary(results.len(), launcher.app_count())),
            i18n.text(Message::KeyboardHint),
            theme,
        ))
        .into_any_element()
}

fn result_row(
    index: usize,
    selected: bool,
    result: SearchResult,
    i18n: &I18n,
    theme: &AppTheme,
    on_activate: ActivateHandler,
) -> impl IntoElement {
    let icon = result_icon(&result, selected, theme);
    let title = i18n.result_title(&result.title);
    let detail = i18n.result_detail(&result.detail);
    let kind = i18n.result_kind(result.kind);
    let compact = result.kind == ResultKind::Application;
    let action = result.action;

    result_row_shell(index, selected, icon, title, detail, kind, compact, theme)
        .on_click(move |_, window, cx| on_activate(action.clone(), window, cx))
}

fn recent_section(
    cards: Vec<AnyElement>,
    title: &'static str,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(spacing.sm)
        .pb(spacing.sm)
        .child(
            div()
                .px(spacing.sm)
                .text_size(typography.xs.size)
                .line_height(typography.xs.line_height)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.muted_foreground)
                .child(title),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_wrap()
                .gap(spacing.sm)
                .children(cards),
        )
}

fn recent_card(
    index: usize,
    selected: bool,
    result: SearchResult,
    i18n: &I18n,
    theme: &AppTheme,
    on_activate: ActivateHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let hover_background = theme.materials.hover;
    let icon = result_icon(&result, selected, theme);
    let title = i18n.result_title(&result.title);
    let action = result.action;

    div()
        .id(format!("launcher-recent-{index}"))
        .w(theme.metrics.recent_application_tile_width)
        .h(theme.metrics.recent_application_tile_height)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(spacing.xxs)
        .px(spacing.xxs)
        .rounded(theme.semantic.radius.md)
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
        .on_click(move |_, window, cx| on_activate(action.clone(), window, cx))
        .child(icon)
        .child(
            div()
                .w_full()
                .text_center()
                .text_size(typography.xs.size)
                .line_height(typography.xs.line_height)
                .text_ellipsis()
                .child(title),
        )
}

fn result_icon(result: &SearchResult, selected: bool, theme: &AppTheme) -> AnyElement {
    let colors = theme.semantic.colors;
    let icon_color = if selected {
        theme.materials.selection_foreground
    } else {
        colors.primary
    };
    let fallback_glyph = match result.kind {
        ResultKind::Application | ResultKind::RecentApplication => Glyph::Application,
        ResultKind::Path | ResultKind::QuickAction => Glyph::Folder,
        ResultKind::Plugin => match result.title {
            crate::plugin::protocol::ResultTitle::JsonPlugin => Glyph::Json,
            crate::plugin::protocol::ResultTitle::YamlPlugin => Glyph::Yaml,
            crate::plugin::protocol::ResultTitle::TranslatePlugin => Glyph::Translate,
            crate::plugin::protocol::ResultTitle::SettingsPlugin => Glyph::Appearance,
            _ => Glyph::Clipboard,
        },
        ResultKind::Web => Glyph::Web,
    };
    if let Some(path) = result.icon_path.clone() {
        let fallback_size = theme.metrics.result_icon;
        let image_size = theme.metrics.application_icon;
        let fallback_color = icon_color;
        img(path)
            .size(image_size)
            .with_loading(move || {
                glyph(fallback_glyph, fallback_size, fallback_color).into_any_element()
            })
            .with_fallback(move || {
                glyph(fallback_glyph, fallback_size, fallback_color).into_any_element()
            })
            .into_any_element()
    } else {
        glyph(fallback_glyph, theme.metrics.result_icon, icon_color).into_any_element()
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

fn result_list(scroll: &ScrollHandle, theme: &AppTheme) -> Stateful<Div> {
    let spacing = theme.semantic.spacing;
    div()
        .id("results-scroll")
        .track_scroll(scroll)
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .px(spacing.lg)
        .py(spacing.md)
        .flex()
        .flex_col()
        .gap(spacing.xs)
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

fn result_row_shell(
    index: usize,
    selected: bool,
    icon: AnyElement,
    title: String,
    detail: String,
    kind: &'static str,
    compact: bool,
    theme: &AppTheme,
) -> Stateful<Div> {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let radius = theme.semantic.radius;
    let typography = &theme.semantic.typography;
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let selected_control_background = theme.materials.selection_control;
    let hover_background = theme.materials.hover;
    let control_background = theme.materials.control;

    let mut text = div()
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
        );
    if !compact {
        text = text.child(
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
        );
    }

    let mut row = div()
        .id(format!("launcher-result-{index}"))
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
                .child(icon),
        )
        .child(text);
    if !compact {
        row = row.child(
            div()
                .px(spacing.sm)
                .py(spacing.xxs)
                .rounded(radius.md)
                .bg(if selected {
                    selected_control_background
                } else {
                    control_background
                })
                .text_size(typography.xs.size)
                .line_height(typography.xs.line_height)
                .text_color(if selected {
                    selected_foreground
                } else {
                    colors.muted_foreground
                })
                .child(kind),
        );
    }
    row
}
