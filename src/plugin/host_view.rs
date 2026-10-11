//! Unified rendering boundary between the application shell and plugin views.

use std::{rc::Rc, sync::Arc};

use gpui_kit::{component::input::InputState, *};

use crate::{i18n::I18n, theme::AppTheme, update::UpdateStatus};

use super::{
    PluginEvent, PluginId, PluginManager, clipboard::view as clipboard_view,
    launcher::view as launcher_view, protocol::CommandAction, settings::view as settings_view,
};

pub type EventHandler = Rc<dyn for<'a, 'b> Fn(PluginEvent, &'a mut Window, &'b mut App)>;

#[allow(clippy::too_many_arguments)]
pub fn render(
    plugins: &PluginManager,
    input: &Entity<InputState>,
    panel_width_input: &Entity<InputState>,
    panel_height_input: &Entity<InputState>,
    scroll: &ScrollHandle,
    clipboard_scroll: &UniformListScrollHandle,
    clipboard_preview_image: Option<Arc<RenderImage>>,
    selected: usize,
    status: Option<String>,
    i18n: &I18n,
    theme: &AppTheme,
    on_event: EventHandler,
    recording_shortcut: Option<u64>,
    editing_shortcut_query: Option<u64>,
    shortcut_conflicts: Vec<u64>,
    update_status: &UpdateStatus,
) -> AnyElement {
    match plugins.active() {
        PluginId::Launcher => {
            let handler = on_event.clone();
            let on_activate: launcher_view::ActivateHandler = Rc::new(move |action, window, cx| {
                handler(PluginEvent::Action(action), window, cx);
            });
            launcher_view::render(
                input,
                scroll,
                plugins.launcher(),
                plugins.results(),
                selected,
                status,
                i18n,
                theme,
                on_activate,
            )
        }
        PluginId::Clipboard => {
            let back_handler = on_event.clone();
            let activate_handler = on_event.clone();
            let sort_handler = on_event.clone();
            let clear_handler = on_event.clone();
            let on_back: clipboard_view::BackHandler = Rc::new(move |window, cx| {
                back_handler(PluginEvent::Back, window, cx);
            });
            let on_activate: clipboard_view::ActivateHandler = Rc::new(move |index, window, cx| {
                activate_handler(PluginEvent::PasteClipboard(index), window, cx);
            });
            let on_sort: clipboard_view::SortHandler = Rc::new(move |mode, window, cx| {
                sort_handler(PluginEvent::ClipboardSort(mode), window, cx);
            });
            let on_clear: clipboard_view::ClearHandler = Rc::new(move |window, cx| {
                clear_handler(PluginEvent::ClearClipboard, window, cx);
            });
            clipboard_view::render(
                input,
                clipboard_scroll,
                plugins.clipboard().current_entries(),
                plugins.clipboard().len(),
                plugins.clipboard().sort_mode(),
                plugins.clipboard().preview_open(),
                clipboard_preview_image,
                selected,
                status,
                i18n,
                theme,
                on_back,
                on_activate,
                on_sort,
                on_clear,
            )
        }
        PluginId::Json => plugins.json().page().clone().into_any_element(),
        PluginId::Yaml => plugins.yaml().page().clone().into_any_element(),
        PluginId::Translate => plugins.translate().page().clone().into_any_element(),
        PluginId::Settings => {
            let back_handler = on_event.clone();
            let change_handler = on_event;
            let section_handler = change_handler.clone();
            let shortcut_handler = section_handler.clone();
            let update_handler = section_handler.clone();
            let release_handler = section_handler.clone();
            let download_handler = section_handler.clone();
            let install_handler = section_handler.clone();
            let on_back: settings_view::BackHandler = Rc::new(move |window, cx| {
                back_handler(PluginEvent::Back, window, cx);
            });
            let on_change: settings_view::ChangeHandler = Rc::new(move |change, window, cx| {
                change_handler(PluginEvent::Settings(change), window, cx);
            });
            let on_section: settings_view::SectionHandler = Rc::new(move |section, window, cx| {
                section_handler(PluginEvent::SettingsSection(section), window, cx);
            });
            let on_shortcut: settings_view::ShortcutHandler = Rc::new(move |event, window, cx| {
                shortcut_handler(PluginEvent::ShortcutBinding(event), window, cx);
            });
            let on_check_updates: settings_view::CheckUpdatesHandler =
                Rc::new(move |window, cx| {
                    update_handler(PluginEvent::CheckForUpdates, window, cx);
                });
            let on_open_release: settings_view::OpenReleaseHandler =
                Rc::new(move |url, window, cx| {
                    release_handler(PluginEvent::Action(CommandAction::Url(url)), window, cx);
                });
            let on_download_update: settings_view::DownloadUpdateHandler =
                Rc::new(move |version, url, window, cx| {
                    download_handler(PluginEvent::DownloadUpdate { version, url }, window, cx);
                });
            let on_install_update: settings_view::InstallUpdateHandler =
                Rc::new(move |version, dmg_path, window, cx| {
                    install_handler(PluginEvent::InstallUpdate { version, dmg_path }, window, cx);
                });
            settings_view::render(
                input,
                panel_width_input,
                panel_height_input,
                plugins.setting_definitions(i18n),
                plugins.shortcuts().bindings().to_vec(),
                recording_shortcut,
                editing_shortcut_query,
                shortcut_conflicts,
                plugins.settings().section(),
                plugins.system(),
                i18n,
                theme,
                on_back,
                on_change,
                on_section,
                on_shortcut,
                update_status,
                on_check_updates,
                on_open_release,
                on_download_update,
                on_install_update,
                plugins.translate().settings_page().clone(),
            )
        }
        PluginId::System | PluginId::Shortcuts => {
            unreachable!("service plugins do not render pages")
        }
    }
}
