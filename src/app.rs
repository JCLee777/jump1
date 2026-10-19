use std::{rc::Rc, time::Duration};

use gpui_kit::{
    component::{
        input::{InputEvent, InputState},
        theme::{Theme, ThemeMode},
    },
    prelude::*,
    *,
};

use crate::{
    hotkey::{self, GlobalShortcuts, ShortcutAction},
    i18n::{I18n, Message},
    platform,
    plugin::{
        PluginCommand, PluginEvent, PluginId, PluginManager,
        clipboard::{
            ClipboardFilterMode, ClipboardFormat, ClipboardPreviewImageAsset,
            view::ClipboardPinnedView,
        },
        host_view,
        protocol::{CommandAction, HostEffect},
        settings::SettingsChange,
        shortcuts::ShortcutBindingEvent,
        system::{SystemChange, ThemeChoice},
    },
    state::Preferences,
    theme::{
        AccentColor, AppTheme, MAX_PANEL_HEIGHT, MAX_PANEL_WIDTH, MIN_PANEL_HEIGHT,
        MIN_PANEL_WIDTH, PanelSizePreset, apply_accent_color,
    },
    update::{UpdateStatus, is_newer_version},
};

pub(crate) struct Jump1Shell {
    input: Entity<InputState>,
    _input_subscription: Subscription,
    panel_width_input: Entity<InputState>,
    _panel_width_subscription: Subscription,
    panel_height_input: Entity<InputState>,
    _panel_height_subscription: Subscription,
    _key_subscription: Subscription,
    _appearance_subscription: Subscription,
    _activation_subscription: Subscription,
    global_shortcuts: Option<GlobalShortcuts>,
    results_scroll: ScrollHandle,
    clipboard_results_scroll: UniformListScrollHandle,
    i18n: I18n,
    plugins: PluginManager,
    update_prompt_focus: FocusHandle,
    previous_update_focus: Option<FocusHandle>,
    update_status: UpdateStatus,
    dismissed_update_prompt: Option<String>,
    selected: usize,
    status: Option<String>,
    recording_shortcut: Option<u64>,
    editing_shortcut_query: Option<u64>,
    shortcut_conflicts: Vec<u64>,
    previous_application: Option<platform::ApplicationTarget>,
}

impl Jump1Shell {
    pub(crate) fn new(
        previous_application: Option<platform::ApplicationTarget>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let preferences = Preferences::load();
        let i18n = I18n::new(preferences.locale);
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(i18n.text(Message::SearchPlaceholder))
        });
        let panel_size = preferences.panel_size;
        let panel_width_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(i18n.text(Message::SettingsPanelWidth))
                .default_value(panel_size.custom_width.to_string())
                .step(10.)
                .min(f64::from(MIN_PANEL_WIDTH))
                .max(f64::from(MAX_PANEL_WIDTH))
                .validate(|value, _| value.chars().all(|character| character.is_ascii_digit()))
        });
        let panel_height_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(i18n.text(Message::SettingsPanelHeight))
                .default_value(panel_size.custom_height.to_string())
                .step(10.)
                .min(f64::from(MIN_PANEL_HEIGHT))
                .max(f64::from(MAX_PANEL_HEIGHT))
                .validate(|value, _| value.chars().all(|character| character.is_ascii_digit()))
        });
        let update_prompt_focus = cx.focus_handle();
        let input_subscription =
            cx.subscribe(&input, |this, input, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    let query = input.read(cx).value().to_string();
                    if let Some(id) = this.editing_shortcut_query {
                        this.plugins.shortcuts_mut().set_query(id, query);
                    } else {
                        this.plugins.set_query(query);
                    }
                    this.selected = 0;
                    this.scroll_to_result(0);
                    if this.plugins.active() == PluginId::Clipboard
                        && this.plugins.clipboard().preview_open()
                    {
                        this.plugins.open_clipboard_preview(0);
                    }
                    this.status = None;
                    cx.notify();
                }
                _ => {}
            });
        let panel_width_subscription = cx.subscribe_in(
            &panel_width_input,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(width) = input.read(cx).value().to_string().parse::<u32>() else {
                    return;
                };
                if !(MIN_PANEL_WIDTH..=MAX_PANEL_WIDTH).contains(&width) {
                    return;
                }
                let panel_size = this.plugins.system().panel_size();
                if panel_size.preset != PanelSizePreset::Custom || panel_size.custom_width == width
                {
                    return;
                }
                this.handle_plugin_event(
                    PluginEvent::Settings(SettingsChange::System(SystemChange::CustomPanelSize {
                        width,
                        height: panel_size.custom_height,
                    })),
                    window,
                    cx,
                );
            },
        );
        let panel_height_subscription = cx.subscribe_in(
            &panel_height_input,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(height) = input.read(cx).value().to_string().parse::<u32>() else {
                    return;
                };
                if !(MIN_PANEL_HEIGHT..=MAX_PANEL_HEIGHT).contains(&height) {
                    return;
                }
                let panel_size = this.plugins.system().panel_size();
                if panel_size.preset != PanelSizePreset::Custom
                    || panel_size.custom_height == height
                {
                    return;
                }
                this.handle_plugin_event(
                    PluginEvent::Settings(SettingsChange::System(SystemChange::CustomPanelSize {
                        width: panel_size.custom_width,
                        height,
                    })),
                    window,
                    cx,
                );
            },
        );

        let input_focus = input.focus_handle(cx);
        let shell = cx.entity().downgrade();
        let key_subscription = cx.intercept_keystrokes(move |event, window, cx| {
            let handled = shell
                .update(cx, |shell, cx| {
                    if shell.update_prompt_is_visible() {
                        if event.keystroke.key == "escape"
                            && let UpdateStatus::Downloaded { version, .. } = &shell.update_status
                        {
                            shell.dismissed_update_prompt = Some(version.clone());
                            cx.notify();
                        }
                        return true;
                    }
                    if shell.plugins.active() == PluginId::Clipboard
                        && event.keystroke.key.eq_ignore_ascii_case("d")
                        && event.keystroke.modifiers.control
                    {
                        let Some(entry) = shell
                            .plugins
                            .clipboard()
                            .current_entries()
                            .get(shell.selected)
                            .map(|entry| (*entry).clone())
                        else {
                            return true;
                        };
                        let i18n = shell.i18n.clone();
                        let display = window.display(cx).or_else(|| cx.primary_display());
                        let display_id = display.as_ref().map(|display| display.id());
                        let window_size = if entry.format == ClipboardFormat::Image {
                            display
                                .as_ref()
                                .map(|display| {
                                    let bounds = display.visible_bounds();
                                    size(bounds.size.width * 0.5, bounds.size.height * 0.5)
                                })
                                .unwrap_or_else(|| size(px(640.), px(480.)))
                        } else {
                            size(px(640.), px(480.))
                        };
                        let window_bounds = Bounds::centered(display_id, window_size, cx);
                        let options = WindowOptions {
                            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                            display_id,
                            titlebar: Some(TitlebarOptions {
                                appears_transparent: true,
                                ..Default::default()
                            }),
                            app_owns_titlebar_drag: true,
                            is_resizable: true,
                            is_minimizable: true,
                            window_min_size: Some(size(px(360.), px(240.))),
                            window_decorations: Some(WindowDecorations::Client),
                            window_background: WindowBackgroundAppearance::Transparent,
                            ..Default::default()
                        };
                        if let Err(error) = gpui_kit::open_window(options, cx, move |window, cx| {
                            platform::set_window_always_on_top(window);
                            cx.new(|_| ClipboardPinnedView::new(entry, i18n))
                        }) {
                            eprintln!("failed to open pinned clipboard window: {error}");
                        }
                        return true;
                    }
                    if !input_focus.is_focused(window) {
                        return false;
                    }
                    if let Some(id) = shell.recording_shortcut {
                        if event.keystroke.key == "escape" {
                            shell.recording_shortcut = None;
                            cx.notify();
                            return true;
                        }
                        if let Some(shortcut) = hotkey::recorded_shortcut(&event.keystroke) {
                            if shell.plugins.shortcuts_mut().set_shortcut(id, shortcut) {
                                shell.recording_shortcut = None;
                                shell.editing_shortcut_query = Some(id);
                                shell.reload_shortcut_bindings();
                                let query = shell
                                    .plugins
                                    .shortcuts()
                                    .binding(id)
                                    .map(|binding| binding.query.clone())
                                    .unwrap_or_default();
                                shell.input.update(cx, |input, cx| {
                                    input.set_value(query, window, cx);
                                    input.set_placeholder(
                                        shell.i18n.text(Message::ShortcutTargetPlaceholder),
                                        window,
                                        cx,
                                    );
                                });
                            }
                            cx.notify();
                            return true;
                        }
                    }
                    if shell.editing_shortcut_query.is_some() && event.keystroke.key == "enter" {
                        shell.editing_shortcut_query = None;
                        cx.notify();
                        return true;
                    }
                    if shell.editing_shortcut_query.is_some() && event.keystroke.key == "escape" {
                        shell.editing_shortcut_query = None;
                        cx.notify();
                        return true;
                    }
                    match event.keystroke.key.as_str() {
                        "tab" if shell.plugins.active() == PluginId::Clipboard => {
                            let next_filter = match shell.plugins.clipboard().filter_mode() {
                                ClipboardFilterMode::All => ClipboardFilterMode::Favorites,
                                ClipboardFilterMode::Favorites => ClipboardFilterMode::All,
                            };
                            shell.handle_plugin_event(
                                PluginEvent::ClipboardFilter(next_filter),
                                window,
                                cx,
                            );
                            true
                        }
                        "up" => {
                            shell.move_selection(-1, cx);
                            true
                        }
                        "left"
                            if shell.plugins.active() == PluginId::Launcher
                                && shell.plugins.launcher().query().trim().is_empty() =>
                        {
                            shell.move_selection(-1, cx);
                            true
                        }
                        "down" => {
                            shell.move_selection(1, cx);
                            true
                        }
                        "right" if shell.plugins.active() == PluginId::Clipboard => {
                            if shell.current_count() == 0 {
                                false
                            } else {
                                shell.plugins.open_clipboard_preview(shell.selected);
                                cx.notify();
                                true
                            }
                        }
                        "right"
                            if shell.plugins.active() == PluginId::Launcher
                                && shell.plugins.launcher().query().trim().is_empty() =>
                        {
                            shell.move_selection(1, cx);
                            true
                        }
                        "left"
                            if shell.plugins.active() == PluginId::Clipboard
                                && shell.plugins.clipboard().preview_open() =>
                        {
                            shell.plugins.close_clipboard_preview();
                            cx.notify();
                            true
                        }
                        "enter" => {
                            shell.activate_selected(window, cx);
                            true
                        }
                        "escape" => {
                            if shell.plugins.active() == PluginId::Clipboard
                                && shell.plugins.clipboard().preview_open()
                            {
                                shell.plugins.close_clipboard_preview();
                                cx.notify();
                            } else if shell.plugins.active() != PluginId::Launcher {
                                shell.show_launcher(window, cx);
                            } else if !shell.plugins.launcher().query().is_empty() {
                                shell.reset_input(
                                    shell.plugins.input_placeholder(&shell.i18n),
                                    window,
                                    cx,
                                );
                            } else {
                                shell.recording_shortcut = None;
                                shell.editing_shortcut_query = None;
                                shell.hide_panel(window, cx);
                            }
                            true
                        }
                        _ => false,
                    }
                })
                .unwrap_or(false);
            if handled {
                cx.stop_propagation();
            }
        });
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            match this.plugins.system().theme() {
                ThemeChoice::System => Theme::sync_system_appearance(Some(window), cx),
                ThemeChoice::Light => Theme::change(ThemeMode::Light, None, cx),
                ThemeChoice::Dark => Theme::change(ThemeMode::Dark, None, cx),
            }
            let accent = this.plugins.system().accent_color();
            if accent != AccentColor::Default {
                apply_accent_color(accent, cx);
            }
            cx.notify();
        });
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.hide_panel(window, cx);
            }
        });

        let event_shell = cx.entity().downgrade();
        let plugin_event_handler = Rc::new(move |event, window: &mut Window, cx: &mut App| {
            let _ = event_shell.update(cx, |shell, cx| {
                shell.handle_plugin_event(event, window, cx);
            });
        });
        let mut plugins = PluginManager::load(preferences, plugin_event_handler, window, cx);
        let global_shortcuts = GlobalShortcuts::register(plugins.shortcuts().bindings()).ok();
        if let Some(shortcuts) = global_shortcuts.as_ref() {
            let launcher_id = shortcuts.launcher_id();
            let shortcut_actions = shortcuts.actions();
            let window_handle = window.window_handle();
            let shortcut_shell = cx.entity().downgrade();
            let timer = cx.background_executor().clone();
            cx.spawn(async move |_, cx| {
                loop {
                    timer.timer(Duration::from_millis(50)).await;
                    if let Some(action) = hotkey::take_action(launcher_id, &shortcut_actions) {
                        let alive = window_handle
                            .update(cx, |_, window, cx| {
                                shortcut_shell
                                    .update(cx, |shell, cx| {
                                        if shell.update_prompt_is_visible() {
                                            return;
                                        }
                                        match action {
                                            ShortcutAction::ToggleLauncher => {
                                                shell.toggle_panel(window, cx)
                                            }
                                            ShortcutAction::RunBinding(id) => {
                                                shell.run_shortcut_binding(id, window, cx)
                                            }
                                        }
                                    })
                                    .is_ok()
                            })
                            .unwrap_or(false);
                        if !alive {
                            break;
                        }
                    }
                }
            })
            .detach();
        }
        let shortcut_conflicts = global_shortcuts
            .as_ref()
            .map(|shortcuts| shortcuts.conflicts().to_vec())
            .unwrap_or_default();

        window.focus(&input.focus_handle(cx), cx);

        if let Some(item) = cx.read_from_clipboard() {
            plugins.capture_clipboard(item);
        }

        let scan = cx
            .background_executor()
            .spawn(async { platform::discover_apps() });
        cx.spawn(async move |this, cx| {
            let apps = scan.await;
            let _ = this.update(cx, |shell, cx| {
                shell.plugins.set_apps(apps);
                cx.notify();
            });
        })
        .detach();

        let timer = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            loop {
                timer.timer(Duration::from_millis(700)).await;
                let Ok(()) = this.update(cx, |shell, cx| {
                    if let Some(target) = platform::frontmost_application() {
                        shell.previous_application = Some(target);
                    }
                    let changed = cx
                        .read_from_clipboard()
                        .is_some_and(|item| shell.plugins.capture_clipboard(item));
                    if changed {
                        shell.selected = 0;
                        shell.scroll_to_result(0);
                        shell.plugins.update_clipboard_preview(0);
                        cx.notify();
                    }
                }) else {
                    break;
                };
            }
        })
        .detach();

        #[cfg(any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64")
        ))]
        {
            let timer = cx.background_executor().clone();
            cx.spawn(async move |this, cx| {
                timer.timer(Duration::from_secs(5)).await;
                loop {
                    if this
                        .update(cx, |shell, cx| shell.check_for_updates(cx))
                        .is_err()
                    {
                        break;
                    }
                    timer.timer(Duration::from_secs(12 * 60 * 60)).await;
                }
            })
            .detach();
        }

        Self {
            input,
            _input_subscription: input_subscription,
            panel_width_input,
            _panel_width_subscription: panel_width_subscription,
            panel_height_input,
            _panel_height_subscription: panel_height_subscription,
            _key_subscription: key_subscription,
            _appearance_subscription: appearance_subscription,
            _activation_subscription: activation_subscription,
            global_shortcuts,
            results_scroll: ScrollHandle::new(),
            clipboard_results_scroll: UniformListScrollHandle::new(),
            i18n,
            plugins,
            update_prompt_focus,
            previous_update_focus: None,
            update_status: UpdateStatus::default(),
            dismissed_update_prompt: None,
            selected: 0,
            status: None,
            recording_shortcut: None,
            editing_shortcut_query: None,
            shortcut_conflicts,
            previous_application,
        }
    }

    fn current_count(&self) -> usize {
        self.plugins.current_count()
    }

    fn update_prompt_is_visible(&self) -> bool {
        matches!(
            &self.update_status,
            UpdateStatus::Downloaded { version, .. }
                if self.dismissed_update_prompt.as_deref() != Some(version.as_str())
        )
    }

    fn scroll_to_result(&self, index: usize) {
        if self.plugins.active() == PluginId::Clipboard {
            self.clipboard_results_scroll
                .scroll_to_item(index, ScrollStrategy::Nearest);
        } else {
            self.results_scroll.scroll_to_item(index);
        }
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.current_count();
        if count == 0 {
            return;
        }
        self.selected = self.selected.saturating_add_signed(delta).min(count - 1);
        self.scroll_to_result(self.selected);
        if self.plugins.active() == PluginId::Clipboard {
            self.plugins.update_clipboard_preview(self.selected);
        }
        cx.notify();
    }

    fn activate_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.plugins.activate(self.selected) {
            PluginCommand::Action(action) => {
                self.activate_command(action, window, cx);
            }
            PluginCommand::PasteClipboard(item) => self.paste_clipboard_item(item, window, cx),
            PluginCommand::None => {}
        }
    }

    fn handle_plugin_event(
        &mut self,
        event: PluginEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PluginEvent::Action(action) => {
                self.activate_command(action, window, cx);
            }
            PluginEvent::Back => self.show_launcher(window, cx),
            PluginEvent::PasteClipboard(index) => {
                self.selected = index;
                if let PluginCommand::PasteClipboard(item) = self.plugins.activate(index) {
                    self.paste_clipboard_item(item, window, cx);
                }
            }
            PluginEvent::ClipboardSort(mode) => {
                self.plugins.set_clipboard_sort(mode);
                self.selected = 0;
                self.scroll_to_result(0);
                self.plugins.update_clipboard_preview(0);
                self.status = None;
                cx.notify();
            }
            PluginEvent::ClipboardFilter(mode) => {
                self.plugins.set_clipboard_filter(mode);
                self.selected = 0;
                self.scroll_to_result(0);
                if self.plugins.clipboard().preview_open() {
                    self.plugins.open_clipboard_preview(0);
                }
                self.status = None;
                cx.notify();
            }
            PluginEvent::ToggleClipboardFavorite(index) => {
                if self.plugins.toggle_clipboard_favorite(index) {
                    let preview_open = self.plugins.clipboard().preview_open();
                    self.selected = index.min(self.current_count().saturating_sub(1));
                    self.scroll_to_result(self.selected);
                    if self.current_count() == 0 {
                        self.plugins.close_clipboard_preview();
                    } else if preview_open {
                        self.plugins.update_clipboard_preview(self.selected);
                    }
                    self.status = None;
                    cx.notify();
                }
            }
            PluginEvent::ClearClipboard => {
                self.plugins
                    .clear_clipboard_history(cx.read_from_clipboard());
                self.selected = 0;
                self.status = Some(self.i18n.text(Message::ClipboardCleared).into());
                cx.notify();
            }
            PluginEvent::WriteClipboard(text) => self.copy_clipboard_text(text, cx),
            PluginEvent::ImportClipboard(plugin) => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.plugins.import_clipboard(plugin, text, window, cx);
                }
            }
            PluginEvent::Settings(change) => {
                for effect in self.plugins.apply_setting(&change, window, cx) {
                    self.apply_host_effect(effect, window, cx);
                }
                let placeholder = self.plugins.input_placeholder(&self.i18n);
                self.input.update(cx, |input, cx| {
                    input.set_placeholder(placeholder, window, cx);
                });
                let width_placeholder = self.i18n.text(Message::SettingsPanelWidth);
                self.panel_width_input.update(cx, |input, cx| {
                    input.set_placeholder(width_placeholder, window, cx);
                });
                let height_placeholder = self.i18n.text(Message::SettingsPanelHeight);
                self.panel_height_input.update(cx, |input, cx| {
                    input.set_placeholder(height_placeholder, window, cx);
                });
                cx.notify();
            }
            PluginEvent::SettingsSection(section) => {
                self.recording_shortcut = None;
                self.editing_shortcut_query = None;
                self.plugins.select_settings_section(section);
                cx.notify();
            }
            PluginEvent::CheckForUpdates => {
                #[cfg(any(
                    all(target_os = "macos", target_arch = "aarch64"),
                    all(target_os = "windows", target_arch = "x86_64")
                ))]
                self.check_for_updates(cx);
            }
            PluginEvent::DownloadUpdate { version, url } => {
                #[cfg(any(
                    all(target_os = "macos", target_arch = "aarch64"),
                    all(target_os = "windows", target_arch = "x86_64")
                ))]
                self.download_update(version, url, cx);
                #[cfg(not(any(
                    all(target_os = "macos", target_arch = "aarch64"),
                    all(target_os = "windows", target_arch = "x86_64")
                )))]
                let _ = (version, url);
            }
            PluginEvent::InstallUpdate {
                version,
                package_path,
            } => {
                #[cfg(any(
                    all(target_os = "macos", target_arch = "aarch64"),
                    all(target_os = "windows", target_arch = "x86_64")
                ))]
                self.install_update(version, package_path, cx);
                #[cfg(not(any(
                    all(target_os = "macos", target_arch = "aarch64"),
                    all(target_os = "windows", target_arch = "x86_64")
                )))]
                let _ = (version, package_path);
            }
            PluginEvent::ShortcutBinding(event) => {
                self.handle_shortcut_binding_event(event, window, cx);
            }
        }
    }

    #[cfg(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    ))]
    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.update_status.is_busy() {
            return;
        }
        self.update_status = UpdateStatus::Checking;
        cx.notify();

        let request = cx
            .background_executor()
            .spawn(async { crate::update::fetch_latest_release() });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |shell, cx| match result {
                Ok(releases) => {
                    let update = releases
                        .first()
                        .filter(|release| {
                            is_newer_version(&release.version, env!("CARGO_PKG_VERSION"))
                        })
                        .and_then(|release| {
                            release
                                .download_url
                                .as_ref()
                                .map(|url| (release.version.clone(), url.clone()))
                        });
                    shell.update_status = UpdateStatus::Ready(releases);
                    if let Some((version, url)) = update {
                        shell.download_update(version, url, cx);
                    } else {
                        cx.notify();
                    }
                }
                Err(error) => {
                    shell.update_status = UpdateStatus::Failed(error);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    #[cfg(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    ))]
    fn download_update(&mut self, version: String, url: String, cx: &mut Context<Self>) {
        if self.update_status.is_busy() {
            return;
        }
        let releases = self.update_status.releases().unwrap_or_default().to_vec();
        let Some(release) = releases.iter().find(|release| release.version == version) else {
            return;
        };
        if release.download_url.as_deref() != Some(url.as_str()) {
            return;
        }

        self.update_status = UpdateStatus::Downloading {
            releases: releases.clone(),
            version: version.clone(),
        };
        cx.notify();

        let download_version = version.clone();
        let request = cx
            .background_executor()
            .spawn(async move { crate::update::download_update(&download_version, &url) });
        cx.spawn(async move |this, cx| {
            let status = match request.await {
                Ok(package_path) => UpdateStatus::Downloaded {
                    releases,
                    version,
                    package_path,
                },
                Err(error) => UpdateStatus::DownloadFailed { releases, error },
            };
            let _ = this.update(cx, |shell, cx| {
                shell.update_status = status;
                cx.notify();
            });
        })
        .detach();
    }

    #[cfg(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    ))]
    fn install_update(
        &mut self,
        version: String,
        package_path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        if self.update_status.is_busy() {
            return;
        }
        let (releases, expected_version, expected_path) = match &self.update_status {
            UpdateStatus::Downloaded {
                releases,
                version,
                package_path,
            }
            | UpdateStatus::InstallFailed {
                releases,
                version,
                package_path,
                ..
            } => (releases.clone(), version.clone(), package_path.clone()),
            _ => return,
        };
        if version != expected_version || package_path != expected_path {
            return;
        }

        match crate::update::install_update_and_restart(&package_path, &version) {
            Ok(()) => {
                self.update_status = UpdateStatus::Installing { releases, version };
                cx.quit();
            }
            Err(error) => {
                self.update_status = UpdateStatus::InstallFailed {
                    releases,
                    version,
                    package_path,
                    error,
                };
                cx.notify();
            }
        }
    }

    fn activate_command(
        &mut self,
        action: crate::plugin::launcher::CommandAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let CommandAction::OpenPlugin(plugin) = &action {
            self.plugins.record_launcher_entry_usage(&action);
            self.show_plugin(*plugin, window, cx);
            return true;
        }
        let result = match &action {
            CommandAction::Application(app) => platform::launch_app(app),
            CommandAction::Path(path) => platform::open_path(path),
            CommandAction::Url(url) => platform::open_url(url),
            CommandAction::OpenPlugin(_) => unreachable!(),
        };
        let succeeded = result.is_ok();
        if succeeded {
            match &action {
                CommandAction::Application(app) => self.plugins.record_application_usage(app),
                CommandAction::Path(_) => self.plugins.record_launcher_entry_usage(&action),
                CommandAction::OpenPlugin(_) | CommandAction::Url(_) => {}
            }
        }
        self.status = Some(match result {
            Ok(()) => self.i18n.text(Message::OpenSuccess).into(),
            Err(error) => self.i18n.open_error(&error),
        });
        succeeded
    }

    fn show_launcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_plugin(PluginId::Launcher, window, cx);
    }

    fn toggle_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_window_active() {
            self.hide_panel(window, cx);
            return;
        }
        if let Some(target) = platform::frontmost_application() {
            self.previous_application = Some(target);
        }
        self.recording_shortcut = None;
        self.editing_shortcut_query = None;
        self.activate_panel_window(window, cx);
        self.focus_active_plugin(window, cx);
    }

    fn activate_panel_window(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !window.is_window_active() {
            platform::center_window_on_pointer(window);
        }
        platform::show_panel_window(window);
        cx.activate(true);
        window.activate_window();
    }

    fn run_shortcut_binding(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some((query, auto_copy)) = self
            .plugins
            .shortcuts()
            .binding(id)
            .map(|binding| (binding.query.trim().to_owned(), binding.auto_copy))
            .filter(|(query, _)| !query.is_empty())
        else {
            return;
        };
        if auto_copy && platform::trigger_copy().is_ok() {
            let timer = cx.background_executor().clone();
            let window_handle = window.window_handle();
            let shell = cx.entity().downgrade();
            cx.spawn(async move |_, cx| {
                timer.timer(Duration::from_millis(180)).await;
                let _ = window_handle.update(cx, |_, window, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        shell.execute_shortcut_query(query, window, cx);
                    });
                });
            })
            .detach();
            return;
        }
        self.execute_shortcut_query(query, window, cx);
    }

    fn execute_shortcut_query(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.plugins.open(PluginId::Launcher, Some(window), cx);
        self.plugins.clear_queries();
        self.plugins.set_query(query.clone());
        self.selected = 0;
        self.status = None;
        self.input.update(cx, |input, cx| {
            input.set_value(query, window, cx);
            input.set_placeholder(self.plugins.input_placeholder(&self.i18n), window, cx);
        });

        match self.plugins.activate(0) {
            PluginCommand::Action(action @ CommandAction::OpenPlugin(_)) => {
                self.activate_panel_window(window, cx);
                self.activate_command(action, window, cx);
            }
            PluginCommand::Action(action) => {
                if self.activate_command(action, window, cx) {
                    self.reset_input(self.plugins.input_placeholder(&self.i18n), window, cx);
                } else {
                    self.activate_panel_window(window, cx);
                    window.focus(&self.input.focus_handle(cx), cx);
                    cx.notify();
                }
            }
            PluginCommand::PasteClipboard(_) | PluginCommand::None => {
                self.activate_panel_window(window, cx);
                window.focus(&self.input.focus_handle(cx), cx);
                cx.notify();
            }
        }
    }

    fn handle_shortcut_binding_event(
        &mut self,
        event: ShortcutBindingEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            ShortcutBindingEvent::Add => {
                let id = self.plugins.shortcuts_mut().add_binding();
                self.recording_shortcut = Some(id);
                self.editing_shortcut_query = None;
            }
            ShortcutBindingEvent::Record(id) => {
                self.recording_shortcut = Some(id);
                self.editing_shortcut_query = None;
            }
            ShortcutBindingEvent::EditQuery(id) => {
                let Some(query) = self
                    .plugins
                    .shortcuts()
                    .binding(id)
                    .map(|binding| binding.query.clone())
                else {
                    return;
                };
                self.recording_shortcut = None;
                self.editing_shortcut_query = Some(id);
                self.input.update(cx, |input, cx| {
                    input.set_value(query, window, cx);
                    input.set_placeholder(
                        self.i18n.text(Message::ShortcutTargetPlaceholder),
                        window,
                        cx,
                    );
                });
                window.focus(&self.input.focus_handle(cx), cx);
            }
            ShortcutBindingEvent::SetAutoCopy(id, auto_copy) => {
                self.plugins.shortcuts_mut().set_auto_copy(id, auto_copy);
            }
            ShortcutBindingEvent::Remove(id) => {
                self.plugins.shortcuts_mut().remove_binding(id);
                if self.recording_shortcut == Some(id) {
                    self.recording_shortcut = None;
                }
                if self.editing_shortcut_query == Some(id) {
                    self.editing_shortcut_query = None;
                }
                self.reload_shortcut_bindings();
            }
        }
        cx.notify();
    }

    fn reload_shortcut_bindings(&mut self) {
        let bindings = self.plugins.shortcuts().bindings().to_vec();
        if let Some(shortcuts) = self.global_shortcuts.as_mut() {
            shortcuts.set_bindings(&bindings);
            self.shortcut_conflicts = shortcuts.conflicts().to_vec();
        }
    }

    fn show_plugin(&mut self, plugin: PluginId, window: &mut Window, cx: &mut Context<Self>) {
        self.recording_shortcut = None;
        self.editing_shortcut_query = None;
        self.plugins.open(plugin, Some(window), cx);
        let clipboard_text = cx.read_from_clipboard().and_then(|item| item.text());
        self.plugins
            .prepare_open(plugin, clipboard_text, window, cx);
        self.reset_input(self.plugins.input_placeholder(&self.i18n), window, cx);
    }

    fn apply_host_effect(
        &mut self,
        effect: HostEffect,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match effect {
            HostEffect::ApplyTheme(theme) => {
                match theme {
                    ThemeChoice::System => Theme::sync_system_appearance(Some(window), cx),
                    ThemeChoice::Light => Theme::change(ThemeMode::Light, None, cx),
                    ThemeChoice::Dark => Theme::change(ThemeMode::Dark, None, cx),
                }
                let accent = self.plugins.system().accent_color();
                if accent != AccentColor::Default {
                    apply_accent_color(accent, cx);
                }
            }
            HostEffect::ApplyAccentColor(accent) => apply_accent_color(accent, cx),
            HostEffect::ApplyLocale(locale) => self.i18n.set_locale(locale),
            HostEffect::ResizePanel(panel_size) => {
                window.resize(crate::theme::window_size(panel_size));
            }
            HostEffect::LaunchAtLogin(result) => {
                self.status = Some(match result {
                    Ok(true) => self.i18n.text(Message::LaunchAtLoginEnabled).into(),
                    Ok(false) => self.i18n.text(Message::LaunchAtLoginDisabled).into(),
                    Err(error) => self.i18n.launch_at_login_error(&error),
                });
            }
        }
    }

    fn reset_input(
        &mut self,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.plugins.clear_queries();
        self.selected = 0;
        self.status = None;
        self.scroll_to_result(0);
        self.input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.set_placeholder(placeholder, window, cx);
        });
        self.focus_active_plugin(window, cx);
        cx.notify();
    }

    fn focus_active_plugin(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(focus) = self.plugins.active_focus_handle(cx) {
            window.focus(&focus, cx);
        } else {
            window.focus(&self.input.focus_handle(cx), cx);
        }
    }

    fn copy_clipboard_text(&mut self, text: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.status = Some(self.i18n.text(Message::ClipboardCopied).into());
        cx.notify();
    }

    fn paste_clipboard_item(
        &mut self,
        selection: crate::plugin::clipboard::ClipboardSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut clipboard_written = false;
        if let Some(native) = &selection.native {
            match platform::write_native_clipboard(native) {
                Ok(()) => clipboard_written = true,
                Err(error) => eprintln!("failed to write native clipboard: {error}"),
            }
        }
        if !clipboard_written {
            #[cfg(target_os = "windows")]
            {
                let file_paths = selection
                    .item
                    .entries()
                    .iter()
                    .find_map(|entry| match entry {
                        ClipboardEntry::ExternalPaths(paths) if !paths.paths().is_empty() => {
                            Some(paths.paths().to_vec())
                        }
                        _ => None,
                    });
                if let Some(paths) = file_paths {
                    if let Err(error) = platform::write_file_paths_to_clipboard(window, &paths) {
                        eprintln!("failed to write file paths to clipboard: {error}");
                        return;
                    }
                } else {
                    cx.write_to_clipboard(selection.item.clone());
                    if let Some(image) =
                        selection
                            .item
                            .entries()
                            .iter()
                            .find_map(|entry| match entry {
                                ClipboardEntry::Image(image) => Some(image),
                                _ => None,
                            })
                    {
                        if let Err(error) = platform::write_image_to_clipboard(window, image) {
                            eprintln!("failed to add a Windows bitmap clipboard format: {error}");
                        }
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            cx.write_to_clipboard(selection.item.clone());
        }
        self.plugins.ignore_clipboard_item(Some(selection.item));
        let target_application = self.previous_application;
        #[cfg(target_os = "windows")]
        if let Some(target) = target_application {
            if let Err(error) = platform::activate_application(target) {
                eprintln!("failed to restore previous application: {error}");
            }
        }
        self.hide_panel(window, cx);
        #[cfg(not(target_os = "windows"))]
        if let Some(target) = target_application {
            if let Err(error) = platform::activate_application(target) {
                eprintln!("failed to restore previous application: {error}");
            }
        }
        let timer = cx.background_executor().clone();
        cx.spawn(async move |_, _| {
            if let Some(target) = target_application {
                // Restore the exact window that owned focus before the panel opened.
                let mut restored = false;
                for _ in 0..50 {
                    if platform::is_frontmost_application(target) {
                        restored = true;
                        break;
                    }
                    timer.timer(Duration::from_millis(10)).await;
                }
                if !restored {
                    eprintln!("paste target did not become the frontmost application");
                    return;
                }
            } else {
                timer.timer(Duration::from_millis(80)).await;
            }
            if let Err(error) = platform::trigger_paste() {
                eprintln!("failed to trigger paste: {error}");
            }
        })
        .detach();
    }

    fn hide_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.plugins.active() == PluginId::Clipboard {
            self.plugins.leave_clipboard_page(Some(window), cx);
        }
        #[cfg(not(target_os = "macos"))]
        cx.hide();
        platform::hide_panel_window(window);
    }
}

impl Render for Jump1Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clipboard_preview_image = if self.plugins.active() == PluginId::Clipboard
            && self.plugins.clipboard().preview_open()
        {
            self.plugins
                .clipboard()
                .preview_source()
                .and_then(|source| {
                    self.plugins.track_clipboard_preview_asset(source.clone());
                    window
                        .use_asset::<AssetLogger<ClipboardPreviewImageAsset>>(&source, cx)
                        .and_then(Result::ok)
                })
        } else {
            None
        };

        let theme = AppTheme::resolve(cx);
        let shell = cx.entity();
        let event_shell = shell.clone();
        let on_event: host_view::EventHandler = Rc::new(move |event, window, cx| {
            let _ = event_shell.update(cx, |shell, cx| {
                shell.handle_plugin_event(event, window, cx);
            });
        });
        let content = host_view::render(
            &self.plugins,
            &self.input,
            &self.panel_width_input,
            &self.panel_height_input,
            &self.results_scroll,
            &self.clipboard_results_scroll,
            clipboard_preview_image,
            self.selected,
            self.status.clone(),
            &self.i18n,
            &theme,
            on_event.clone(),
            self.recording_shortcut,
            self.editing_shortcut_query,
            self.shortcut_conflicts.clone(),
            &self.update_status,
        );
        let update_prompt = match &self.update_status {
            UpdateStatus::Downloaded {
                releases,
                version,
                package_path,
            } if self.dismissed_update_prompt.as_deref() != Some(version.as_str()) => {
                let notes = releases
                    .iter()
                    .find(|release| release.version == *version)
                    .map(|release| release.notes.for_locale(self.i18n.locale()).to_owned())
                    .unwrap_or_default();
                Some((version.clone(), package_path.clone(), notes))
            }
            _ => None,
        };
        if update_prompt.is_some() {
            if !self.update_prompt_focus.contains_focused(window, cx) {
                self.previous_update_focus = window.focused(cx);
                window.focus(&self.update_prompt_focus, cx);
            }
        } else if self.update_prompt_focus.is_focused(window) {
            if let Some(previous_focus) = self.previous_update_focus.take() {
                window.focus(&previous_focus, cx);
            } else {
                window.focus(&self.input.focus_handle(cx), cx);
            }
        }
        let content = if let Some((version, package_path, release_notes)) = update_prompt {
            let dismiss_shell = shell.downgrade();
            let dismiss_version = version.clone();
            let on_install = on_event.clone();
            let version_label =
                format!("v{}", version.strip_prefix('v').unwrap_or(version.as_str()));
            let colors = theme.semantic.colors;
            let spacing = theme.semantic.spacing;
            let typography = &theme.semantic.typography;
            let hover_surface = theme.materials.hover;
            let release_note_items = release_notes
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if line.is_empty() {
                        return None;
                    }
                    let line = line
                        .strip_prefix("- ")
                        .or_else(|| line.strip_prefix("* "))
                        .unwrap_or(line)
                        .to_owned();
                    Some(
                        div()
                            .flex()
                            .items_start()
                            .gap(spacing.sm)
                            .child(div().text_color(colors.accent).child("•"))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(typography.sm.size)
                                    .text_color(colors.foreground)
                                    .child(line),
                            ),
                    )
                })
                .collect::<Vec<_>>();
            div()
                .relative()
                .size_full()
                .child(content)
                .child(
                    div()
                        .id("update-ready-backdrop")
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(0x000000).opacity(0.38))
                        .occlude()
                        .on_click(|_, _, _| {})
                        .child(
                            div()
                                .id("update-ready-dialog")
                                .w(px(500.))
                                .track_focus(&self.update_prompt_focus)
                                .flex()
                                .flex_col()
                                .gap(spacing.md)
                                .p(spacing.xl)
                                .rounded(theme.semantic.radius.lg)
                                .border_1()
                                .border_color(colors.border)
                                .bg(theme.materials.surface)
                                .shadow_lg()
                                .on_click(|_, _, _| {})
                                .child(
                                    div().flex().flex_col().gap(spacing.xs).child(
                                        div()
                                            .text_size(typography.lg.size)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(
                                                self.i18n
                                                    .text(Message::UpdateReadyTitle)
                                                    .replace("{version}", &version_label),
                                            ),
                                    ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(spacing.sm)
                                        .child(
                                            div()
                                                .text_size(typography.xs.size)
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(colors.muted_foreground)
                                                .child(
                                                    self.i18n.text(Message::SettingsReleaseNotes),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("update-ready-notes")
                                                .w_full()
                                                .max_h(px(240.))
                                                .overflow_y_scroll()
                                                .p(spacing.md)
                                                .rounded(theme.semantic.radius.md)
                                                .bg(theme.materials.window)
                                                .flex()
                                                .flex_col()
                                                .gap(spacing.sm)
                                                .children(release_note_items),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .items_center()
                                        .gap(spacing.sm)
                                        .child(
                                            div()
                                                .id("update-ready-later")
                                                .h(px(34.))
                                                .px(spacing.md)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(theme.semantic.radius.md)
                                                .border_1()
                                                .border_color(colors.border)
                                                .bg(theme.materials.window)
                                                .text_size(typography.xs.size)
                                                .cursor_pointer()
                                                .hover(move |this| this.bg(hover_surface))
                                                .on_click(move |_, _, cx| {
                                                    let version = dismiss_version.clone();
                                                    let _ =
                                                        dismiss_shell.update(cx, |shell, cx| {
                                                            shell.dismissed_update_prompt =
                                                                Some(version);
                                                            cx.notify();
                                                        });
                                                })
                                                .child(self.i18n.text(Message::UpdateLater)),
                                        )
                                        .child(
                                            div()
                                                .id("update-ready-install")
                                                .h(px(34.))
                                                .px(spacing.md)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(theme.semantic.radius.md)
                                                .bg(colors.accent)
                                                .text_color(colors.accent_foreground)
                                                .text_size(typography.xs.size)
                                                .cursor_pointer()
                                                .hover(move |this| {
                                                    this.bg(colors.accent.opacity(0.88))
                                                })
                                                .on_click(move |_, window, cx| {
                                                    on_install(
                                                        PluginEvent::InstallUpdate {
                                                            version: version.clone(),
                                                            package_path: package_path.clone(),
                                                        },
                                                        window,
                                                        cx,
                                                    );
                                                })
                                                .child(
                                                    self.i18n.text(Message::SettingsInstallUpdate),
                                                ),
                                        ),
                                ),
                        ),
                )
                .into_any_element()
        } else {
            content
        };
        #[cfg(target_os = "macos")]
        {
            content
        }
        #[cfg(not(target_os = "macos"))]
        {
            gpui_kit::component::window_border()
                .child(content)
                .into_any_element()
        }
    }
}
