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
    theme::{AccentColor, AppTheme, PanelSizePreset},
    update::{UpdateStatus, is_newer_version},
};

use super::{SettingDefinition, SettingsChange, SettingsSection};
use crate::plugin::system::{SystemChange, SystemPlugin, ThemeChoice};

pub type BackHandler = Rc<dyn for<'a, 'b> Fn(&'a mut Window, &'b mut App)>;
pub type ChangeHandler = Rc<dyn for<'a, 'b> Fn(SettingsChange, &'a mut Window, &'b mut App)>;
pub type SectionHandler = Rc<dyn for<'a, 'b> Fn(SettingsSection, &'a mut Window, &'b mut App)>;
pub type ShortcutHandler =
    Rc<dyn for<'a, 'b> Fn(ShortcutBindingEvent, &'a mut Window, &'b mut App)>;
pub type CheckUpdatesHandler = Rc<dyn for<'a, 'b> Fn(&'a mut Window, &'b mut App)>;
pub type OpenReleaseHandler = Rc<dyn for<'a, 'b> Fn(String, &'a mut Window, &'b mut App)>;
pub type DownloadUpdateHandler =
    Rc<dyn for<'a, 'b> Fn(String, String, &'a mut Window, &'b mut App)>;
pub type InstallUpdateHandler =
    Rc<dyn for<'a, 'b> Fn(String, std::path::PathBuf, &'a mut Window, &'b mut App)>;

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
    update_status: &UpdateStatus,
    on_check_updates: CheckUpdatesHandler,
    on_open_release: OpenReleaseHandler,
    on_download_update: DownloadUpdateHandler,
    on_install_update: InstallUpdateHandler,
    translate_settings: Entity<crate::plugin::translate::view::TranslateSettingsPanel>,
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
    if !plugin_ids.contains(&PluginId::Translate) {
        plugin_ids.push(PluginId::Translate);
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
    let (content_title, content_description) = match selected_section {
        SettingsSection::General => (
            i18n.text(Message::SettingsGeneral),
            i18n.text(Message::SettingsGeneralDescription),
        ),
        SettingsSection::Shortcuts => (
            i18n.text(Message::ShortcutBindings),
            i18n.text(Message::ShortcutBindingsDescription),
        ),
        SettingsSection::Plugin(plugin) => {
            let (title, description, _) = plugin_presentation(plugin, i18n);
            (title, description)
        }
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
                .px(spacing.md)
                .pt(theme.metrics.header_top)
                .pb(spacing.md)
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
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .flex()
                .flex_col()
                .px(spacing.xl)
                .pt(theme.metrics.header_top)
                .gap(spacing.lg)
                .child(content_heading(content_title, content_description, theme))
                .child(
                    div()
                        .id(content_id)
                        .w_full()
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_y_scroll()
                        .pb(spacing.xl)
                        .when(selected_section == SettingsSection::General, |this| {
                            this.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(spacing.lg)
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
                                                    (
                                                        ThemeChoice::System,
                                                        i18n.text(Message::ThemeSystem),
                                                    ),
                                                    (
                                                        ThemeChoice::Light,
                                                        i18n.text(Message::ThemeLight),
                                                    ),
                                                    (
                                                        ThemeChoice::Dark,
                                                        i18n.text(Message::ThemeDark),
                                                    ),
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
                                            .child(accent_color_row(
                                                system.accent_color(),
                                                i18n,
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
                                                    (
                                                        Locale::ZhCn,
                                                        i18n.text(Message::LanguageChinese),
                                                    ),
                                                    (
                                                        Locale::EnUs,
                                                        i18n.text(Message::LanguageEnglish),
                                                    ),
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
                                                system.panel_size().preset
                                                    == PanelSizePreset::Custom,
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
                                                i18n.text(
                                                    Message::SettingsLaunchAtLoginDescription,
                                                ),
                                                system.launch_at_login(),
                                                SystemChange::LaunchAtLogin,
                                                &on_change,
                                                theme,
                                            )),
                                    )
                                    .when(
                                        cfg!(any(
                                            target_os = "macos",
                                            all(target_os = "windows", target_arch = "x86_64")
                                        )),
                                        |this| {
                                            this.child(update_panel(
                                                update_status,
                                                env!("CARGO_PKG_VERSION"),
                                                i18n,
                                                theme,
                                                &on_check_updates,
                                                &on_open_release,
                                                &on_download_update,
                                                &on_install_update,
                                            ))
                                        },
                                    ),
                            )
                        })
                        .when_some(selected_plugin, |this, plugin| {
                            this.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(spacing.lg)
                                    .children(
                                        plugin_settings
                                            .into_iter()
                                            .filter(|setting| setting.plugin == plugin)
                                            .map(|setting| {
                                                plugin_setting_row(setting, &on_change, theme)
                                            }),
                                    )
                                    .when(plugin == PluginId::Translate, |this| {
                                        this.child(translate_settings.clone().into_any_element())
                                    }),
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
                ),
        )
        .into_any_element()
}

fn update_panel(
    status: &UpdateStatus,
    current_version: &str,
    i18n: &I18n,
    theme: &AppTheme,
    on_check: &CheckUpdatesHandler,
    on_open_release: &OpenReleaseHandler,
    on_download_update: &DownloadUpdateHandler,
    on_install_update: &InstallUpdateHandler,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let releases = status.releases();
    let latest = releases.and_then(|releases| releases.first());
    let has_update =
        latest.is_some_and(|release| is_newer_version(&release.version, current_version));
    let state_text = match status {
        UpdateStatus::NotChecked => i18n.text(Message::SettingsNeverChecked).to_owned(),
        UpdateStatus::Checking => i18n.text(Message::SettingsCheckingUpdates).to_owned(),
        UpdateStatus::Failed(error) => {
            format!("{}: {error}", i18n.text(Message::SettingsCheckFailed))
        }
        UpdateStatus::Ready(releases) => match releases.first() {
            Some(_) if has_update => i18n.text(Message::SettingsUpdateAvailable).to_owned(),
            Some(_) => i18n.text(Message::SettingsUpToDate).to_owned(),
            None => i18n.text(Message::SettingsNoReleases).to_owned(),
        },
        UpdateStatus::Downloading { .. } => {
            i18n.text(Message::SettingsDownloadingUpdate).to_owned()
        }
        UpdateStatus::Downloaded { .. } => i18n.text(Message::SettingsUpdateDownloaded).to_owned(),
        UpdateStatus::Installing { .. } => i18n.text(Message::SettingsInstallingUpdate).to_owned(),
        UpdateStatus::DownloadFailed { error, .. } => {
            format!("{}: {error}", i18n.text(Message::SettingsDownloadFailed))
        }
        UpdateStatus::InstallFailed { error, .. } => {
            format!("{}: {error}", i18n.text(Message::SettingsInstallFailed))
        }
    };
    let is_busy = status.is_busy();
    let check_label = if matches!(status, UpdateStatus::Checking) {
        i18n.text(Message::SettingsCheckingUpdates)
    } else {
        i18n.text(Message::SettingsCheckForUpdates)
    };
    let check_callback = on_check.clone();

    let mut panel = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(spacing.sm)
        .p(spacing.md)
        .rounded(theme.semantic.radius.lg)
        .border_1()
        .border_color(colors.border)
        .bg(theme.materials.surface)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(spacing.md)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(spacing.xs)
                        .child(
                            div()
                                .text_size(typography.md.size)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(i18n.text(Message::SettingsUpdatesTitle)),
                        )
                        .child(
                            div()
                                .text_size(typography.xs.size)
                                .text_color(colors.muted_foreground)
                                .child(format!(
                                    "{}：v{current_version}",
                                    i18n.text(Message::SettingsCurrentVersion)
                                )),
                        ),
                )
                .child(
                    div()
                        .id("settings-check-updates")
                        .h(px(32.))
                        .px(spacing.md)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme.semantic.radius.md)
                        .border_1()
                        .border_color(colors.border)
                        .bg(theme.materials.window)
                        .text_size(typography.xs.size)
                        .when(!is_busy, |this| {
                            this.cursor_pointer()
                                .hover(move |this| this.bg(theme.materials.hover))
                                .on_click(move |_, window, cx| check_callback(window, cx))
                        })
                        .child(check_label),
                ),
        )
        .child(
            div()
                .text_size(typography.xs.size)
                .text_color(colors.muted_foreground)
                .child(state_text),
        );

    if let Some(release) =
        latest.filter(|_| has_update || matches!(status, UpdateStatus::InstallFailed { .. }))
    {
        enum UpdateButtonAction {
            OpenRelease(String),
            Download(String, String),
            Install(String, std::path::PathBuf),
            Disabled,
        }
        let (button_label, button_action) = match status {
            UpdateStatus::Downloading { version, .. } if version == &release.version => (
                i18n.text(Message::SettingsDownloadingUpdate),
                UpdateButtonAction::Disabled,
            ),
            UpdateStatus::Installing { version, .. } if version == &release.version => (
                i18n.text(Message::SettingsInstallingUpdate),
                UpdateButtonAction::Disabled,
            ),
            UpdateStatus::Downloaded {
                version,
                package_path,
                ..
            } if version == &release.version => (
                i18n.text(Message::SettingsInstallUpdate),
                UpdateButtonAction::Install(version.clone(), package_path.clone()),
            ),
            UpdateStatus::InstallFailed { version, .. } if version == &release.version => (
                i18n.text(Message::SettingsOpenRelease),
                UpdateButtonAction::OpenRelease(release.release_url.clone()),
            ),
            _ if release.download_url.is_some() => (
                i18n.text(Message::SettingsDownloadUpdate),
                UpdateButtonAction::Download(
                    release.version.clone(),
                    release.download_url.clone().unwrap_or_default(),
                ),
            ),
            _ => (
                i18n.text(Message::SettingsOpenRelease),
                UpdateButtonAction::OpenRelease(release.release_url.clone()),
            ),
        };
        let mut update_button = div()
            .id("settings-update-action")
            .h(px(28.))
            .px(spacing.sm)
            .flex()
            .items_center()
            .rounded(theme.semantic.radius.md)
            .bg(colors.accent)
            .text_color(colors.accent_foreground)
            .text_size(typography.xs.size)
            .child(button_label);
        update_button = match button_action {
            UpdateButtonAction::OpenRelease(url) => {
                let callback = on_open_release.clone();
                update_button
                    .cursor_pointer()
                    .on_click(move |_, window, cx| callback(url.clone(), window, cx))
            }
            UpdateButtonAction::Download(version, url) => {
                let callback = on_download_update.clone();
                update_button
                    .cursor_pointer()
                    .on_click(move |_, window, cx| {
                        callback(version.clone(), url.clone(), window, cx)
                    })
            }
            UpdateButtonAction::Install(version, package_path) => {
                let callback = on_install_update.clone();
                update_button
                    .cursor_pointer()
                    .on_click(move |_, window, cx| {
                        callback(version.clone(), package_path.clone(), window, cx)
                    })
            }
            UpdateButtonAction::Disabled => update_button,
        };

        panel = panel.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(spacing.md)
                .child(
                    div()
                        .text_size(typography.xs.size)
                        .text_color(colors.primary)
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!(
                            "v{}",
                            release
                                .version
                                .strip_prefix('v')
                                .unwrap_or(&release.version)
                        )),
                )
                .child(update_button),
        );

        let notes = release.notes.for_locale(i18n.locale());
        if !notes.is_empty() && has_update {
            panel = panel
                .child(div().h(theme.metrics.divider).bg(colors.border))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(spacing.xs)
                        .child(
                            div()
                                .text_size(typography.xs.size)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(i18n.text(Message::SettingsReleaseNotes)),
                        )
                        .child(
                            div()
                                .text_size(typography.xs.size)
                                .line_height(typography.xs.line_height)
                                .text_color(colors.muted_foreground)
                                .child(notes.to_owned()),
                        ),
                );
        }
    }

    let fallback_url = match status {
        UpdateStatus::Failed(_) => Some("https://github.com/JCLee777/jump1/releases".to_owned()),
        UpdateStatus::DownloadFailed { .. } => latest.map(|release| release.release_url.clone()),
        _ => None,
    };
    if let Some(url) = fallback_url {
        let callback = on_open_release.clone();
        panel = panel.child(
            div()
                .id("settings-open-releases")
                .text_size(typography.xs.size)
                .text_color(colors.primary)
                .cursor_pointer()
                .on_click(move |_, window, cx| callback(url.clone(), window, cx))
                .child(i18n.text(Message::SettingsOpenRelease)),
        );
    }

    panel
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
        PluginId::Markdown => (
            Message::MarkdownName,
            Message::MarkdownDescription,
            Glyph::Markdown,
        ),
        PluginId::Translate => (
            Message::TranslateName,
            Message::TranslateSettingsDescription,
            Glyph::Translate,
        ),
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
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let selected_control_background = theme.materials.selection_control;
    let hover_background = theme.materials.hover;

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
            selected_background
        } else {
            transparent_black()
        })
        .text_color(if active {
            selected_foreground
        } else {
            colors.foreground
        })
        .hover(move |this| {
            this.bg(if active {
                selected_background
            } else {
                hover_background
            })
        })
        .on_click(move |_, window, cx| callback(section, window, cx))
        .child(
            div()
                .size(theme.metrics.settings_sidebar_icon_box)
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.semantic.radius.md)
                .bg(if active {
                    selected_control_background
                } else {
                    theme.materials.control
                })
                .child(glyph(
                    icon,
                    theme.metrics.settings_sidebar_icon,
                    if active {
                        selected_foreground
                    } else {
                        colors.primary
                    },
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
        .min_h(theme.metrics.setting_row_height)
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

fn accent_color_row(
    selected: AccentColor,
    i18n: &I18n,
    on_change: &ChangeHandler,
    theme: &AppTheme,
) -> impl IntoElement {
    let colors = theme.semantic.colors;
    let spacing = theme.semantic.spacing;
    let typography = &theme.semantic.typography;
    let options = [
        AccentColor::Default,
        AccentColor::Blue,
        AccentColor::Purple,
        AccentColor::Pink,
        AccentColor::Orange,
        AccentColor::Green,
        AccentColor::Teal,
    ];

    div()
        .w_full()
        .min_h(theme.metrics.setting_row_height)
        .px(spacing.lg)
        .flex()
        .items_center()
        .justify_between()
        .gap(spacing.md)
        .child(
            div()
                .text_size(typography.sm.size)
                .line_height(typography.sm.line_height)
                .font_weight(FontWeight::MEDIUM)
                .child(i18n.text(Message::SettingsAccentColor)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(spacing.xs)
                .children(options.into_iter().map(|accent| {
                    let active = accent == selected;
                    let color = accent.color().unwrap_or(colors.muted_foreground);
                    let callback = on_change.clone();
                    div()
                        .id(format!("accent-color-{}", accent.key()))
                        .size(theme.metrics.accent_swatch_hit_size)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme.semantic.radius.full)
                        .border_1()
                        .border_color(if active { colors.accent } else { colors.border })
                        .cursor_pointer()
                        .on_click(move |_, window, cx| {
                            callback(
                                SettingsChange::System(SystemChange::AccentColor(accent)),
                                window,
                                cx,
                            )
                        })
                        .child(
                            div()
                                .size(theme.metrics.accent_swatch_size)
                                .rounded(theme.semantic.radius.full)
                                .bg(color),
                        )
                        .into_any_element()
                })),
        )
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
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let hover_background = theme.materials.hover;

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
                    selected_background
                } else {
                    transparent_black()
                })
                .text_color(if active {
                    selected_foreground
                } else {
                    colors.foreground
                })
                .hover(move |this| {
                    this.bg(if active {
                        selected_background
                    } else {
                        hover_background
                    })
                })
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
    let selected_background = theme.materials.selection;
    let selected_foreground = theme.materials.selection_foreground;
    let hover_background = theme.materials.hover;
    let plugin = setting.plugin;
    let key = setting.key;
    let visibility_setting = setting.options.len() == 2
        && setting.options.iter().any(|(value, _)| value == "show")
        && setting.options.iter().any(|(value, _)| value == "hide");
    let is_visible = setting.selected == "show";
    let accessibility_title = setting.title.clone();

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
                .when(visibility_setting, |this| {
                    let callback = on_change.clone();
                    this.flex().items_center().child(
                        Switch::new(format!("setting-{key}"))
                            .checked(is_visible)
                            .accessibility_label(accessibility_title)
                            .on_click(move |checked, window, cx| {
                                callback(
                                    SettingsChange::Plugin {
                                        plugin,
                                        key: key.to_owned(),
                                        value: if *checked { "show" } else { "hide" }.to_owned(),
                                    },
                                    window,
                                    cx,
                                )
                            }),
                    )
                })
                .when(!visibility_setting, |this| {
                    this.flex()
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
                                    selected_background
                                } else {
                                    transparent_black()
                                })
                                .text_color(if active {
                                    selected_foreground
                                } else {
                                    colors.foreground
                                })
                                .hover(move |this| {
                                    this.bg(if active {
                                        selected_background
                                    } else {
                                        hover_background
                                    })
                                })
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
                        }))
                }),
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
                .hover(move |this| this.bg(theme.materials.hover))
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
                .hover(move |this| this.bg(theme.materials.hover).text_color(colors.foreground))
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
