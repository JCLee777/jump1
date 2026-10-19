//! Application bootstrap and module boundaries for Jump1.

mod app;
mod hotkey;
mod i18n;
mod icon;
mod network;
mod platform;
mod plugin;
mod state;
mod text_search;
mod theme;
mod update;

use app::Jump1Shell;
use gpui_kit::{component::theme::Theme, *};
use plugin::system::ThemeChoice;
use theme::{AccentColor, apply_accent_color};

pub fn run() {
    #[cfg(target_os = "windows")]
    if !platform::acquire_single_instance() {
        return;
    }

    #[cfg(target_os = "windows")]
    let start_in_background = std::env::args_os().any(|argument| argument == "--background");
    #[cfg(not(target_os = "windows"))]
    let start_in_background = false;

    application()
        .with_http_client(network::gpui_http_client())
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            let previous_application = platform::frontmost_application();
            if !start_in_background {
                cx.activate(true);
            }
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
                    focus: !start_in_background,
                    show: !start_in_background,
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    app_owns_titlebar_drag: true,
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
                    #[cfg(target_os = "windows")]
                    platform::install_tray_icon(window);
                    platform::center_window_on_pointer(window);
                    Theme::sync_system_appearance(Some(window), cx);
                    let shell = cx.new(|cx| Jump1Shell::new(previous_application, window, cx));
                    if preferences.accent_color != AccentColor::Default {
                        apply_accent_color(preferences.accent_color, cx);
                    }
                    shell
                },
            )
            .expect("failed to open Jump1 window");
        });
}
