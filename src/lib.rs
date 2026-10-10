//! Application bootstrap and module boundaries for Jump1.

mod app;
mod hotkey;
mod i18n;
mod icon;
mod platform;
mod plugin;
mod state;
mod text_search;
mod theme;

use app::Jump1Shell;
use gpui_kit::{component::theme::Theme, *};
use plugin::system::ThemeChoice;

pub fn run() {
    application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            let previous_application = platform::frontmost_application_pid();
            cx.activate(true);
            let preferences = state::Preferences::load();
            match preferences.theme {
                ThemeChoice::System => Theme::sync_system_appearance(None, cx),
                ThemeChoice::Light => Theme::change(component::theme::ThemeMode::Light, None, cx),
                ThemeChoice::Dark => Theme::change(component::theme::ThemeMode::Dark, None, cx),
            }

            let bounds = Bounds::centered(None, theme::window_size(preferences.panel_size), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: true,
                    show: true,
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    is_resizable: true,
                    is_minimizable: false,
                    window_min_size: Some(theme::window_min_size()),
                    window_decorations: Some(WindowDecorations::Client),
                    window_background: WindowBackgroundAppearance::Blurred,
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    #[cfg(target_os = "macos")]
                    platform::hide_window_controls(window);
                    Theme::sync_system_appearance(Some(window), cx);
                    cx.new(|cx| Jump1Shell::new(previous_application, window, cx))
                },
            )
            .expect("failed to open Jump1 window");
        });
}
