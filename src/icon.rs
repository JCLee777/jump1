//! Project-owned SVG icon catalog and rendering helpers.

use gpui_kit::{Hsla, Pixels, Styled, component::Icon};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Glyph {
    Search,
    Application,
    ArrowDown,
    ArrowLeft,
    ArrowUp,
    Appearance,
    Audio,
    Clipboard,
    Compare,
    Copy,
    Format,
    File,
    Folder,
    Json,
    Image,
    Minify,
    Plugins,
    Sort,
    Trash,
    Translate,
    Web,
    Yaml,
}

impl Glyph {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Search => include_bytes!("../assets/icons/search.svg"),
            Self::Application => include_bytes!("../assets/icons/application.svg"),
            Self::ArrowDown => include_bytes!("../assets/icons/arrow-down.svg"),
            Self::ArrowLeft => include_bytes!("../assets/icons/arrow-left.svg"),
            Self::ArrowUp => include_bytes!("../assets/icons/arrow-up.svg"),
            Self::Appearance => include_bytes!("../assets/icons/appearance.svg"),
            Self::Audio => include_bytes!("../assets/icons/audio.svg"),
            Self::Clipboard => include_bytes!("../assets/icons/clipboard.svg"),
            Self::Compare => include_bytes!("../assets/icons/compare.svg"),
            Self::Copy => include_bytes!("../assets/icons/copy.svg"),
            Self::Format => include_bytes!("../assets/icons/format.svg"),
            Self::File => include_bytes!("../assets/icons/file.svg"),
            Self::Folder => include_bytes!("../assets/icons/folder.svg"),
            Self::Json => include_bytes!("../assets/icons/json.svg"),
            Self::Image => include_bytes!("../assets/icons/image.svg"),
            Self::Minify => include_bytes!("../assets/icons/minify.svg"),
            Self::Plugins => include_bytes!("../assets/icons/plugins.svg"),
            Self::Sort => include_bytes!("../assets/icons/sort.svg"),
            Self::Trash => include_bytes!("../assets/icons/trash.svg"),
            Self::Translate => include_bytes!("../assets/icons/translate.svg"),
            Self::Web => include_bytes!("../assets/icons/web.svg"),
            Self::Yaml => include_bytes!("../assets/icons/yaml.svg"),
        }
    }
}

pub fn glyph(glyph: Glyph, size: Pixels, color: Hsla) -> Icon {
    Icon::default()
        .data(glyph.bytes())
        .size(size)
        .text_color(color)
}
