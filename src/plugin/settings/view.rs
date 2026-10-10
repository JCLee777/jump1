//! Settings plugin view.

use std::rc::Rc;

use gpui_kit::{
    component::{
        input::{Input, InputState, NumberInput},
        switch::Switch,
    },
    prelude::*,
    *,
};

use crate::{
    hotkey,
    i18n::{I18n, Locale, Message},
    icon::{Glyph, glyph},
    plugin::{
        PluginId,
        shortcuts::{ShortcutBinding, ShortcutBindingEvent},
    },
    theme::{AppTheme, PanelSizePreset},
};

use super::{SettingDefinition, SettingsChange, SettingsSection};
use crate::plugin::system::{SystemChange, SystemPlugin, ThemeChoice};

pub type BackHandler = Rc<dyn for<'a, 'b> Fn(&'a mut Window, &'b mut App)>;
pub type ChangeHandler = Rc<dyn for<'a, 'b> Fn(SettingsChange, &'a mut Window, &'b mut App)>;
pub type SectionHandler = Rc<dyn for<'a, 'b> Fn(SettingsSection, &'a mut Window, &'b mut App)>;
pub type ShortcutHandler =
    Rc<dyn for<'a, 'b> Fn(ShortcutBindingEvent, &'a mut Window, &'b mut App)>;

#[allow(clippy::too_many_arguments)]
pub fn render(
    input: &Entity<InputState>,
    panel_width_input: &Entity<InputState>,
    panel_height_input: &Entity<InputState>,
    plugin_settings: Vec<SettingDefinition>,
    shortcut_bindings: Vec<ShortcutBinding>,
    recording_shortcut: Option<u64>,
    editing_query: Option<u64>,
    shortcut_conflicts: Vec<u64>,
    selected_section: SettingsSection,
    system: &SystemPlugin,
    i18n: &I18n,
    theme: &AppTheme,
    on_back: BackHandler,
    on_change: ChangeHandler,
    on_section: SectionHandler,
    on_shortcut: ShortcutHandler,
) -> AnyElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let mut plugin_ids = Vec::new();
    for setting in &plugin_settings {
        if !plugin_ids.contains(&setting.plugin) {
            plugin_ids.push(setting.plugin);
        }
    }
    let selected_plugin = match selected_section {
        SettingsSection::Plugin(plugin) => Some(plugin),
        _ => None,
    };
    let content_id = match selected_section {
        SettingsSection::General => "settings-content-general".to_owned(),
        SettingsSection::Shortcuts => "settings-content-shortcuts".to_owned(),
        SettingsSection::Plugin(plugin) => format!("settings-content-{}", plugin.slug()),
    };

    div()
        .size_full()
        .flex()
        .font_family(typography.sans.clone())
        .bg(theme.materials.window)
        .text_color(colors.foreground)
        .child(
            div()
                .w(theme.metrics.settings_sidebar_width)
                .h_full()
                .flex()
                .flex_col()
                .p(spacing.md)
                .border_r_1()
                .border_color(colors.border)
                .bg(theme.materials.sidebar)
                .child(
                    div()
                        .h(px(44.))
                        .flex()
                        .items_center()
                        .gap(spacing.sm)
                        .mb(spacing.md)
                        .child(
                            div()
                                .id("settings-back")
                                .size(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(theme.semantic.radius.md)
                                .cursor_pointer()
                                .hover(move |this| this.bg(colors.muted))
                                .on_click(move |_, window, cx| on_back(window, cx))
                                .child(glyph(
                                    Glyph::ArrowLeft,
                                    theme.metrics.result_icon,
                                    colors.primary,
                                )),
                        )
                        .child(
                            div()
                                .text_size(typography.sm.size)
                                .line_height(typography.sm.line_height)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(i18n.text(Message::SettingsName)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(spacing.xs)
                        .child(sidebar_group_heading(
                            i18n.text(Message::SettingsSystem),
                            theme,
                        ))
                        .child(sidebar_item(
                            "settings-general",
                            i18n.text(Message::SettingsGeneral),
                            Glyph::Appearance,
                            selected_section == SettingsSection::General,
                            SettingsSection::General,
                            &on_section,
                            theme,
                        ))
                        .child(sidebar_item(
                            "settings-shortcuts",
                            i18n.text(Message::ShortcutBindings),
                            Glyph::Application,
                            selected_section == SettingsSection::Shortcuts,
                            SettingsSection::Shortcuts,
                            &on_section,
                            theme,
                        ))
                        .child(div().h(spacing.md))
                        .child(sidebar_group_heading(
                            i18n.text(Message::SettingsPlugins),
                            theme,
                        ))
                        .children(plugin_ids.into_iter().map(|plugin| {
                            let (title, _, icon) = plugin_presentation(plugin, i18n);
                            sidebar_item(
                                plugin.slug(),
                                title,
                                icon,
                                selected_section == SettingsSection::Plugin(plugin),
                                SettingsSection::Plugin(plugin),
                                &on_section,
                                theme,
                            )
                        })),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .px(spacing.sm)
                        .pb(spacing.xs)
                        .text_size(typography.xs.size)
                        .line_height(typography.xs.line_height)
                        .text_color(colors.muted_foreground)
                        .child(i18n.text(Message::SettingsKeyboardHint)),
                ),
        )
        .child(
            div()
                .id(content_id)
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .overflow_y_scroll()
                .px(spacing.xl)
                .py(spacing.xl)
                .when(selected_section == SettingsSection::General, |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(spacing.lg)
                            .child(content_heading(
                                i18n.text(Message::SettingsGeneral),
                                i18n.text(Message::SettingsGeneralDescription),
                                theme,
                            ))
                            .child(
                                div()
                                    .w_full()
                                    .rounded(theme.semantic.radius.lg)
                                    .border_1()
                                    .border_color(colors.border)
                                    .bg(theme.materials.surface)
                                    .child(setting_row(
                                        i18n.text(Message::SettingsTheme),
                                        [
                                            (ThemeChoice::System, i18n.text(Message::ThemeSystem)),
                                            (ThemeChoice::Light, i18n.text(Message::ThemeLight)),
                                            (ThemeChoice::Dark, i18n.text(Message::ThemeDark)),
                                        ],
                                        system.theme(),
                                        &on_change,
                                        theme,
                                    ))
                                    .child(
                                        div()
                                            .h(theme.metrics.divider)
                                            .ml(spacing.lg)
                                            .bg(colors.border),
                                    )
                                    .child(setting_row(
                                        i18n.text(Message::SettingsLanguage),
                                        [
                                            (Locale::ZhCn, i18n.text(Message::LanguageChinese)),
                                            (Locale::EnUs, i18n.text(Message::LanguageEnglish)),
                                        ],
                                        system.locale(),
                                        &on_change,
                                        theme,
                                    ))
                                    .child(
                                        div()
                                            .h(theme.metrics.divider)
                                            .ml(spacing.lg)
                                            .bg(colors.border),
                                    )
                                    .child(setting_row(
                                        i18n.text(Message::SettingsPanelSize),
                                        [
                                            (
                                                PanelSizePreset::Small,
                                                i18n.text(Message::PanelSizeSmall),
                                            ),
                                            (
                                                PanelSizePreset::Medium,
                                                i18n.text(Message::PanelSizeMedium),
                                            ),
                                            (
                                                PanelSizePreset::Large,
                                                i18n.text(Message::PanelSizeLarge),
                                            ),
                                            (
                                                PanelSizePreset::Custom,
                                                i18n.text(Message::PanelSizeCustom),
                                            ),
                                        ],
                                        system.panel_size().preset,
                                        &on_change,
                                        theme,
                                    ))
                                    .when(
                                        system.panel_size().preset == PanelSizePreset::Custom,
                                        |this| {
                                            this.child(panel_dimensions_row(
                                                panel_width_input,
                                                panel_height_input,
                                                i18n,
                                                theme,
                                            ))
                                        },
                                    )
                                    .child(
                                        div()
                                            .h(theme.metrics.divider)
                                            .ml(spacing.lg)
                                            .bg(colors.border),
                                    )
                                    .child(boolean_setting_row(
                                        "settings-launch-at-login",
                                        i18n.text(Message::SettingsLaunchAtLogin),
                                        i18n.text(Message::SettingsLaunchAtLoginDescription),
                                        system.launch_at_login(),
                                        SystemChange::LaunchAtLogin,
                                        &on_change,
                                        theme,
                                    )),
                            ),
                    )
                })
                .when_some(selected_plugin, |this, plugin| {
                    let (title, description, _) = plugin_presentation(plugin, i18n);
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(spacing.lg)
                            .child(content_heading(title, description, theme))
                            .children(
                                plugin_settings
                                    .into_iter()
                                    .filter(|setting| setting.plugin == plugin)
                                    .map(|setting| plugin_setting_row(setting, &on_change, theme)),
                            ),
                    )
                })
                .when(selected_section == SettingsSection::Shortcuts, |this| {
                    this.child(shortcut_bindings_panel(
                        input,
                        shortcut_bindings,
                        recording_shortcut,
                        editing_query,
                        shortcut_conflicts,
                        i18n,
                        theme,
                        &on_shortcut,
                    ))
                }),
        )
        .into_any_element()
}

fn plugin_presentation(plugin: PluginId, i18n: &I18n) -> (&'static str, &'static str, Glyph) {
    let (title, description, icon) = match plugin {
        PluginId::Launcher => (
            Message::SettingsLauncher,
            Message::SettingsLauncherDescription,
            Glyph::Search,
        ),
        PluginId::Clipboard => (
            Message::ClipboardName,
            Message::ClipboardDescription,
            Glyph::Clipboard,
        ),
        PluginId::Json => (Message::JsonName, Message::JsonDescription, Glyph::Json),
        PluginId::Yaml => (Message::YamlName, Message::YamlDescription, Glyph::Yaml),
        _ => (
            Message::SettingsPlugins,
            Message::SettingsPluginsDescription,
            Glyph::Plugins,
        ),
    };
    (i18n.text(title), i18n.text(description), icon)
}

fn sidebar_group_heading(title: &'static str, theme: &AppTheme) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    div()
        .px(spacing.sm)
        .py(spacing.xs)
        .text_size(typography.xs.size)
        .line_height(typography.xs.line_height)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(colors.muted_foreground)
        .child(title)
}

#[allow(clippy::too_many_arguments)]
fn sidebar_item(
    id: &'static str,
    title: &'static str,
    icon: Glyph,
    active: bool,
    section: SettingsSection,
    on_section: &SectionHandler,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let callback = on_section.clone();

    div()
        .id(id)
        .w_full()
        .h(theme.metrics.settings_sidebar_item_height)
        .px(spacing.sm)
        .flex()
        .items_center()
        .gap(spacing.md)
        .rounded(theme.semantic.radius.md)
        .cursor_pointer()
        .bg(if active {
            colors.accent
        } else {
            transparent_black()
        })
        .hover(move |this| this.bg(colors.muted))
        .on_click(move |_, window, cx| callback(section, window, cx))
        .child(
            div()
                .size(theme.metrics.settings_sidebar_icon_box)
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.semantic.radius.md)
                .bg(colors.primary)
                .child(glyph(
                    icon,
                    theme.metrics.settings_sidebar_icon,
                    colors.primary_foreground,
                )),
        )
        .child(
            div()
                .text_size(typography.sm.size)
                .line_height(typography.sm.line_height)
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .child(title),
        )
}

fn content_heading(
    title: &'static str,
    description: &'static str,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;

    div()
        .flex()
        .flex_col()
        .gap(spacing.xs)
        .mb(spacing.sm)
        .child(
            div()
                .text_size(typography.xl.size)
                .line_height(typography.xl.line_height)
                .font_weight(FontWeight::BOLD)
                .child(title),
        )
        .child(
            div()
                .text_size(typography.sm.size)
                .line_height(typography.sm.line_height)
                .text_color(colors.muted_foreground)
                .child(description),
        )
}

fn setting_row<T: Copy + Eq + 'static, const N: usize>(
    title: &'static str,
    options: [(T, &'static str); N],
    selected: T,
    on_change: &ChangeHandler,
    theme: &AppTheme,
) -> impl IntoElement
where
    SettingsChange: From<T>,
{
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;

    div()
        .w_full()
        .min_h(px(68.))
        .px(spacing.lg)
        .flex()
        .items_center()
        .justify_between()
        .gap(spacing.xl)
        .child(
            div()
                .text_size(typography.sm.size)
                .line_height(typography.sm.line_height)
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(segmented_options(options, selected, on_change, theme))
}

fn panel_dimensions_row(
    width_input: &Entity<InputState>,
    height_input: &Entity<InputState>,
    i18n: &I18n,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let width_label = div()
        .text_size(typography.xs.size)
        .text_color(colors.muted_foreground)
        .child(i18n.text(Message::SettingsPanelWidth));
    let height_label = div()
        .text_size(typography.xs.size)
        .text_color(colors.muted_foreground)
        .child(i18n.text(Message::SettingsPanelHeight));

    div()
        .w_full()
        .min_h(px(76.))
        .px(spacing.lg)
        .flex()
        .items_center()
        .justify_between()
        .gap(spacing.xl)
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
                        .font_weight(FontWeight::MEDIUM)
                        .child(i18n.text(Message::SettingsPanelSizeCustomDimensions)),
                )
                .child(
                    div()
                        .text_size(typography.xs.size)
                        .text_color(colors.muted_foreground)
                        .child(i18n.text(Message::SettingsPanelSizeDescription)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(spacing.sm)
                .child(
                    NumberInput::new(width_input)
                        .w(px(144.))
                        .prefix(width_label)
                        .suffix("px"),
                )
                .child(
                    NumberInput::new(height_input)
                        .w(px(144.))
                        .prefix(height_label)
                        .suffix("px"),
                ),
        )
}

#[allow(clippy::too_many_arguments)]
fn boolean_setting_row(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    checked: bool,
    change: fn(bool) -> SystemChange,
    on_change: &ChangeHandler,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let callback = on_change.clone();

    div()
        .w_full()
        .min_h(px(76.))
        .px(spacing.lg)
        .py(spacing.md)
        .flex()
        .items_center()
        .justify_between()
        .gap(spacing.xl)
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
                        .child(title),
                )
                .child(
                    div()
                        .text_size(typography.xs.size)
                        .line_height(typography.xs.line_height)
                        .text_color(colors.muted_foreground)
                        .child(description),
                ),
        )
        .child(
            Switch::new(id)
                .checked(checked)
                .accessibility_label(title)
                .on_click(move |checked, window, cx| {
                    callback(SettingsChange::System(change(*checked)), window, cx)
                }),
        )
}

fn segmented_options<T: Copy + Eq + 'static, const N: usize>(
    options: [(T, &'static str); N],
    selected: T,
    on_change: &ChangeHandler,
    theme: &AppTheme,
) -> impl IntoElement
where
    SettingsChange: From<T>,
{
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;

    div()
        .flex()
        .items_center()
        .gap(spacing.xxs)
        .p(spacing.xxs)
        .rounded(theme.semantic.radius.md)
        .border_1()
        .border_color(colors.border)
        .bg(theme.materials.window)
        .children(options.into_iter().map(|(value, label)| {
            let active = value == selected;
            let callback = on_change.clone();
            div()
                .id(label)
                .h(px(30.))
                .px(spacing.md)
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.semantic.radius.md)
                .cursor_pointer()
                .bg(if active {
                    colors.accent
                } else {
                    transparent_black()
                })
                .hover(move |this| this.bg(colors.muted))
                .on_click(move |_, window, cx| callback(SettingsChange::from(value), window, cx))
                .child(
                    div()
                        .text_size(typography.xs.size)
                        .line_height(typography.xs.line_height)
                        .font_weight(if active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .child(label),
                )
        }))
}

fn plugin_setting_row(
    setting: SettingDefinition,
    on_change: &ChangeHandler,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let plugin = setting.plugin;
    let key = setting.key;

    div()
        .w_full()
        .min_h(px(76.))
        .px(spacing.lg)
        .py(spacing.md)
        .flex()
        .items_center()
        .justify_between()
        .gap(spacing.xl)
        .rounded(theme.semantic.radius.lg)
        .border_1()
        .border_color(colors.border)
        .bg(theme.materials.surface)
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
                        .child(setting.title),
                )
                .child(
                    div()
                        .text_size(typography.xs.size)
                        .line_height(typography.xs.line_height)
                        .text_color(colors.muted_foreground)
                        .child(setting.description),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(spacing.xxs)
                .p(spacing.xxs)
                .rounded(theme.semantic.radius.md)
                .border_1()
                .border_color(colors.border)
                .bg(theme.materials.window)
                .children(setting.options.into_iter().map(|(value, label)| {
                    let active = value == setting.selected;
                    let callback = on_change.clone();
                    div()
                        .id(format!("setting-{key}-{value}"))
                        .h(px(30.))
                        .px(spacing.md)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme.semantic.radius.md)
                        .cursor_pointer()
                        .bg(if active {
                            colors.accent
                        } else {
                            transparent_black()
                        })
                        .hover(move |this| this.bg(colors.muted))
                        .on_click(move |_, window, cx| {
                            callback(
                                SettingsChange::Plugin {
                                    plugin,
                                    key: key.to_owned(),
                                    value: value.clone(),
                                },
                                window,
                                cx,
                            )
                        })
                        .child(
                            div()
                                .text_size(typography.xs.size)
                                .font_weight(if active {
                                    FontWeight::SEMIBOLD
                                } else {
                                    FontWeight::NORMAL
                                })
                                .child(label),
                        )
                })),
        )
}

#[allow(clippy::too_many_arguments)]
fn shortcut_bindings_panel(
    input: &Entity<InputState>,
    bindings: Vec<ShortcutBinding>,
    recording_shortcut: Option<u64>,
    editing_query: Option<u64>,
    shortcut_conflicts: Vec<u64>,
    i18n: &I18n,
    theme: &AppTheme,
    on_shortcut: &ShortcutHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let add_handler = on_shortcut.clone();

    div()
        .flex()
        .flex_col()
        .gap(spacing.lg)
        .child(content_heading(
            i18n.text(Message::ShortcutBindings),
            i18n.text(Message::ShortcutBindingsDescription),
            theme,
        ))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(spacing.sm)
                .px(spacing.md)
                .text_size(typography.xs.size)
                .line_height(typography.xs.line_height)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.muted_foreground)
                .child(
                    div()
                        .w(theme.metrics.shortcut_key_width)
                        .child(i18n.text(Message::ShortcutKeyColumn)),
                )
                .child(
                    div()
                        .flex_1()
                        .child(i18n.text(Message::ShortcutTargetColumn)),
                )
                .child(
                    div()
                        .w(theme.metrics.shortcut_copy_width)
                        .text_center()
                        .child(i18n.text(Message::ShortcutAutoCopyColumn)),
                )
                .child(div().w(px(30.))),
        )
        .child(
            div()
                .w_full()
                .rounded(theme.semantic.radius.lg)
                .border_1()
                .border_color(colors.border)
                .bg(theme.materials.surface)
                .when(bindings.is_empty(), |this| {
                    this.child(
                        div()
                            .p(spacing.xl)
                            .text_size(typography.sm.size)
                            .text_color(colors.muted_foreground)
                            .child(i18n.text(Message::ShortcutBindingsEmpty)),
                    )
                })
                .children(bindings.into_iter().map(|binding| {
                    let conflicted = shortcut_conflicts.contains(&binding.id);
                    shortcut_binding_row(
                        input,
                        binding,
                        recording_shortcut,
                        editing_query,
                        conflicted,
                        i18n,
                        theme,
                        on_shortcut,
                    )
                })),
        )
        .child(
            div()
                .id("shortcut-binding-add")
                .h(px(34.))
                .px(spacing.md)
                .flex()
                .items_center()
                .self_start()
                .rounded(theme.semantic.radius.md)
                .border_1()
                .border_color(colors.border)
                .cursor_pointer()
                .hover(move |this| this.bg(colors.muted))
                .on_click(move |_, window, cx| add_handler(ShortcutBindingEvent::Add, window, cx))
                .child(i18n.text(Message::ShortcutBindingAdd)),
        )
}

#[allow(clippy::too_many_arguments)]
fn shortcut_binding_row(
    input: &Entity<InputState>,
    binding: ShortcutBinding,
    recording_shortcut: Option<u64>,
    editing_query: Option<u64>,
    conflicted: bool,
    i18n: &I18n,
    theme: &AppTheme,
    on_shortcut: &ShortcutHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let id = binding.id;
    let record_handler = on_shortcut.clone();
    let edit_handler = on_shortcut.clone();
    let remove_handler = on_shortcut.clone();
    let copy_handler = on_shortcut.clone();
    let recording = recording_shortcut == Some(id);
    let editing = editing_query == Some(id);
    let shortcut = if recording {
        i18n.text(Message::ShortcutRecording).into()
    } else if binding.shortcut.is_empty() {
        i18n.text(Message::ShortcutRecord).into()
    } else {
        hotkey::shortcut_label(&binding.shortcut)
    };
    let shortcut = if conflicted {
        format!("{shortcut} · {}", i18n.text(Message::ShortcutConflict))
    } else {
        shortcut
    };

    div()
        .w_full()
        .min_h(px(58.))
        .px(spacing.md)
        .flex()
        .items_center()
        .gap(spacing.sm)
        .border_b_1()
        .border_color(colors.border)
        .child(
            div()
                .id(format!("shortcut-record-{id}"))
                .w(theme.metrics.shortcut_key_width)
                .h(px(34.))
                .px(spacing.sm)
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.semantic.radius.md)
                .border_1()
                .border_color(if conflicted {
                    colors.primary
                } else if recording {
                    colors.primary
                } else {
                    colors.input
                })
                .bg(theme.materials.window)
                .cursor_pointer()
                .text_size(typography.xs.size)
                .text_color(if conflicted {
                    colors.primary
                } else if recording {
                    colors.primary
                } else {
                    colors.foreground
                })
                .on_click(move |_, window, cx| {
                    record_handler(ShortcutBindingEvent::Record(id), window, cx)
                })
                .child(shortcut),
        )
        .child(
            div()
                .id(format!("shortcut-query-{id}"))
                .h(px(34.))
                .flex_1()
                .min_w(px(0.))
                .px(spacing.sm)
                .flex()
                .items_center()
                .rounded(theme.semantic.radius.md)
                .border_1()
                .border_color(if editing {
                    colors.primary
                } else {
                    colors.input
                })
                .bg(theme.materials.window)
                .cursor_text()
                .on_click(move |_, window, cx| {
                    edit_handler(ShortcutBindingEvent::EditQuery(id), window, cx)
                })
                .when(editing, |this| {
                    this.child(Input::new(input).w_full().bordered(false))
                })
                .when(!editing, |this| {
                    this.child(
                        div()
                            .w_full()
                            .text_size(typography.sm.size)
                            .text_color(if binding.query.is_empty() {
                                colors.muted_foreground
                            } else {
                                colors.foreground
                            })
                            .text_ellipsis()
                            .child(if binding.query.is_empty() {
                                i18n.text(Message::ShortcutTargetPlaceholder).into()
                            } else {
                                binding.query
                            }),
                    )
                }),
        )
        .child(
            div()
                .w(theme.metrics.shortcut_copy_width)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Switch::new(format!("shortcut-copy-{id}"))
                        .checked(binding.auto_copy)
                        .accessibility_label(i18n.text(Message::ShortcutAutoCopyColumn))
                        .on_click(move |checked, window, cx| {
                            copy_handler(
                                ShortcutBindingEvent::SetAutoCopy(id, *checked),
                                window,
                                cx,
                            )
                        }),
                ),
        )
        .child(
            div()
                .id(format!("shortcut-remove-{id}"))
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.semantic.radius.md)
                .cursor_pointer()
                .text_color(colors.muted_foreground)
                .hover(move |this| this.bg(colors.muted).text_color(colors.foreground))
                .on_click(move |_, window, cx| {
                    remove_handler(ShortcutBindingEvent::Remove(id), window, cx)
                })
                .child("×"),
        )
}

impl From<ThemeChoice> for SettingsChange {
    fn from(value: ThemeChoice) -> Self {
        Self::System(SystemChange::Theme(value))
    }
}

impl From<Locale> for SettingsChange {
    fn from(value: Locale) -> Self {
        Self::System(SystemChange::Locale(value))
    }
}

impl From<PanelSizePreset> for SettingsChange {
    fn from(value: PanelSizePreset) -> Self {
        Self::System(SystemChange::PanelSizePreset(value))
    }
}
