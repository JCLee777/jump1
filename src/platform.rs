//! Small platform boundary for discovering and opening installed applications.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(target_os = "macos")]
use std::{
    collections::hash_map::DefaultHasher,
    fs::File,
    hash::{Hash, Hasher},
    io::{BufReader, BufWriter},
    time::UNIX_EPOCH,
};

use crate::{plugin::protocol::ApplicationEntry, text_search};

#[derive(Clone, Debug)]
pub struct NativeClipboard {
    pub items: Vec<NativeClipboardItem>,
}

#[derive(Clone, Debug)]
pub struct NativeClipboardItem {
    pub representations: Vec<NativeClipboardRepresentation>,
}

#[derive(Clone, Debug)]
pub struct NativeClipboardRepresentation {
    pub type_name: String,
    pub data: Vec<u8>,
}

pub fn discover_apps() -> Vec<ApplicationEntry> {
    let mut apps = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let mut roots = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
        ];
        if let Some(home) = home_dir() {
            roots.push(home.join("Applications"));
        }
        for root in roots {
            scan_macos_apps(&root, 2, &mut apps);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mut roots = Vec::new();
        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            roots.push(PathBuf::from(data_home).join("applications"));
        } else if let Some(home) = home_dir() {
            roots.push(home.join(".local/share/applications"));
        }
        let data_dirs =
            std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        roots.extend(
            data_dirs
                .split(':')
                .map(|dir| PathBuf::from(dir).join("applications")),
        );
        for root in roots {
            scan_linux_apps(&root, &mut apps);
        }
    }

    #[cfg(target_os = "windows")]
    {
        for variable in ["APPDATA", "PROGRAMDATA"] {
            if let Ok(base) = std::env::var(variable) {
                scan_windows_shortcuts(
                    &PathBuf::from(base).join("Microsoft/Windows/Start Menu/Programs"),
                    4,
                    &mut apps,
                );
            }
        }
    }

    apps.sort_by(|left, right| left.path.cmp(&right.path));
    apps.dedup_by(|left, right| left.path == right.path);
    apps.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    apps
}

/// Keep AppKit's native resize behavior without showing its window controls.
#[cfg(target_os = "macos")]
pub fn hide_window_controls(window: &gpui_kit::Window) {
    use objc2_app_kit::{NSView, NSWindowButton};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    // The AppKit handle owns this view for the lifetime of `window`.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let Some(native_window) = view.window() else {
        return;
    };
    for button in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(control) = native_window.standardWindowButton(button) {
            control.setHidden(true);
        }
    }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    })
    .map(PathBuf::from)
}

pub fn preferences_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = {
        let support_dir = std::env::var_os("HOME")
            .map(PathBuf::from)?
            .join("Library/Application Support");
        relocated_data_dir(
            support_dir.join("io.github.jclee777.jump1"),
            support_dir.join("dev.huandian"),
        )
    };
    #[cfg(target_os = "windows")]
    let base = {
        let app_data = PathBuf::from(std::env::var_os("APPDATA")?);
        relocated_data_dir(app_data.join("Jump1"), app_data.join("HuanDian"))
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = {
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".config")))?;
        relocated_data_dir(config_dir.join("jump1"), config_dir.join("huandian"))
    };
    Some(base.join("preferences.conf"))
}

// Keep existing preferences and plugin data when the product's storage name changes.
fn relocated_data_dir(current: PathBuf, legacy: PathBuf) -> PathBuf {
    if current.exists() || !legacy.exists() {
        return current;
    }

    if std::fs::rename(&legacy, &current).is_ok() {
        current
    } else {
        legacy
    }
}

pub fn plugin_data_dir() -> Option<PathBuf> {
    preferences_path()?
        .parent()
        .map(|path| path.join("plugins"))
}

pub fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer.exe");

    command
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };

    command
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn launch_app(app: &ApplicationEntry) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        return Command::new("gio")
            .arg("launch")
            .arg(&app.path)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }

    #[cfg(not(target_os = "linux"))]
    open_path(&app.path)
}

pub fn trigger_copy() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return macos_trigger_shortcut(8);
    }

    #[cfg(target_os = "windows")]
    {
        return windows_trigger_shortcut(0x43);
    }

    #[cfg(target_os = "linux")]
    {
        return Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+c"])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }

    #[allow(unreachable_code)]
    Err("copy simulation is unavailable on this platform".into())
}

pub fn trigger_paste() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return macos_trigger_shortcut(9);
    }
    #[cfg(target_os = "windows")]
    {
        return windows_trigger_shortcut(0x56);
    }
    #[cfg(target_os = "linux")]
    {
        return Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    #[allow(unreachable_code)]
    Err("paste simulation is unavailable on this platform".into())
}

pub fn read_native_clipboard() -> Option<NativeClipboard> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSPasteboard;

        const MAX_NATIVE_BYTES: usize = 32 * 1024 * 1024;
        let pasteboard = NSPasteboard::generalPasteboard();
        let mut total_bytes: usize = 0;
        let mut items = Vec::new();
        for item in pasteboard.pasteboardItems()?.iter() {
            let mut representations = Vec::new();
            for data_type in item.types().iter() {
                let type_name = data_type.to_string();
                if type_name.starts_with("dyn.") || type_name == "org.nspasteboard.ConcealedType" {
                    continue;
                }
                let Some(data) = item.dataForType(&data_type) else {
                    continue;
                };
                total_bytes = total_bytes.checked_add(data.length())?;
                if total_bytes > MAX_NATIVE_BYTES {
                    return None;
                }
                representations.push(NativeClipboardRepresentation {
                    type_name,
                    data: data.to_vec(),
                });
            }
            if !representations.is_empty() {
                items.push(NativeClipboardItem { representations });
            }
        }
        return (!items.is_empty()).then_some(NativeClipboard { items });
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn native_clipboard_change_count() -> Option<isize> {
    #[cfg(target_os = "macos")]
    {
        return Some(objc2_app_kit::NSPasteboard::generalPasteboard().changeCount());
    }
    #[cfg(not(target_os = "macos"))]
    None
}

pub fn native_clipboard_is_concealed() -> bool {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSPasteboard;

        return NSPasteboard::generalPasteboard()
            .pasteboardItems()
            .is_some_and(|items| {
                items.iter().any(|item| {
                    item.types()
                        .iter()
                        .any(|data_type| data_type.to_string() == "org.nspasteboard.ConcealedType")
                })
            });
    }
    #[cfg(not(target_os = "macos"))]
    false
}

pub fn write_native_clipboard(clipboard: &NativeClipboard) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2::{rc::Retained, runtime::ProtocolObject};
        use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
        use objc2_foundation::{NSArray, NSData, NSString};

        if clipboard.items.is_empty() {
            return Err("clipboard item has no representations".into());
        }
        let pasteboard = NSPasteboard::generalPasteboard();
        let mut file_objects: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = Vec::new();
        let mut wrote_data = false;
        pasteboard.clearContents();
        for native_item in &clipboard.items {
            for representation in &native_item.representations {
                let data_type = NSString::from_str(&representation.type_name);
                let data = NSData::with_bytes(&representation.data);
                if representation.type_name == "public.file-url" {
                    let item = NSPasteboardItem::new();
                    if !item.setData_forType(&data, &data_type) {
                        return Err("could not write file URL clipboard item".into());
                    }
                    file_objects.push(ProtocolObject::from_retained(item));
                    continue;
                }
                if !pasteboard.setData_forType(Some(&data), &data_type) {
                    return Err(format!(
                        "could not write clipboard type {}",
                        representation.type_name
                    ));
                }
                wrote_data = true;
            }
        }
        if !file_objects.is_empty()
            && !pasteboard.writeObjects(&NSArray::from_retained_slice(&file_objects))
        {
            return Err("could not write file URL clipboard items".into());
        }
        if wrote_data || !file_objects.is_empty() {
            return Ok(());
        }
        return Err("clipboard item has no writable representations".into());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = clipboard;
        Err("native clipboard is unavailable on this platform".into())
    }
}

pub fn frontmost_application_pid() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;

        let application = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let pid = application.processIdentifier();
        return (pid > 0 && pid != std::process::id() as i32).then_some(pid);
    }

    #[cfg(not(target_os = "macos"))]
    None
}

pub fn activate_application(pid: i32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

        let application = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            .ok_or_else(|| format!("application process {pid} is no longer running"))?;
        if application.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows) {
            return Ok(());
        }
        return Err(format!(
            "macOS refused to activate application process {pid}"
        ));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        Ok(())
    }
}

pub fn launch_at_login_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        use smappservice_rs::{AppService, ServiceStatus, ServiceType};

        return matches!(
            AppService::new(ServiceType::MainApp).status(),
            ServiceStatus::Enabled | ServiceStatus::RequiresApproval
        );
    }

    #[cfg(not(target_os = "macos"))]
    false
}

pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use smappservice_rs::{AppService, ServiceStatus, ServiceType};

        let service = AppService::new(ServiceType::MainApp);
        let status = service.status();
        if enabled {
            if status == ServiceStatus::Enabled {
                return Ok(());
            }
            if status == ServiceStatus::RequiresApproval {
                AppService::open_system_settings_login_items();
                return Ok(());
            }
            service.register().map_err(|error| error.to_string())?;
            if service.status() == ServiceStatus::RequiresApproval {
                AppService::open_system_settings_login_items();
            }
            return Ok(());
        }

        if status == ServiceStatus::NotRegistered || status == ServiceStatus::NotFound {
            return Ok(());
        }
        return service.unregister().map_err(|error| error.to_string());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = enabled;
        Err("launch at login is unavailable on this platform".into())
    }
}

#[cfg(target_os = "macos")]
fn macos_trigger_shortcut(virtual_key: u16) -> Result<(), String> {
    use std::ffi::c_void;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceCreate(state_id: u32) -> *mut c_void;
        fn CGEventCreateKeyboardEvent(
            source: *mut c_void,
            virtual_key: u16,
            key_down: bool,
        ) -> *mut c_void;
        fn CGEventSetFlags(event: *mut c_void, flags: u64);
        fn CGEventPost(tap: u32, event: *mut c_void);
        fn CGPreflightPostEventAccess() -> bool;
        fn CGRequestPostEventAccess() -> bool;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: *const c_void);
    }

    const COMMAND_FLAG: u64 = 1 << 20;
    const SESSION_EVENT_TAP: u32 = 1;

    unsafe {
        if !CGPreflightPostEventAccess() && !CGRequestPostEventAccess() {
            return Err("accessibility permission is required to send keyboard input".into());
        }
        let source = CGEventSourceCreate(0);
        if source.is_null() {
            return Err("could not create keyboard event source".into());
        }
        let events = [
            CGEventCreateKeyboardEvent(source, virtual_key, true),
            CGEventCreateKeyboardEvent(source, virtual_key, false),
        ];
        if events.iter().any(|event| event.is_null()) {
            for event in events.into_iter().filter(|event| !event.is_null()) {
                CFRelease(event);
            }
            CFRelease(source);
            return Err("could not create keyboard shortcut events".into());
        }
        CGEventSetFlags(events[0], COMMAND_FLAG | 0x8);
        CGEventSetFlags(events[1], COMMAND_FLAG | 0x8);
        for event in events {
            CGEventPost(SESSION_EVENT_TAP, event);
            CFRelease(event);
        }
        CFRelease(source);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_trigger_shortcut(key: u8) -> Result<(), String> {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn keybd_event(virtual_key: u8, scan_code: u8, flags: u32, extra_info: usize);
    }

    const CONTROL: u8 = 0x11;
    const KEY_UP: u32 = 0x0002;

    unsafe {
        keybd_event(CONTROL, 0, 0, 0);
        keybd_event(key, 0, 0, 0);
        keybd_event(key, 0, KEY_UP, 0);
        keybd_event(CONTROL, 0, KEY_UP, 0);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn scan_macos_apps(root: &Path, depth: usize, apps: &mut Vec<ApplicationEntry>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "app") {
            let (name, aliases) = macos_app_names(&path);
            let icon_path = macos_app_icon(&path);
            apps.push(application_entry(name, aliases, path, icon_path));
        } else if depth > 0 && path.is_dir() {
            scan_macos_apps(&path, depth - 1, apps);
        }
    }
}

#[cfg(target_os = "linux")]
fn scan_linux_apps(root: &Path, apps: &mut Vec<ApplicationEntry>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_none_or(|extension| extension != "desktop")
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if content
            .lines()
            .any(|line| line == "Hidden=true" || line == "NoDisplay=true")
        {
            continue;
        }
        if let Some(fallback_name) = content.lines().find_map(|line| line.strip_prefix("Name=")) {
            let localized_name = ["Name[zh_CN]=", "Name[zh_CN.UTF-8]=", "Name[zh]="]
                .into_iter()
                .find_map(|prefix| content.lines().find_map(|line| line.strip_prefix(prefix)));
            let name = localized_name.unwrap_or(fallback_name).to_owned();
            let aliases = (name != fallback_name)
                .then(|| fallback_name.to_owned())
                .into_iter()
                .collect();
            let icon_path = content
                .lines()
                .find_map(|line| line.strip_prefix("Icon="))
                .and_then(resolve_linux_icon);
            apps.push(application_entry(name, aliases, path, icon_path));
        }
    }
}

#[cfg(target_os = "windows")]
fn scan_windows_shortcuts(root: &Path, depth: usize, apps: &mut Vec<ApplicationEntry>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            apps.push(application_entry(name, Vec::new(), path, None));
        } else if depth > 0 && path.is_dir() {
            scan_windows_shortcuts(&path, depth - 1, apps);
        }
    }
}

fn application_entry(
    name: String,
    aliases: Vec<String>,
    path: PathBuf,
    icon_path: Option<PathBuf>,
) -> ApplicationEntry {
    let search_terms = text_search::application_terms(
        std::iter::once(name.as_str()).chain(aliases.iter().map(String::as_str)),
    );
    ApplicationEntry {
        name,
        search_terms,
        path,
        icon_path,
    }
}

#[cfg(target_os = "macos")]
fn macos_app_names(app_path: &Path) -> (String, Vec<String>) {
    let resources = app_path.join("Contents/Resources");
    let mut names = Vec::new();

    for locale in [
        "zh-Hans.lproj",
        "zh_CN.lproj",
        "zh-Hans_CN.lproj",
        "zh.lproj",
    ] {
        if let Some(name) = plist_display_name(&resources.join(locale).join("InfoPlist.strings")) {
            push_name(&mut names, name);
        }
    }

    let info_plist = app_path.join("Contents/Info.plist");
    if let Ok(value) = plist::Value::from_file(info_plist)
        && let Some(dictionary) = value.as_dictionary()
    {
        for key in ["CFBundleDisplayName", "CFBundleName"] {
            if let Some(name) = dictionary.get(key).and_then(plist::Value::as_string)
                && !name.contains("$(")
            {
                push_name(&mut names, name.to_owned());
            }
        }
    }

    push_name(
        &mut names,
        app_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    );

    let name = names.first().cloned().unwrap_or_default();
    let aliases = names.into_iter().skip(1).collect();
    (name, aliases)
}

#[cfg(target_os = "macos")]
fn plist_display_name(path: &Path) -> Option<String> {
    if let Ok(value) = plist::Value::from_file(path)
        && let Some(dictionary) = value.as_dictionary()
        && let Some(name) = ["CFBundleDisplayName", "CFBundleName"]
            .into_iter()
            .find_map(|key| dictionary.get(key).and_then(plist::Value::as_string))
            .filter(|name| !name.contains("$("))
    {
        return Some(name.to_owned());
    }

    let bytes = fs::read(path).ok()?;
    let contents = decode_strings_file(&bytes)?;
    ["CFBundleDisplayName", "CFBundleName"]
        .into_iter()
        .find_map(|key| openstep_string_value(&contents, key))
        .filter(|name| !name.contains("$("))
}

#[cfg(target_os = "macos")]
fn decode_strings_file(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little_endian = bytes.starts_with(&[0xff, 0xfe]);
        let units: Vec<_> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| {
                if little_endian {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        String::from_utf16(&units).ok()
    } else {
        String::from_utf8(bytes.to_vec()).ok()
    }
}

#[cfg(target_os = "macos")]
fn openstep_string_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.trim();
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim().trim_matches('"') != key {
            return None;
        }
        let value = value
            .trim()
            .trim_end_matches(';')
            .trim()
            .trim_matches('"')
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(target_os = "macos")]
fn push_name(names: &mut Vec<String>, name: String) {
    if !name.trim().is_empty() && !names.iter().any(|existing| existing == &name) {
        names.push(name);
    }
}

#[cfg(target_os = "macos")]
fn macos_app_icon(app_path: &Path) -> Option<PathBuf> {
    let resources = app_path.join("Contents/Resources");
    let plist = plist::Value::from_file(app_path.join("Contents/Info.plist")).ok();
    let declared_name = plist
        .as_ref()
        .and_then(plist::Value::as_dictionary)
        .and_then(|dictionary| dictionary.get("CFBundleIconFile"))
        .and_then(plist::Value::as_string);

    let source = declared_name
        .map(PathBuf::from)
        .map(|name| {
            if name.extension().is_some() {
                resources.join(name)
            } else {
                resources.join(name).with_extension("icns")
            }
        })
        .filter(|path| path.exists())
        .or_else(|| {
            let conventional = resources.join("AppIcon.icns");
            conventional.exists().then_some(conventional)
        })
        .or_else(|| {
            fs::read_dir(&resources).ok()?.flatten().find_map(|entry| {
                let path = entry.path();
                path.extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("icns"))
                    .then_some(path)
            })
        })?;

    cache_icns_as_png(&source)
}

#[cfg(target_os = "macos")]
fn cache_icns_as_png(source: &Path) -> Option<PathBuf> {
    let modified = source
        .metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    modified.hash(&mut hasher);

    let cache_dir = home_dir()?.join("Library/Caches/io.github.jclee777.jump1/icons");
    fs::create_dir_all(&cache_dir).ok()?;
    let target = cache_dir.join(format!("{:016x}.png", hasher.finish()));
    if target.exists() {
        return Some(target);
    }

    let family = icns::IconFamily::read(BufReader::new(File::open(source).ok()?)).ok()?;
    let icon_type = family
        .available_icons()
        .into_iter()
        .min_by_key(|icon_type| {
            let width = icon_type.pixel_width();
            if width >= 64 {
                width - 64
            } else {
                10_000 + 64 - width
            }
        })?;
    let image = family.get_icon_with_type(icon_type).ok()?;
    let temporary = target.with_extension("png.tmp");
    image
        .write_png(BufWriter::new(File::create(&temporary).ok()?))
        .ok()?;
    fs::rename(&temporary, &target).ok()?;
    Some(target)
}

#[cfg(target_os = "linux")]
fn resolve_linux_icon(value: &str) -> Option<PathBuf> {
    let requested = PathBuf::from(value);
    if requested.is_absolute() {
        return requested.exists().then_some(requested);
    }

    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        roots.push(home.join(".local/share/icons"));
        roots.push(home.join(".icons"));
    }
    roots.push(PathBuf::from("/usr/share/icons"));
    roots.push(PathBuf::from("/usr/share/pixmaps"));

    let file_names: Vec<_> = if requested.extension().is_some() {
        vec![requested]
    } else {
        ["png", "svg", "ico"]
            .into_iter()
            .map(|extension| requested.with_extension(extension))
            .collect()
    };
    let locations = [
        "",
        "hicolor/64x64/apps",
        "hicolor/48x48/apps",
        "hicolor/32x32/apps",
        "hicolor/scalable/apps",
        "Adwaita/64x64/apps",
        "Adwaita/48x48/apps",
        "Adwaita/scalable/apps",
    ];

    roots.into_iter().find_map(|root| {
        locations.iter().find_map(|location| {
            file_names.iter().find_map(|file_name| {
                let candidate = root.join(location).join(file_name);
                candidate.exists().then_some(candidate)
            })
        })
    })
}
