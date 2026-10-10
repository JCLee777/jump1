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
        PluginCommand, PluginEvent, PluginId, PluginManager, host_view,
        protocol::{CommandAction, HostEffect},
        settings::SettingsChange,
        shortcuts::ShortcutBindingEvent,
        system::{SystemChange, ThemeChoice},
    },
    state::Preferences,
    theme::{
        AppTheme, MAX_PANEL_HEIGHT, MAX_PANEL_WIDTH, MIN_PANEL_HEIGHT, MIN_PANEL_WIDTH,
        PanelSizePreset,
    },
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
    selected: usize,
    status: Option<String>,
    recording_shortcut: Option<u64>,
    editing_shortcut_query: Option<u64>,
    shortcut_conflicts: Vec<u64>,
    previous_application: Option<i32>,
}

impl Jump1Shell {
    pub(crate) fn new(
        previous_application: Option<i32>,
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
            if !input_focus.is_focused(window) {
                return;
            }
            let handled = shell
                .update(cx, |shell, cx| {
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
                            shell.plugins.toggle_clipboard_sort();
                            shell.selected = 0;
                            shell.scroll_to_result(0);
                            if shell.plugins.clipboard().preview_open() {
                                shell.plugins.open_clipboard_preview(0);
                            }
                            shell.status = None;
                            cx.notify();
                            true
                        }
                        "up" => {
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
                                cx.hide();
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
        let appearance_subscription = cx.observe_window_appearance(window, |_, window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            cx.notify();
        });
        let activation_subscription = cx.observe_window_activation(window, |_, window, cx| {
            if !window.is_window_active() {
                cx.hide();
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
                                    .update(cx, |shell, cx| match action {
                                        ShortcutAction::ToggleLauncher => {
                                            shell.toggle_panel(window, cx)
                                        }
                                        ShortcutAction::RunBinding(id) => {
                                            shell.run_shortcut_binding(id, window, cx)
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
                    if let Some(pid) = platform::frontmost_application_pid() {
                        shell.previous_application = Some(pid);
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
            PluginEvent::ShortcutBinding(event) => {
                self.handle_shortcut_binding_event(event, window, cx);
            }
        }
    }

    fn activate_command(
        &mut self,
        action: crate::plugin::launcher::CommandAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let CommandAction::OpenPlugin(plugin) = action {
            self.show_plugin(plugin, window, cx);
            return true;
        }
        let result = match action {
            CommandAction::Application(app) => {
                let result = platform::launch_app(&app);
                if result.is_ok() {
                    self.plugins.record_application_usage(&app);
                }
                result
            }
            CommandAction::Path(path) => platform::open_path(&path),
            CommandAction::Url(url) => platform::open_url(&url),
            CommandAction::OpenPlugin(_) => unreachable!(),
        };
        let succeeded = result.is_ok();
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
            cx.hide();
            return;
        }
        if let Some(pid) = platform::frontmost_application_pid() {
            self.previous_application = Some(pid);
        }
        self.recording_shortcut = None;
        self.editing_shortcut_query = None;
        cx.activate(true);
        window.activate_window();
        self.focus_active_plugin(window, cx);
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
        self.plugins.open(PluginId::Launcher);
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
                cx.activate(true);
                window.activate_window();
                self.activate_command(action, window, cx);
            }
            PluginCommand::Action(action) => {
                if self.activate_command(action, window, cx) {
                    self.reset_input(self.plugins.input_placeholder(&self.i18n), window, cx);
                } else {
                    cx.activate(true);
                    window.activate_window();
                    window.focus(&self.input.focus_handle(cx), cx);
                    cx.notify();
                }
            }
            PluginCommand::PasteClipboard(_) | PluginCommand::None => {
                cx.activate(true);
                window.activate_window();
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
        self.plugins.open(plugin);
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
            HostEffect::ApplyTheme(ThemeChoice::System) => {
                Theme::sync_system_appearance(Some(window), cx);
            }
            HostEffect::ApplyTheme(ThemeChoice::Light) => Theme::change(ThemeMode::Light, None, cx),
            HostEffect::ApplyTheme(ThemeChoice::Dark) => Theme::change(ThemeMode::Dark, None, cx),
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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(native) = &selection.native {
            if let Err(error) = platform::write_native_clipboard(native) {
                eprintln!("failed to write native clipboard: {error}");
                cx.write_to_clipboard(selection.item.clone());
            }
        } else {
            cx.write_to_clipboard(selection.item.clone());
        }
        self.plugins.ignore_clipboard_item(Some(selection.item));
        self.plugins.close_clipboard_preview();
        cx.hide();
        let target_application = self.previous_application;
        if let Some(pid) = target_application {
            if let Err(error) = platform::activate_application(pid) {
                eprintln!("failed to restore previous application: {error}");
            }
        }
        let timer = cx.background_executor().clone();
        cx.spawn(async move |_, _| {
            if let Some(pid) = target_application {
                // Application activation is asynchronous on macOS. Observe the
                // real frontmost process instead of relying on a fixed delay.
                let mut restored = false;
                for _ in 0..24 {
                    if platform::frontmost_application_pid() == Some(pid) {
                        restored = true;
                        break;
                    }
                    timer.timer(Duration::from_millis(5)).await;
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
}

impl Render for Jump1Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = AppTheme::resolve(cx);
        let shell = cx.entity();
        let on_event: host_view::EventHandler = Rc::new(move |event, window, cx| {
            let _ = shell.update(cx, |shell, cx| {
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
            self.selected,
            self.status.clone(),
            &self.i18n,
            &theme,
            on_event,
            self.recording_shortcut,
            self.editing_shortcut_query,
            self.shortcut_conflicts.clone(),
        );
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
