//! Rich clipboard history with versioned persistence and content classification.

pub mod view;

use gpui_kit::{ClipboardEntry as SystemEntry, ClipboardItem, ExternalPaths, Image, ImageFormat};
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::platform::{self, NativeClipboard, NativeClipboardItem, NativeClipboardRepresentation};

const HISTORY_MAGIC: &[u8; 5] = b"HDCB2";
const MAX_ENTRY_BYTES: usize = 32 * 1024 * 1024;
const MAX_DECODED_ENTRIES: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardSortMode {
    Recent,
    Frequent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HistoryLimit {
    Limited(usize),
    Unlimited,
}
impl HistoryLimit {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "50" => Some(Self::Limited(50)),
            "100" => Some(Self::Limited(100)),
            "200" => Some(Self::Limited(200)),
            "unlimited" => Some(Self::Unlimited),
            _ => None,
        }
    }
    fn value(self) -> String {
        match self {
            Self::Limited(value) => value.to_string(),
            Self::Unlimited => "unlimited".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ClipboardContent {
    Text(Arc<str>),
    Image {
        format: ImageFormat,
        bytes: Arc<[u8]>,
        id: u64,
    },
    Files(Arc<[PathBuf]>),
}
impl ClipboardContent {
    pub fn size(&self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            Self::Image { bytes, .. } => bytes.len(),
            Self::Files(paths) => paths.iter().map(|path| path.as_os_str().len()).sum(),
        }
    }
    pub fn searchable_text(&self) -> String {
        match self {
            Self::Text(value) => value.to_string(),
            Self::Image { format, .. } => format!("image {}", format.extension()),
            Self::Files(paths) => paths
                .iter()
                .map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
    fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => a == b,
            (
                Self::Image {
                    id: a, bytes: ab, ..
                },
                Self::Image {
                    id: b, bytes: bb, ..
                },
            ) => a == b || ab == bb,
            (Self::Files(a), Self::Files(b)) => a == b,
            _ => false,
        }
    }
    pub fn to_clipboard_item(&self) -> ClipboardItem {
        match self {
            Self::Text(value) => ClipboardItem::new_string(value.to_string()),
            Self::Image { format, bytes, .. } => {
                ClipboardItem::new_image(&Image::from_bytes(*format, bytes.to_vec()))
            }
            Self::Files(paths) => ClipboardItem {
                entries: vec![SystemEntry::ExternalPaths(ExternalPaths(
                    paths.iter().cloned().collect(),
                ))],
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardFormat {
    PlainText,
    Json,
    Yaml,
    Http,
    Bash,
    Sql,
    Html,
    Xml,
    Markdown,
    Email,
    Jwt,
    Pdf,
    Word,
    Spreadsheet,
    Presentation,
    Image,
    Audio,
    Video,
    Archive,
    Code,
    Folder,
    File,
}
impl ClipboardFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlainText => "TEXT",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Http => "HTTP",
            Self::Bash => "BASH",
            Self::Sql => "SQL",
            Self::Html => "HTML",
            Self::Xml => "XML",
            Self::Markdown => "MARKDOWN",
            Self::Email => "EMAIL",
            Self::Jwt => "JWT",
            Self::Pdf => "PDF",
            Self::Word => "WORD",
            Self::Spreadsheet => "SHEET",
            Self::Presentation => "SLIDES",
            Self::Image => "IMAGE",
            Self::Audio => "AUDIO",
            Self::Video => "VIDEO",
            Self::Archive => "ARCHIVE",
            Self::Code => "CODE",
            Self::Folder => "FOLDER",
            Self::File => "FILE",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClipboardEntry {
    pub content: ClipboardContent,
    pub native: Option<Arc<NativeClipboard>>,
    pub captured_at: u64,
    pub use_count: u64,
    pub format: ClipboardFormat,
}

pub struct ClipboardSelection {
    pub item: ClipboardItem,
    pub native: Option<Arc<NativeClipboard>>,
}

pub struct ClipboardPlugin {
    entries: Vec<ClipboardEntry>,
    storage: Option<crate::plugin::storage::PluginStorage>,
    query: String,
    history_limit: HistoryLimit,
    sort_mode: ClipboardSortMode,
    preview_open: bool,
    preview_image: Option<(Arc<[u8]>, Arc<Image>)>,
    dirty: bool,
    ignored_clipboard: Option<ClipboardContent>,
    last_native_change_count: Option<isize>,
}

impl crate::plugin::api::Plugin for ClipboardPlugin {
    fn manifest(&self) -> crate::plugin::api::PluginManifest {
        crate::plugin::api::PluginManifest {
            id: crate::plugin::PluginId::Clipboard,
            name: "Clipboard",
            version: "0.2.0",
            kind: crate::plugin::api::PluginKind::Hybrid,
            permissions: &[
                crate::plugin::api::PluginPermission::ClipboardRead,
                crate::plugin::api::PluginPermission::ClipboardWrite,
            ],
        }
    }

    fn on_shutdown(&mut self) -> crate::plugin::api::PluginResult {
        self.persist_if_dirty();
        Ok(())
    }
}

impl ClipboardPlugin {
    pub fn matches_launcher_query(query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || ["剪切板", "剪贴板", "clipboard"]
                .iter()
                .any(|keyword| keyword.contains(&query) || query.contains(keyword))
    }
    pub fn load(storage: Option<crate::plugin::storage::PluginStorage>) -> Self {
        let history_limit = storage
            .as_ref()
            .and_then(|s| s.read_text("settings.conf"))
            .as_deref()
            .and_then(HistoryLimit::parse)
            .unwrap_or(HistoryLimit::Limited(100));
        let mut entries = storage
            .as_ref()
            .and_then(|s| s.read("data.bin"))
            .map(|bytes| decode(&bytes))
            .unwrap_or_default();
        if let HistoryLimit::Limited(limit) = history_limit {
            entries.truncate(limit);
        }
        Self {
            entries,
            storage,
            query: String::new(),
            history_limit,
            sort_mode: ClipboardSortMode::Recent,
            preview_open: false,
            preview_image: None,
            dirty: false,
            ignored_clipboard: None,
            last_native_change_count: None,
        }
    }
    pub fn setting_definitions(
        &self,
        i18n: &crate::i18n::I18n,
    ) -> Vec<crate::plugin::settings::SettingDefinition> {
        vec![crate::plugin::settings::SettingDefinition {
            plugin: crate::plugin::PluginId::Clipboard,
            key: "history_limit",
            title: i18n
                .text(crate::i18n::Message::ClipboardHistoryLimit)
                .into(),
            description: i18n
                .text(crate::i18n::Message::ClipboardHistoryLimitDescription)
                .into(),
            options: vec![
                ("50".into(), "50".into()),
                ("100".into(), "100".into()),
                ("200".into(), "200".into()),
                (
                    "unlimited".into(),
                    i18n.text(crate::i18n::Message::ClipboardHistoryUnlimited)
                        .into(),
                ),
            ],
            selected: self.history_limit.value(),
        }]
    }
    pub fn apply_setting(&mut self, key: &str, value: &str) -> bool {
        if key != "history_limit" {
            return false;
        }
        let Some(limit) = HistoryLimit::parse(value) else {
            return false;
        };
        self.history_limit = limit;
        self.apply_limit();
        self.persist();
        self.dirty = false;
        self.persist_settings();
        true
    }
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }
    pub fn clear_query(&mut self) {
        self.query.clear();
        self.close_preview();
    }
    pub fn preview_open(&self) -> bool {
        self.preview_open
    }
    pub fn preview_image(&self) -> Option<Arc<Image>> {
        self.preview_image.as_ref().map(|(_, image)| image.clone())
    }
    pub fn open_preview(&mut self, index: usize) {
        self.preview_open = index < self.current_entries().len();
        if !self.preview_open {
            self.preview_image = None;
            return;
        }
        self.update_preview_image(index);
    }
    pub fn close_preview(&mut self) {
        self.preview_open = false;
        self.preview_image = None;
    }
    pub fn update_preview_image(&mut self, index: usize) {
        if !self.preview_open {
            return;
        }
        let image = self.current_entries().get(index).and_then(|entry| {
            if let ClipboardContent::Image { format, bytes, .. } = &entry.content {
                Some((*format, bytes.clone()))
            } else {
                None
            }
        });
        if let Some((format, bytes)) = image {
            if self
                .preview_image
                .as_ref()
                .is_some_and(|(cached, _)| Arc::ptr_eq(cached, &bytes))
            {
                return;
            }
            let image = Image::from_bytes(format, bytes.to_vec());
            self.preview_image = Some((bytes.clone(), Arc::new(image)));
        } else {
            self.preview_image = None;
        }
    }
    pub fn sort_mode(&self) -> ClipboardSortMode {
        self.sort_mode
    }
    pub fn set_sort_mode(&mut self, mode: ClipboardSortMode) {
        self.sort_mode = mode;
    }
    pub fn toggle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            ClipboardSortMode::Recent => ClipboardSortMode::Frequent,
            ClipboardSortMode::Frequent => ClipboardSortMode::Recent,
        };
    }
    pub fn current_entries(&self) -> Vec<&ClipboardEntry> {
        let query = self.query.trim().to_lowercase();
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| {
                query.is_empty()
                    || entry
                        .content
                        .searchable_text()
                        .to_lowercase()
                        .contains(&query)
                    || entry.format.label().to_lowercase().contains(&query)
            })
            .collect();
        match self.sort_mode {
            ClipboardSortMode::Recent => entries.sort_by_key(|e| std::cmp::Reverse(e.captured_at)),
            ClipboardSortMode::Frequent => {
                entries.sort_by_key(|e| std::cmp::Reverse((e.use_count, e.captured_at)))
            }
        }
        entries
    }
    pub fn activate_entry(&mut self, index: usize) -> Option<ClipboardSelection> {
        let entry_index = {
            let selected = *self.current_entries().get(index)?;
            self.entries
                .iter()
                .position(|entry| std::ptr::eq(entry, selected))?
        };
        let entry = &mut self.entries[entry_index];
        entry.use_count = entry.use_count.saturating_add(1);
        let selection = ClipboardSelection {
            item: entry.content.to_clipboard_item(),
            native: entry.native.clone(),
        };
        // Persist on the next clipboard polling pass. Encoding an unlimited
        // history, especially one containing images, must not block paste.
        self.dirty = true;
        Some(selection)
    }
    pub fn capture(&mut self, item: ClipboardItem) -> bool {
        let change_count = platform::native_clipboard_change_count();
        if change_count.is_some() && change_count == self.last_native_change_count {
            self.persist_if_dirty();
            return false;
        }
        if platform::native_clipboard_is_concealed() {
            self.last_native_change_count = change_count;
            self.persist_if_dirty();
            return false;
        }
        let native = platform::read_native_clipboard();
        self.last_native_change_count = change_count;
        let Some(content) = content_from_item(item) else {
            self.persist_if_dirty();
            return false;
        };
        if change_count.is_none()
            && self
                .ignored_clipboard
                .as_ref()
                .is_some_and(|ignored| ignored.same_as(&content))
        {
            self.persist_if_dirty();
            return false;
        }
        self.ignored_clipboard = None;
        if content.size() == 0 || content.size() > MAX_ENTRY_BYTES {
            self.persist_if_dirty();
            return false;
        }
        if self
            .entries
            .first()
            .is_some_and(|entry| entry.content.same_as(&content))
        {
            if let Some(native) = native {
                self.entries[0].native = Some(Arc::new(native));
                self.dirty = true;
            }
            self.persist_if_dirty();
            return false;
        }
        let use_count = self
            .entries
            .iter()
            .find(|entry| entry.content.same_as(&content))
            .map_or(1, |entry| entry.use_count.saturating_add(1));
        self.entries
            .retain(|entry| !entry.content.same_as(&content));
        self.entries.insert(
            0,
            ClipboardEntry {
                format: detect_format(&content),
                content,
                native: native.map(Arc::new),
                captured_at: now_seconds(),
                use_count,
            },
        );
        self.apply_limit();
        self.persist();
        self.dirty = false;
        true
    }
    pub fn clear(&mut self, current_clipboard: Option<ClipboardItem>) {
        self.close_preview();
        self.ignore_item(current_clipboard);
        self.entries.clear();
        self.persist();
        self.dirty = false;
    }
    pub fn ignore_item(&mut self, item: Option<ClipboardItem>) {
        self.last_native_change_count = platform::native_clipboard_change_count();
        self.ignored_clipboard = if self.last_native_change_count.is_some() {
            None
        } else {
            item.and_then(content_from_item)
        };
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    fn apply_limit(&mut self) {
        if let HistoryLimit::Limited(limit) = self.history_limit {
            self.entries.truncate(limit);
        }
    }
    fn persist(&self) {
        if let Some(storage) = &self.storage {
            let _ = storage.write_atomic("data.bin", &encode(&self.entries));
        }
    }
    fn persist_if_dirty(&mut self) {
        if self.dirty {
            self.persist();
            self.dirty = false;
        }
    }
    fn persist_settings(&self) {
        if let Some(storage) = &self.storage {
            let _ = storage.write_text_atomic("settings.conf", &self.history_limit.value());
        }
    }
}

fn content_from_item(item: ClipboardItem) -> Option<ClipboardContent> {
    let entries: Vec<_> = item.into_entries().collect();
    for entry in &entries {
        if let SystemEntry::ExternalPaths(paths) = entry {
            if !paths.paths().is_empty() {
                return Some(ClipboardContent::Files(
                    paths.paths().iter().cloned().collect(),
                ));
            }
        }
    }
    for entry in &entries {
        if let SystemEntry::Image(image) = entry {
            return Some(ClipboardContent::Image {
                format: image.format(),
                bytes: image.bytes().to_vec().into(),
                id: image.id,
            });
        }
    }
    entries.into_iter().find_map(|entry| match entry {
        SystemEntry::String(value) if !value.text.trim().is_empty() => {
            Some(ClipboardContent::Text(value.text.into()))
        }
        _ => None,
    })
}

fn detect_format(content: &ClipboardContent) -> ClipboardFormat {
    match content {
        ClipboardContent::Image { .. } => ClipboardFormat::Image,
        ClipboardContent::Files(paths) => paths
            .first()
            .map_or(ClipboardFormat::File, |path| detect_path_format(path)),
        ClipboardContent::Text(text) => detect_text_format(text),
    }
}
fn detect_text_format(text: &str) -> ClipboardFormat {
    let value = text.trim();
    let lower = value.to_ascii_lowercase();
    if serde_json::from_str::<serde_json::Value>(value).is_ok() {
        ClipboardFormat::Json
    } else if matches!(
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(value),
        Ok(serde_yaml_ng::Value::Mapping(_)) | Ok(serde_yaml_ng::Value::Sequence(_))
    ) {
        ClipboardFormat::Yaml
    } else if lower.starts_with("http://") || lower.starts_with("https://") {
        ClipboardFormat::Http
    } else if value.starts_with("#!")
        || [
            "sudo ", "cd ", "npm ", "cargo ", "git ", "curl ", "brew ", "docker ",
        ]
        .iter()
        .any(|p| lower.starts_with(p))
    {
        ClipboardFormat::Bash
    } else if [
        "select ",
        "insert ",
        "update ",
        "delete ",
        "create table",
        "alter table",
    ]
    .iter()
    .any(|p| lower.starts_with(p))
    {
        ClipboardFormat::Sql
    } else if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        ClipboardFormat::Html
    } else if lower.starts_with("<?xml") {
        ClipboardFormat::Xml
    } else if value
        .lines()
        .any(|line| line.starts_with("# ") || line.starts_with("```"))
    {
        ClipboardFormat::Markdown
    } else if value.matches('.').count() == 2
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
    {
        ClipboardFormat::Jwt
    } else if value.contains('@') && !value.contains(char::is_whitespace) {
        ClipboardFormat::Email
    } else {
        ClipboardFormat::PlainText
    }
}
fn detect_path_format(path: &Path) -> ClipboardFormat {
    if path.is_dir() {
        return ClipboardFormat::Folder;
    }
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => ClipboardFormat::Pdf,
        "doc" | "docx" | "rtf" | "pages" => ClipboardFormat::Word,
        "xls" | "xlsx" | "csv" | "numbers" => ClipboardFormat::Spreadsheet,
        "ppt" | "pptx" | "key" => ClipboardFormat::Presentation,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "tiff" | "heic" => {
            ClipboardFormat::Image
        }
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" => ClipboardFormat::Audio,
        "mp4" | "mov" | "mkv" | "avi" | "webm" => ClipboardFormat::Video,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => ClipboardFormat::Archive,
        "yaml" | "yml" => ClipboardFormat::Yaml,
        "rs" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java" | "kt" | "swift" | "c"
        | "cpp" | "h" | "css" | "scss" | "toml" => ClipboardFormat::Code,
        _ => ClipboardFormat::File,
    }
}
fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn encode(entries: &[ClipboardEntry]) -> Vec<u8> {
    let mut output = HISTORY_MAGIC.to_vec();
    for entry in entries {
        let (kind, payload) = match &entry.content {
            ClipboardContent::Text(value) => (0, value.as_bytes().to_vec()),
            ClipboardContent::Image { format, bytes, .. } => {
                let mut value = vec![image_code(*format)];
                value.extend_from_slice(bytes);
                (1, value)
            }
            ClipboardContent::Files(paths) => (
                2,
                paths
                    .iter()
                    .map(|p| p.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\0")
                    .into_bytes(),
            ),
        };
        let Ok(length) = u32::try_from(payload.len()) else {
            continue;
        };
        let native = entry.native.as_ref().and_then(|value| encode_native(value));
        let native = native.as_deref().unwrap_or_default();
        let Ok(native_length) = u32::try_from(native.len()) else {
            continue;
        };
        output.extend_from_slice(&entry.captured_at.to_le_bytes());
        output.extend_from_slice(&entry.use_count.to_le_bytes());
        output.push(kind);
        output.extend_from_slice(&length.to_le_bytes());
        output.extend_from_slice(&payload);
        output.extend_from_slice(&native_length.to_le_bytes());
        output.extend_from_slice(native);
    }
    output
}
fn decode(bytes: &[u8]) -> Vec<ClipboardEntry> {
    if bytes.starts_with(HISTORY_MAGIC) {
        decode_entries(&bytes[HISTORY_MAGIC.len()..])
    } else {
        Vec::new()
    }
}
fn decode_entries(bytes: &[u8]) -> Vec<ClipboardEntry> {
    let mut cursor = Cursor::new(bytes);
    let mut entries = Vec::new();
    while entries.len() < MAX_DECODED_ENTRIES {
        let mut timestamp = [0; 8];
        let mut uses = [0; 8];
        let mut kind = [0; 1];
        let mut length = [0; 4];
        if cursor.read_exact(&mut timestamp).is_err()
            || cursor.read_exact(&mut uses).is_err()
            || cursor.read_exact(&mut kind).is_err()
            || cursor.read_exact(&mut length).is_err()
        {
            break;
        }
        let length = u32::from_le_bytes(length) as usize;
        if length > MAX_ENTRY_BYTES {
            break;
        }
        let mut payload = vec![0; length];
        if cursor.read_exact(&mut payload).is_err() {
            break;
        }
        let Some(native_length) = read_u32(&mut cursor) else {
            break;
        };
        if native_length as usize > MAX_ENTRY_BYTES {
            break;
        }
        let mut native_payload = vec![0; native_length as usize];
        if cursor.read_exact(&mut native_payload).is_err() {
            break;
        }
        let content = match kind[0] {
            0 => String::from_utf8(payload)
                .ok()
                .map(|v| ClipboardContent::Text(v.into())),
            1 if !payload.is_empty() => {
                image_from_code(payload[0]).map(|format| ClipboardContent::Image {
                    format,
                    bytes: payload[1..].to_vec().into(),
                    id: u64::from_le_bytes(timestamp),
                })
            }
            2 => String::from_utf8(payload).ok().map(|v| {
                ClipboardContent::Files(
                    v.split('\0')
                        .filter(|p| !p.is_empty())
                        .map(PathBuf::from)
                        .collect::<Vec<_>>()
                        .into(),
                )
            }),
            _ => None,
        };
        if let Some(content) = content {
            entries.push(ClipboardEntry {
                format: detect_format(&content),
                content,
                native: decode_native(&native_payload).map(Arc::new),
                captured_at: u64::from_le_bytes(timestamp),
                use_count: u64::from_le_bytes(uses),
            });
        }
    }
    entries
}

fn encode_native(native: &NativeClipboard) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(&u32::try_from(native.items.len()).ok()?.to_le_bytes());
    for item in &native.items {
        output.extend_from_slice(
            &u32::try_from(item.representations.len())
                .ok()?
                .to_le_bytes(),
        );
        for representation in &item.representations {
            let name = representation.type_name.as_bytes();
            output.extend_from_slice(&u32::try_from(name.len()).ok()?.to_le_bytes());
            output.extend_from_slice(name);
            output.extend_from_slice(&u32::try_from(representation.data.len()).ok()?.to_le_bytes());
            output.extend_from_slice(&representation.data);
            if output.len() > MAX_ENTRY_BYTES {
                return None;
            }
        }
    }
    Some(output)
}

fn decode_native(bytes: &[u8]) -> Option<NativeClipboard> {
    if bytes.is_empty() {
        return None;
    }
    let mut cursor = Cursor::new(bytes);
    let item_count = read_u32(&mut cursor)? as usize;
    if item_count > 1024 {
        return None;
    }
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        let representation_count = read_u32(&mut cursor)? as usize;
        if representation_count > 256 {
            return None;
        }
        let mut representations = Vec::with_capacity(representation_count);
        for _ in 0..representation_count {
            let name_length = read_u32(&mut cursor)? as usize;
            if name_length > 1024 {
                return None;
            }
            let mut name = vec![0; name_length];
            cursor.read_exact(&mut name).ok()?;
            let data_length = read_u32(&mut cursor)? as usize;
            if data_length > MAX_ENTRY_BYTES || data_length > bytes.len() {
                return None;
            }
            let mut data = vec![0; data_length];
            cursor.read_exact(&mut data).ok()?;
            representations.push(NativeClipboardRepresentation {
                type_name: String::from_utf8(name).ok()?,
                data,
            });
        }
        items.push(NativeClipboardItem { representations });
    }
    (cursor.position() as usize == bytes.len()).then_some(NativeClipboard { items })
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Option<u32> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes).ok()?;
    Some(u32::from_le_bytes(bytes))
}
fn image_code(format: ImageFormat) -> u8 {
    match format {
        ImageFormat::Png => 0,
        ImageFormat::Jpeg => 1,
        ImageFormat::Webp => 2,
        ImageFormat::Gif => 3,
        ImageFormat::Svg => 4,
        ImageFormat::Bmp => 5,
        ImageFormat::Tiff => 6,
        ImageFormat::Ico => 7,
        ImageFormat::Pnm => 8,
    }
}
fn image_from_code(code: u8) -> Option<ImageFormat> {
    Some(match code {
        0 => ImageFormat::Png,
        1 => ImageFormat::Jpeg,
        2 => ImageFormat::Webp,
        3 => ImageFormat::Gif,
        4 => ImageFormat::Svg,
        5 => ImageFormat::Bmp,
        6 => ImageFormat::Tiff,
        7 => ImageFormat::Ico,
        8 => ImageFormat::Pnm,
        _ => return None,
    })
}
