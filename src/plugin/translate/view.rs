//! Translation page and provider settings form.

use gpui_kit::{
    component::input::{Input, InputEvent, InputState, Textarea, TextareaState},
    prelude::*,
    *,
};

use crate::{
    i18n::{I18n, Locale, Message},
    icon::{Glyph, glyph},
    plugin::{PluginEvent, PluginEventHandler, storage::PluginStorage},
    theme::AppTheme,
};

use super::{
    MAX_TRANSLATION_BYTES, Provider, TranslationDirection, TranslationPreferences,
    load_preferences, save_preferences,
};

#[derive(Clone)]
enum PageStatus {
    Working,
    Copied,
    Failed(String),
}

#[derive(Clone)]
enum ConnectionStatus {
    Testing,
    Connected,
    Failed(String),
}

pub struct TranslatePage {
    source: Entity<TextareaState>,
    result: Option<String>,
    status: Option<PageStatus>,
    revision: u64,
    storage: Option<PluginStorage>,
    locale: Locale,
    on_event: PluginEventHandler,
    _source_subscription: Subscription,
    _escape_subscription: Subscription,
}

impl TranslatePage {
    pub fn new(
        storage: Option<PluginStorage>,
        locale: Locale,
        on_event: PluginEventHandler,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let source = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(I18n::new(locale).text(Message::TranslateInputPlaceholder))
                .auto_grow(5, 8)
        });
        let source_subscription = cx.subscribe(&source, |page, _, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            page.revision = page.revision.wrapping_add(1);
            page.result = None;
            page.status = None;
            cx.notify();
        });
        let source_focus = source.focus_handle(cx);
        let back_handler = on_event.clone();
        let escape_subscription = cx.intercept_keystrokes(move |event, window, cx| {
            if event.keystroke.key == "escape" && source_focus.is_focused(window) {
                back_handler(PluginEvent::Back, window, cx);
                cx.stop_propagation();
            }
        });

        Self {
            source,
            result: None,
            status: None,
            revision: 0,
            storage,
            locale,
            on_event,
            _source_subscription: source_subscription,
            _escape_subscription: escape_subscription,
        }
    }

    pub fn focus_handle(&self, cx: &App) -> gpui_kit::FocusHandle {
        self.source.focus_handle(cx)
    }

    pub fn set_source_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        if text.len() > MAX_TRANSLATION_BYTES {
            self.status = Some(PageStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateTooLong)
                    .to_owned(),
            ));
            cx.notify();
            return;
        }
        self.source
            .update(cx, |source, cx| source.set_value(text, window, cx));
        self.revision = self.revision.wrapping_add(1);
        self.result = None;
        self.status = None;
        cx.notify();
    }

    fn start_translation(&mut self, cx: &mut Context<Self>) {
        let source = self.source.read(cx).value().to_string();
        if source.trim().is_empty() {
            self.status = Some(PageStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateEmptyInput)
                    .to_owned(),
            ));
            cx.notify();
            return;
        }
        if source.len() > MAX_TRANSLATION_BYTES {
            self.status = Some(PageStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateTooLong)
                    .to_owned(),
            ));
            cx.notify();
            return;
        }

        let preferences = load_preferences(self.storage.as_ref());
        let Some(api_key) = preferences
            .api_keys
            .get(preferences.provider.storage_key())
            .filter(|key| !key.trim().is_empty())
            .cloned()
        else {
            self.status = Some(PageStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateApiKeyRequired)
                    .to_owned(),
            ));
            cx.notify();
            return;
        };
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        self.result = None;
        self.status = Some(PageStatus::Working);
        cx.notify();

        let request = cx
            .background_executor()
            .spawn(async move { super::translate(&preferences, &api_key, &source) });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |page, cx| {
                if page.revision != revision {
                    return;
                }
                match result {
                    Ok(translation) => {
                        page.result = Some(translation);
                        page.status = None;
                    }
                    Err(error) => {
                        page.status = Some(PageStatus::Failed(localized_error(page.locale, error)));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for TranslatePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::resolve(cx);
        let i18n = I18n::new(self.locale);
        let colors = theme.semantic.colors;
        let spacing = theme.semantic.spacing;
        let typography = &theme.semantic.typography;
        let page = cx.entity();
        let on_back = self.on_event.clone();
        let source = self.source.read(cx).value().to_string();
        let direction_label = if source.trim().is_empty() {
            i18n.text(Message::TranslateAutoDirection)
        } else {
            match TranslationDirection::detect(&source) {
                TranslationDirection::ChineseToEnglish => {
                    i18n.text(Message::TranslateChineseToEnglish)
                }
                TranslationDirection::EnglishToChinese => {
                    i18n.text(Message::TranslateEnglishToChinese)
                }
            }
        };
        let source_label = i18n.text(Message::TranslateSourceLabel);
        let output_label = i18n.text(Message::TranslateOutputLabel);
        let translate_label = if matches!(self.status, Some(PageStatus::Working)) {
            i18n.text(Message::TranslateWorking)
        } else {
            i18n.text(Message::TranslateStart)
        };
        let copy_label = i18n.text(Message::TranslateCopy);
        let output_empty = i18n.text(Message::TranslateOutputEmpty);
        let name = i18n.text(Message::TranslateName);
        let description = i18n.text(Message::TranslateDescription);

        let status = self.status.as_ref().map(|status| match status {
            PageStatus::Working => i18n.text(Message::TranslateWorking).to_owned(),
            PageStatus::Copied => i18n.text(Message::TranslateCopied).to_owned(),
            PageStatus::Failed(error) => error.clone(),
        });
        let status_is_error = matches!(self.status, Some(PageStatus::Failed(_)));
        let result = self.result.clone();
        let on_translate = page.clone();
        let on_copy = page.clone();
        let copy_event = self.on_event.clone();
        let result_to_copy = result.clone();
        let translate_disabled = matches!(self.status, Some(PageStatus::Working));
        let output_box = div()
            .id("translate-result-output")
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .px(spacing.md)
            .py(spacing.sm)
            .rounded(theme.semantic.radius.md)
            .border_1()
            .border_color(colors.input)
            .bg(theme.materials.window)
            .overflow_y_scroll()
            .when_some(result.clone(), |this, text| {
                this.text_size(typography.sm.size)
                    .line_height(typography.sm.line_height)
                    .text_color(colors.foreground)
                    .child(text)
            })
            .when(result.is_none(), |this| {
                this.flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .text_size(typography.xs.size)
                    .text_color(colors.muted_foreground)
                    .child(output_empty)
            });

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
                    .items_center()
                    .gap(spacing.md)
                    .child(
                        div()
                            .id("translate-back")
                            .size(theme.metrics.result_icon_box)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme.semantic.radius.md)
                            .cursor_pointer()
                            .hover(move |this| this.bg(theme.materials.hover))
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
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_size(typography.xs.size)
                                    .line_height(typography.xs.line_height)
                                    .text_color(colors.muted_foreground)
                                    .child(description),
                            ),
                    ),
            )
            .child(
                div()
                    .id("translate-page-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .px(spacing.xl)
                    .pb(spacing.lg)
                    .flex()
                    .flex_col()
                    .gap(spacing.md)
                    .child(
                        div().flex().items_center().child(
                            div()
                                .text_size(typography.xs.size)
                                .text_color(colors.primary)
                                .child(direction_label),
                        ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex_1()
                            .min_h(px(0.))
                            .flex()
                            .flex_col()
                            .gap(spacing.xs)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .gap(spacing.md)
                                    .child(
                                        div().flex_1().min_w(px(0.)).child(
                                            div()
                                                .h(px(34.))
                                                .flex()
                                                .items_center()
                                                .child(section_label(source_label, theme.clone())),
                                        ),
                                    )
                                    .child(div().w(px(68.)))
                                    .child(
                                        div().flex_1().min_w(px(0.)).child(
                                            div()
                                                .h(px(34.))
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(section_label(output_label, theme.clone()))
                                                .when_some(result.clone(), |this, _| {
                                                    this.child(action_button(
                                                        "translate-copy",
                                                        copy_label,
                                                        false,
                                                        theme.clone(),
                                                        move |window, cx| {
                                                            if let Some(text) =
                                                                result_to_copy.clone()
                                                            {
                                                                let _ = on_copy.update(
                                                                    cx,
                                                                    |page, cx| {
                                                                        page.status = Some(
                                                                            PageStatus::Copied,
                                                                        );
                                                                        cx.notify();
                                                                    },
                                                                );
                                                                copy_event(
                                                                    PluginEvent::WriteClipboard(
                                                                        text,
                                                                    ),
                                                                    window,
                                                                    cx,
                                                                );
                                                            }
                                                        },
                                                    ))
                                                }),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex_1()
                                    .min_h(px(0.))
                                    .flex()
                                    .gap(spacing.md)
                                    .child(
                                        div()
                                            .id("translate-source-input")
                                            .flex_1()
                                            .min_w(px(0.))
                                            .min_h(px(0.))
                                            .flex()
                                            .flex_col()
                                            .rounded(theme.semantic.radius.md)
                                            .border_1()
                                            .border_color(colors.input)
                                            .bg(theme.materials.window)
                                            .child(
                                                Textarea::new(&self.source)
                                                    .h(relative(1.))
                                                    .px(spacing.md)
                                                    .py(spacing.sm)
                                                    .bordered(false),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(68.))
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .justify_center()
                                            .gap(spacing.xs)
                                            .child(
                                                div()
                                                    .id("translate-start")
                                                    .size(px(34.))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(theme.semantic.radius.md)
                                                    .bg(colors.primary)
                                                    .text_color(
                                                        theme.materials.selection_foreground,
                                                    )
                                                    .text_size(typography.md.size)
                                                    .when(!translate_disabled, |this| {
                                                        this.cursor_pointer()
                                                            .hover(|this| this.opacity(0.86))
                                                    })
                                                    .when(translate_disabled, |this| {
                                                        this.opacity(0.6)
                                                    })
                                                    .on_click(move |_, _, cx| {
                                                        if !translate_disabled {
                                                            on_translate.update(cx, |page, cx| {
                                                                page.start_translation(cx)
                                                            });
                                                        }
                                                    })
                                                    .child("→"),
                                            )
                                            .child(
                                                div()
                                                    .w_full()
                                                    .text_center()
                                                    .text_size(typography.xs.size)
                                                    .text_color(colors.muted_foreground)
                                                    .child(translate_label),
                                            ),
                                    )
                                    .child(output_box),
                            ),
                    )
                    .child(
                        div()
                            .h(px(34.))
                            .flex()
                            .items_center()
                            .text_size(typography.xs.size)
                            .text_color(if status_is_error {
                                colors.destructive
                            } else {
                                colors.muted_foreground
                            })
                            .child(status.unwrap_or_default()),
                    ),
            )
    }
}

pub struct TranslateSettingsPanel {
    provider: Provider,
    endpoint: Entity<InputState>,
    model: Entity<InputState>,
    api_key: Entity<InputState>,
    has_saved_key: bool,
    revision: u64,
    status: Option<ConnectionStatus>,
    storage: Option<PluginStorage>,
    locale: Locale,
    _endpoint_subscription: Subscription,
    _model_subscription: Subscription,
}

impl TranslateSettingsPanel {
    pub fn new(
        preferences: TranslationPreferences,
        storage: Option<PluginStorage>,
        locale: Locale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let endpoint = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(preferences.endpoint.clone())
                .placeholder(I18n::new(locale).text(Message::TranslateEndpointPlaceholder))
        });
        let model = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(preferences.model.clone())
                .placeholder(I18n::new(locale).text(Message::TranslateModelPlaceholder))
        });
        let api_key = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(I18n::new(locale).text(Message::TranslateApiKeyPlaceholder))
                .masked(true)
        });
        let has_saved_key = preferences
            .api_keys
            .get(preferences.provider.storage_key())
            .is_some_and(|key| !key.trim().is_empty());
        let endpoint_storage = storage.clone();
        let endpoint_subscription =
            cx.subscribe(&endpoint, move |panel, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    panel.revision = panel.revision.wrapping_add(1);
                    panel.status = None;
                    panel.persist_preferences(cx, endpoint_storage.as_ref());
                    cx.notify();
                }
            });
        let model_storage = storage.clone();
        let model_subscription = cx.subscribe(&model, move |panel, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                panel.revision = panel.revision.wrapping_add(1);
                panel.status = None;
                panel.persist_preferences(cx, model_storage.as_ref());
                cx.notify();
            }
        });
        Self {
            provider: preferences.provider,
            endpoint,
            model,
            api_key,
            has_saved_key,
            revision: 0,
            status: None,
            storage,
            locale,
            _endpoint_subscription: endpoint_subscription,
            _model_subscription: model_subscription,
        }
    }

    fn preferences(&self, cx: &App) -> TranslationPreferences {
        let mut preferences = load_preferences(self.storage.as_ref());
        preferences.provider = self.provider;
        preferences.endpoint = self.endpoint.read(cx).value().to_string();
        preferences.model = self.model.read(cx).value().to_string();
        preferences
    }

    fn persist_preferences(&self, cx: &App, storage: Option<&PluginStorage>) -> bool {
        let preferences = self.preferences(cx);
        save_preferences(storage, &preferences)
    }

    fn select_provider(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
        if self.provider == provider {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
        self.provider = provider;
        let (endpoint, model) = provider.defaults();
        self.endpoint.update(cx, |input, cx| {
            input.set_value(endpoint, window, cx);
        });
        self.model.update(cx, |input, cx| {
            input.set_value(model, window, cx);
        });
        self.api_key
            .update(cx, |input, cx| input.set_value("", window, cx));
        let preferences = self.preferences(cx);
        self.has_saved_key = preferences
            .api_keys
            .get(provider.storage_key())
            .is_some_and(|key| !key.trim().is_empty());
        self.status = None;
        if !self.persist_preferences(cx, self.storage.as_ref()) {
            self.status = Some(ConnectionStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateSaveFailed)
                    .to_owned(),
            ));
        }
        cx.notify();
    }

    fn start_test(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.status, Some(ConnectionStatus::Testing)) {
            return;
        }
        let mut preferences = self.preferences(cx);
        let key_input = self.api_key.read(cx).value().trim().to_owned();
        if !key_input.is_empty() {
            preferences
                .api_keys
                .insert(self.provider.storage_key().to_owned(), key_input.clone());
        }
        let Some(api_key) = preferences
            .api_keys
            .get(self.provider.storage_key())
            .filter(|key| !key.trim().is_empty())
            .cloned()
        else {
            self.status = Some(ConnectionStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateApiKeyRequired)
                    .to_owned(),
            ));
            cx.notify();
            return;
        };
        if !save_preferences(self.storage.as_ref(), &preferences) {
            self.status = Some(ConnectionStatus::Failed(
                I18n::new(self.locale)
                    .text(Message::TranslateSaveFailed)
                    .to_owned(),
            ));
            cx.notify();
            return;
        }
        self.has_saved_key = true;
        if !key_input.is_empty() {
            self.api_key
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.status = Some(ConnectionStatus::Testing);
        self.revision = self.revision.wrapping_add(1);
        let revision = self.revision;
        cx.notify();

        let request = cx
            .background_executor()
            .spawn(async move { super::test_connection(&preferences, &api_key) });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |panel, cx| {
                if panel.revision != revision {
                    return;
                }
                panel.status = Some(match result {
                    Ok(()) => ConnectionStatus::Connected,
                    Err(error) => ConnectionStatus::Failed(localized_error(panel.locale, error)),
                });
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for TranslateSettingsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::resolve(cx);
        let i18n = I18n::new(self.locale);
        let colors = theme.semantic.colors;
        let spacing = theme.semantic.spacing;
        let typography = &theme.semantic.typography;
        let panel = cx.entity();
        let status = self.status.clone();
        let status_text = status.as_ref().map(|status| match status {
            ConnectionStatus::Testing => i18n.text(Message::TranslateTesting).to_owned(),
            ConnectionStatus::Connected => i18n.text(Message::TranslateConnected).to_owned(),
            ConnectionStatus::Failed(error) => error.clone(),
        });
        let status_error = matches!(status, Some(ConnectionStatus::Failed(_)));
        let provider_choices = [
            (Provider::OpenAi, Message::TranslateOpenAI),
            (Provider::DeepSeek, Message::TranslateDeepSeek),
            (Provider::Custom, Message::TranslateCustom),
        ];

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(spacing.lg)
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(spacing.lg)
                    .child(setting_label(i18n.text(Message::TranslateProvider), &theme))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(spacing.xs)
                            .p(spacing.xxs)
                            .rounded(theme.semantic.radius.md)
                            .border_1()
                            .border_color(colors.border)
                            .bg(theme.materials.window)
                            .children(provider_choices.into_iter().map(|(provider, message)| {
                                let active = self.provider == provider;
                                let provider_handler = panel.clone();
                                div()
                                    .id(format!("translate-provider-{}", provider.storage_key()))
                                    .h(px(32.))
                                    .px(spacing.md)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(theme.semantic.radius.md)
                                    .cursor_pointer()
                                    .bg(if active {
                                        theme.materials.selection
                                    } else {
                                        theme.materials.window
                                    })
                                    .text_color(if active {
                                        theme.materials.selection_foreground
                                    } else {
                                        colors.foreground
                                    })
                                    .text_size(typography.xs.size)
                                    .font_weight(if active {
                                        gpui_kit::FontWeight::SEMIBOLD
                                    } else {
                                        gpui_kit::FontWeight::NORMAL
                                    })
                                    .on_click(move |_, window, cx| {
                                        provider_handler.update(cx, |panel, cx| {
                                            panel.select_provider(provider, window, cx)
                                        });
                                    })
                                    .child(i18n.text(message))
                            })),
                    ),
            )
            .child(field(
                i18n.text(Message::TranslateEndpoint),
                &self.endpoint,
                theme.clone(),
            ))
            .child(field(
                i18n.text(Message::TranslateModel),
                &self.model,
                theme.clone(),
            ))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(spacing.xs)
                    .child(setting_label(i18n.text(Message::TranslateApiKey), &theme))
                    .child(
                        div()
                            .w_full()
                            .h(px(40.))
                            .px(spacing.sm)
                            .flex()
                            .items_center()
                            .rounded(theme.semantic.radius.md)
                            .border_1()
                            .border_color(colors.input)
                            .bg(theme.materials.window)
                            .child(Input::new(&self.api_key).w_full().bordered(false)),
                    )
                    .when(self.has_saved_key, |this| {
                        this.child(
                            div()
                                .text_size(typography.xs.size)
                                .text_color(colors.muted_foreground)
                                .child(i18n.text(Message::TranslateApiKeySaved)),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(spacing.md)
                    .child(action_button(
                        "translate-save-and-test",
                        i18n.text(Message::TranslateSaveAndTest),
                        true,
                        theme.clone(),
                        move |window, cx| {
                            panel.update(cx, |panel, cx| panel.start_test(window, cx));
                        },
                    ))
                    .when_some(status_text, |this, text| {
                        this.child(
                            div()
                                .text_size(typography.xs.size)
                                .text_color(if status_error {
                                    colors.destructive
                                } else {
                                    colors.muted_foreground
                                })
                                .child(text),
                        )
                    }),
            )
            .child(
                div()
                    .text_size(typography.xs.size)
                    .line_height(typography.xs.line_height)
                    .text_color(colors.muted_foreground)
                    .child(i18n.text(Message::TranslatePrivacyNote)),
            )
    }
}

fn section_label(label: &'static str, theme: AppTheme) -> impl IntoElement {
    div()
        .text_size(theme.semantic.typography.xs.size)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(theme.semantic.colors.foreground)
        .child(label)
}

fn setting_label(label: &'static str, theme: &AppTheme) -> impl IntoElement {
    div()
        .text_size(theme.semantic.typography.sm.size)
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(theme.semantic.colors.foreground)
        .child(label)
}

fn field(label: &'static str, input: &Entity<InputState>, theme: AppTheme) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(theme.semantic.spacing.xs)
        .child(setting_label(label, &theme))
        .child(
            div()
                .w_full()
                .h(px(40.))
                .px(theme.semantic.spacing.sm)
                .flex()
                .items_center()
                .rounded(theme.semantic.radius.md)
                .border_1()
                .border_color(theme.semantic.colors.input)
                .bg(theme.materials.window)
                .child(Input::new(input).w_full().bordered(false)),
        )
}

fn action_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    theme: AppTheme,
    action: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let background = if primary {
        colors.primary
    } else {
        theme.materials.control
    };
    let foreground = if primary {
        theme.materials.selection_foreground
    } else {
        colors.foreground
    };
    div()
        .id(id)
        .h(px(34.))
        .px(theme.semantic.spacing.md)
        .flex()
        .items_center()
        .justify_center()
        .gap(theme.semantic.spacing.xs)
        .rounded(theme.semantic.radius.md)
        .bg(background)
        .text_color(foreground)
        .text_size(theme.semantic.typography.xs.size)
        .cursor_pointer()
        .hover(move |this| this.opacity(0.86))
        .on_click(move |_, window, cx| action(window, cx))
        .child(label)
}

fn localized_error(locale: Locale, error: String) -> String {
    match locale {
        Locale::ZhCn => format!("连接或翻译失败：{error}"),
        Locale::EnUs => format!("Request failed: {error}"),
    }
}
