use std::{collections::BTreeMap, time::Duration};

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, AppContext as _, Context, Entity, Focusable as _, HighlightStyle, InteractiveElement as _,
    IntoElement, ParentElement, Render, StatefulInteractiveElement as _, Styled, Subscription,
    Window,
    component::input::{
        Editor, EditorState, InputEvent, TabSize, TextDecoration, TextDecorationCollection,
    },
    div, px,
};
use serde::Serialize as _;
use serde_json::{Map, Value};

use crate::{
    i18n::{I18n, Locale, Message},
    icon::{Glyph, glyph},
    plugin::{
        PluginEvent, PluginEventHandler, PluginId,
        compare::{Comparison, ScrollSync, Side, changed_ranges},
        storage::PluginStorage,
    },
    theme::AppTheme,
};

use super::{COMPARE_DRAFT_FILE, DRAFT_FILE, MAX_DRAFT_BYTES};

#[derive(Clone)]
enum Validation {
    Empty,
    Valid {
        lines: usize,
        bytes: usize,
    },
    Invalid {
        line: usize,
        column: usize,
        message: String,
    },
}

pub struct JsonPage {
    editor: Entity<EditorState>,
    compare_editor: Entity<EditorState>,
    left_marks: TextDecorationCollection,
    right_marks: TextDecorationCollection,
    scroll_sync: ScrollSync,
    compare_mode: bool,
    comparison: Comparison,
    validation: Validation,
    notice: Option<Message>,
    locale: Locale,
    indent: usize,
    revision: u64,
    compare_revision: u64,
    draft: String,
    compare_draft: String,
    storage: Option<PluginStorage>,
    on_event: PluginEventHandler,
    _editor_subscription: Subscription,
    _compare_subscription: Subscription,
    _left_scroll_subscription: Subscription,
    _right_scroll_subscription: Subscription,
    _key_subscription: Subscription,
}

impl JsonPage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        draft: String,
        compare_draft: String,
        indent: usize,
        locale: Locale,
        storage: Option<PluginStorage>,
        on_event: PluginEventHandler,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .line_number(true)
                .indent_guides(true)
                .soft_wrap(false)
                .tab_size(TabSize {
                    tab_size: indent,
                    hard_tabs: false,
                })
                .default_value(draft.clone())
        });
        let compare_editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .line_number(true)
                .indent_guides(true)
                .soft_wrap(false)
                .tab_size(TabSize {
                    tab_size: indent,
                    hard_tabs: false,
                })
                .default_value(compare_draft.clone())
        });
        let left_marks = editor.update(cx, |editor, cx| {
            editor.create_decorations_collection(Vec::new(), cx)
        });
        let right_marks = compare_editor.update(cx, |editor, cx| {
            editor.create_decorations_collection(Vec::new(), cx)
        });
        let left_scroll_subscription = cx.observe(&editor, |this, editor, cx| {
            let y = editor.read(cx).scroll_offset().y;
            if let Some(y) = this.scroll_sync.observe(Side::Left, y, this.compare_mode) {
                let mut offset = this.compare_editor.read(cx).scroll_offset();
                if offset.y != y {
                    offset.y = y;
                    this.compare_editor
                        .update(cx, |editor, cx| editor.set_scroll_offset(offset, cx));
                }
            }
        });
        let right_scroll_subscription = cx.observe(&compare_editor, |this, editor, cx| {
            let y = editor.read(cx).scroll_offset().y;
            if let Some(y) = this.scroll_sync.observe(Side::Right, y, this.compare_mode) {
                let mut offset = this.editor.read(cx).scroll_offset();
                if offset.y != y {
                    offset.y = y;
                    this.editor
                        .update(cx, |editor, cx| editor.set_scroll_offset(offset, cx));
                }
            }
        });
        let editor_subscription = cx.subscribe(&editor, |this, editor, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let text = editor.read(cx).value().to_string();
            this.notice = None;
            this.revision = this.revision.saturating_add(1);
            this.draft = text.clone();
            let revision = this.revision;
            let timer = cx.background_executor().clone();
            cx.spawn(async move |this, cx| {
                timer.timer(Duration::from_millis(180)).await;
                let _ = this.update(cx, |this, cx| {
                    if this.revision == revision {
                        this.validation = validate(&this.draft);
                        this.comparison = compare_texts(&this.draft, &this.compare_draft);
                        this.persist_draft(&this.draft);
                        this.refresh_diff(cx);
                        cx.notify();
                    }
                });
            })
            .detach();
            cx.notify();
        });
        let compare_subscription =
            cx.subscribe(&compare_editor, |this, editor, event: &InputEvent, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                this.compare_draft = editor.read(cx).value().to_string();
                this.compare_revision = this.compare_revision.saturating_add(1);
                let revision = this.compare_revision;
                let timer = cx.background_executor().clone();
                cx.spawn(async move |this, cx| {
                    timer.timer(Duration::from_millis(180)).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.compare_revision == revision {
                            this.comparison = compare_texts(&this.draft, &this.compare_draft);
                            this.persist_compare_draft();
                            this.refresh_diff(cx);
                            cx.notify();
                        }
                    });
                })
                .detach();
            });
        let editor_focus = editor.focus_handle(cx);
        let compare_focus = compare_editor.focus_handle(cx);
        let escape_handler = on_event.clone();
        let key_subscription = cx.intercept_keystrokes(move |event, window, cx| {
            if (editor_focus.is_focused(window) || compare_focus.is_focused(window))
                && event.keystroke.key == "escape"
            {
                escape_handler(PluginEvent::Back, window, cx);
                cx.stop_propagation();
            }
        });
        Self {
            editor,
            compare_editor,
            left_marks,
            right_marks,
            scroll_sync: ScrollSync::default(),
            compare_mode: false,
            comparison: compare_texts(&draft, &compare_draft),
            validation: validate(&draft),
            notice: None,
            locale,
            indent,
            revision: 0,
            compare_revision: 0,
            draft,
            compare_draft,
            storage,
            on_event,
            _editor_subscription: editor_subscription,
            _compare_subscription: compare_subscription,
            _left_scroll_subscription: left_scroll_subscription,
            _right_scroll_subscription: right_scroll_subscription,
            _key_subscription: key_subscription,
        }
    }

    pub fn focus_handle(&self, cx: &App) -> gpui_kit::FocusHandle {
        self.editor.focus_handle(cx)
    }

    pub fn set_locale(&mut self, locale: Locale, cx: &mut Context<Self>) {
        self.locale = locale;
        cx.notify();
    }

    pub fn set_indent(&mut self, indent: TabSize, _window: &mut Window, cx: &mut Context<Self>) {
        self.indent = indent.tab_size;
        self.editor
            .update(cx, |editor, cx| editor.set_tab_size(indent, cx));
        self.compare_editor
            .update(cx, |editor, cx| editor.set_tab_size(indent, cx));
    }

    pub fn replace_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.validation = validate(&text);
        self.comparison = compare_texts(&text, &self.compare_draft);
        self.notice = None;
        self.revision = self.revision.saturating_add(1);
        self.draft = text.clone();
        self.persist_draft(&text);
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.refresh_diff(cx);
        window.focus(&self.editor.focus_handle(cx), cx);
        cx.notify();
    }

    fn format(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(value) = self.parsed_value(cx) else {
            return;
        };
        let mut output = Vec::new();
        let indent = vec![b' '; self.indent];
        let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent);
        let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
        if value.serialize(&mut serializer).is_ok() {
            if let Ok(text) = String::from_utf8(output) {
                self.replace_text(text, window, cx);
            }
        }
    }

    fn minify(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(value) = self.parsed_value(cx) else {
            return;
        };
        if let Ok(text) = serde_json::to_string(&value) {
            self.replace_text(text, window, cx);
        }
    }

    fn sort_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(value) = self.parsed_value(cx) else {
            return;
        };
        let sorted = sorted_value(value);
        let mut output = Vec::new();
        let indent = vec![b' '; self.indent];
        let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent);
        let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
        if sorted.serialize(&mut serializer).is_ok() {
            if let Ok(text) = String::from_utf8(output) {
                self.replace_text(text, window, cx);
            }
        }
    }

    fn copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).value().to_string();
        self.notice = Some(Message::ClipboardCopied);
        (self.on_event)(PluginEvent::WriteClipboard(text), window, cx);
        cx.notify();
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text(String::new(), window, cx);
    }

    fn toggle_compare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.compare_mode = !self.compare_mode;
        if self.compare_mode {
            let offset = self.editor.read(cx).scroll_offset();
            self.scroll_sync.follow(Side::Right);
            self.compare_editor
                .update(cx, |editor, cx| editor.set_scroll_offset(offset, cx));
            self.refresh_diff(cx);
        } else {
            self.left_marks.clear(cx);
            self.right_marks.clear(cx);
        }
        window.focus(&self.editor.focus_handle(cx), cx);
        cx.notify();
    }

    fn refresh_diff(&mut self, cx: &mut Context<Self>) {
        if !self.compare_mode || self.compare_draft.trim().is_empty() {
            self.left_marks.clear(cx);
            self.right_marks.clear(cx);
            return;
        }
        let left = self.draft.clone();
        let right = self.compare_draft.clone();
        let revisions = (self.revision, self.compare_revision);
        let diff = cx
            .background_executor()
            .spawn(async move { changed_ranges(&left, &right) });
        cx.spawn(async move |this, cx| {
            let ranges = diff.await;
            let _ = this.update(cx, |this, cx| {
                if !this.compare_mode || (this.revision, this.compare_revision) != revisions {
                    return;
                }
                let style = HighlightStyle {
                    background_color: Some(
                        AppTheme::resolve(cx)
                            .semantic
                            .colors
                            .destructive
                            .opacity(0.2),
                    ),
                    ..Default::default()
                };
                this.left_marks.set(
                    ranges
                        .left
                        .into_iter()
                        .map(|range| TextDecoration::new(range, style))
                        .collect(),
                    cx,
                );
                this.right_marks.set(
                    ranges
                        .right
                        .into_iter()
                        .map(|range| TextDecoration::new(range, style))
                        .collect(),
                    cx,
                );
            });
        })
        .detach();
    }

    fn import_clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let on_event = self.on_event.clone();
        window.defer(cx, move |window, cx| {
            on_event(PluginEvent::ImportClipboard(PluginId::Json), window, cx);
        });
    }

    fn parsed_value(&mut self, cx: &mut Context<Self>) -> Option<Value> {
        let text = self.editor.read(cx).value().to_string();
        match serde_json::from_str(&text) {
            Ok(value) => Some(value),
            Err(error) => {
                let offset = error_offset(&text, error.line(), error.column());
                self.validation = Validation::Invalid {
                    line: error.line(),
                    column: error.column(),
                    message: json_error_message(&error),
                };
                self.editor.update(cx, |editor, cx| {
                    editor.set_selected_range(offset..offset, cx);
                });
                self.notice = None;
                cx.notify();
                None
            }
        }
    }

    fn persist_draft(&self, text: &str) {
        if text.len() <= MAX_DRAFT_BYTES {
            if let Some(storage) = &self.storage {
                let _ = storage.write_text_atomic(DRAFT_FILE, text);
            }
        }
    }

    fn persist_compare_draft(&self) {
        if self.compare_draft.len() <= MAX_DRAFT_BYTES {
            if let Some(storage) = &self.storage {
                let _ = storage.write_text_atomic(COMPARE_DRAFT_FILE, &self.compare_draft);
            }
        }
    }
}

impl Drop for JsonPage {
    fn drop(&mut self) {
        self.persist_draft(&self.draft);
        self.persist_compare_draft();
    }
}

impl Render for JsonPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::resolve(cx);
        let i18n = I18n::new(self.locale);
        let colors = theme.semantic.colors;
        let spacing = theme.semantic.spacing;
        let typography = &theme.semantic.typography;
        let page = cx.entity();
        let on_back = self.on_event.clone();

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
                            .id("json-back")
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
                                    .child(i18n.text(Message::JsonName)),
                            )
                            .child(
                                div()
                                    .text_size(typography.xs.size)
                                    .line_height(typography.xs.line_height)
                                    .text_color(colors.muted_foreground)
                                    .child(i18n.text(Message::JsonDescription)),
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
                    .child(tool_button(
                        "json-format",
                        Glyph::Format,
                        i18n.text(Message::JsonFormat),
                        page.clone(),
                        |page, window, cx| page.format(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "json-minify",
                        Glyph::Minify,
                        i18n.text(Message::JsonMinify),
                        page.clone(),
                        |page, window, cx| page.minify(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "json-sort",
                        Glyph::Sort,
                        i18n.text(Message::JsonSortKeys),
                        page.clone(),
                        |page, window, cx| page.sort_keys(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "json-compare",
                        Glyph::Compare,
                        i18n.text(if self.compare_mode {
                            Message::CompareClose
                        } else {
                            Message::CompareOpen
                        }),
                        page.clone(),
                        |page, window, cx| page.toggle_compare(window, cx),
                        &theme,
                    ))
                    .child(div().flex_1())
                    .child(tool_button(
                        "json-import",
                        Glyph::Clipboard,
                        i18n.text(Message::JsonImportClipboard),
                        page.clone(),
                        |page, window, cx| page.import_clipboard(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "json-copy",
                        Glyph::Copy,
                        i18n.text(Message::JsonCopy),
                        page.clone(),
                        |page, window, cx| page.copy(window, cx),
                        &theme,
                    ))
                    .child(tool_button(
                        "json-clear",
                        Glyph::Trash,
                        i18n.text(Message::JsonClear),
                        page,
                        |page, window, cx| page.clear(window, cx),
                        &theme,
                    )),
            )
            .child(div().h(theme.metrics.divider).bg(colors.border))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .p(spacing.md)
                    .flex()
                    .gap(spacing.md)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .gap(spacing.xs)
                            .when(self.compare_mode, |this| {
                                this.child(
                                    div()
                                        .text_size(typography.xs.size)
                                        .text_color(colors.muted_foreground)
                                        .child(i18n.text(Message::CompareLeft)),
                                )
                            })
                            .child(
                                Editor::new(&self.editor)
                                    .flex_1()
                                    .min_h(px(0.))
                                    .w_full()
                                    .rounded(theme.semantic.radius.lg)
                                    .border_1()
                                    .border_color(colors.input)
                                    .bg(theme.materials.surface),
                            ),
                    )
                    .when(self.compare_mode, |this| {
                        this.child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .flex()
                                .flex_col()
                                .gap(spacing.xs)
                                .child(
                                    div()
                                        .text_size(typography.xs.size)
                                        .text_color(colors.muted_foreground)
                                        .child(i18n.text(Message::CompareRight)),
                                )
                                .child(
                                    Editor::new(&self.compare_editor)
                                        .flex_1()
                                        .min_h(px(0.))
                                        .w_full()
                                        .rounded(theme.semantic.radius.lg)
                                        .border_1()
                                        .border_color(colors.input)
                                        .bg(theme.materials.surface),
                                ),
                        )
                    }),
            )
            .child(json_status(
                self.notice
                    .map(|message| i18n.text(message).to_owned())
                    .unwrap_or_else(|| validation_message(&self.validation, &i18n)),
                matches!(self.validation, Validation::Invalid { .. }),
                self.compare_mode.then(|| self.comparison.message(&i18n)),
                &i18n,
                &theme,
            ))
    }
}

fn tool_button(
    id: &'static str,
    icon: Glyph,
    label: &'static str,
    page: Entity<JsonPage>,
    action: impl Fn(&mut JsonPage, &mut Window, &mut Context<JsonPage>) + 'static,
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
        .on_click(move |_, window, cx| {
            page.update(cx, |page, cx| action(page, window, cx));
        })
        .child(glyph(icon, px(14.), colors.primary))
        .child(label)
}

fn json_status(
    message: String,
    invalid: bool,
    comparison: Option<&'static str>,
    i18n: &I18n,
    theme: &AppTheme,
) -> impl IntoElement {
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
        .child(
            div()
                .text_color(if invalid {
                    colors.destructive
                } else {
                    colors.muted_foreground
                })
                .child(message),
        )
        .child(
            div()
                .text_color(colors.muted_foreground)
                .child(comparison.unwrap_or_else(|| i18n.text(Message::JsonKeyboardHint))),
        )
}

fn compare_texts(left: &str, right: &str) -> Comparison {
    if right.trim().is_empty() {
        return Comparison::Empty;
    }
    match (
        serde_json::from_str::<Value>(left),
        serde_json::from_str::<Value>(right),
    ) {
        (Ok(left_value), Ok(right_value)) if left_value == right_value => {
            if left == right {
                Comparison::Equal
            } else {
                Comparison::FormattingDifferent
            }
        }
        (Ok(_), Ok(_)) => Comparison::Different,
        _ => Comparison::Invalid,
    }
}

fn validate(text: &str) -> Validation {
    if text.trim().is_empty() {
        return Validation::Empty;
    }
    match serde_json::from_str::<Value>(text) {
        Ok(_) => Validation::Valid {
            lines: text.lines().count().max(1),
            bytes: text.len(),
        },
        Err(error) => Validation::Invalid {
            line: error.line(),
            column: error.column(),
            message: json_error_message(&error),
        },
    }
}

fn validation_message(validation: &Validation, i18n: &I18n) -> String {
    match validation {
        Validation::Empty => i18n.text(Message::JsonEmpty).into(),
        Validation::Valid { lines, bytes } => i18n.json_valid_summary(*lines, *bytes),
        Validation::Invalid {
            line,
            column,
            message,
        } => i18n.json_error(*line, *column, message),
    }
}

fn sorted_value(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted = object
                .into_iter()
                .map(|(key, value)| (key, sorted_value(value)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(Map::from_iter(sorted))
        }
        Value::Array(values) => Value::Array(values.into_iter().map(sorted_value).collect()),
        value => value,
    }
}

fn json_error_message(error: &serde_json::Error) -> String {
    let message = error.to_string();
    let suffix = format!(" at line {} column {}", error.line(), error.column());
    message.strip_suffix(&suffix).unwrap_or(&message).to_owned()
}

fn error_offset(text: &str, line: usize, column: usize) -> usize {
    let mut line_start = 0;
    let mut current_line = 1;
    for segment in text.split_inclusive('\n') {
        if current_line == line {
            let line_text = segment.strip_suffix('\n').unwrap_or(segment);
            let character = column.saturating_sub(1);
            return line_start
                + line_text
                    .char_indices()
                    .nth(character)
                    .map(|(offset, _)| offset)
                    .unwrap_or(line_text.len());
        }
        line_start += segment.len();
        current_line += 1;
    }
    text.len()
}
