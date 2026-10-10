//! Application design tokens built on GPUI Kit's semantic theme.

use gpui_kit::component::theme::{ActiveTheme, SemanticThemeTokens};
use gpui_kit::{App, Hsla, Pixels, Size, px, size};

pub const MIN_PANEL_WIDTH: u32 = 700;
pub const MIN_PANEL_HEIGHT: u32 = 440;
pub const MAX_PANEL_WIDTH: u32 = 2400;
pub const MAX_PANEL_HEIGHT: u32 = 1600;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PanelSizePreset {
    Small,
    #[default]
    Medium,
    Large,
    Custom,
}

impl PanelSizePreset {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Custom => "custom",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "small" => Some(Self::Small),
            "medium" => Some(Self::Medium),
            "large" => Some(Self::Large),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelSize {
    pub preset: PanelSizePreset,
    pub custom_width: u32,
    pub custom_height: u32,
}

impl Default for PanelSize {
    fn default() -> Self {
        Self {
            preset: PanelSizePreset::Medium,
            custom_width: 760,
            custom_height: 560,
        }
    }
}

impl PanelSize {
    pub fn dimensions(self) -> (u32, u32) {
        match self.preset {
            PanelSizePreset::Small => (MIN_PANEL_WIDTH, MIN_PANEL_HEIGHT),
            PanelSizePreset::Medium => (760, 560),
            PanelSizePreset::Large => (1000, 760),
            PanelSizePreset::Custom => (self.custom_width, self.custom_height),
        }
    }

    pub fn normalized(self) -> Self {
        Self {
            custom_width: self.custom_width.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH),
            custom_height: self.custom_height.clamp(MIN_PANEL_HEIGHT, MAX_PANEL_HEIGHT),
            ..self
        }
    }
}

pub fn window_size(panel_size: PanelSize) -> Size<Pixels> {
    let (width, height) = panel_size.dimensions();
    size(px(width as f32), px(height as f32))
}

pub fn window_min_size() -> Size<Pixels> {
    size(px(MIN_PANEL_WIDTH as f32), px(MIN_PANEL_HEIGHT as f32))
}

pub struct AppTheme {
    pub semantic: SemanticThemeTokens,
    pub materials: Materials,
    pub metrics: Metrics,
}

impl AppTheme {
    pub fn resolve(cx: &App) -> Self {
        let semantic = cx.theme().semantic_tokens();
        Self {
            materials: Materials::resolve(&semantic),
            semantic,
            metrics: Metrics::default(),
        }
    }
}

pub struct Materials {
    pub window: Hsla,
    pub surface: Hsla,
    pub sidebar: Hsla,
}

impl Materials {
    fn resolve(semantic: &SemanticThemeTokens) -> Self {
        Self {
            window: semantic.colors.background.opacity(0.82),
            surface: semantic.colors.surface.opacity(0.74),
            sidebar: semantic.colors.surface.opacity(0.58),
        }
    }
}

pub struct Metrics {
    pub divider: Pixels,
    pub header_top: Pixels,
    pub header_bottom: Pixels,
    pub search_height: Pixels,
    pub search_icon: Pixels,
    pub recent_application_tile_width: Pixels,
    pub recent_application_tile_height: Pixels,
    pub result_row_height: Pixels,
    pub result_icon_box: Pixels,
    pub result_icon: Pixels,
    pub application_icon: Pixels,
    pub status_bar_height: Pixels,
    pub clipboard_preview_image_height: Pixels,
    pub settings_sidebar_width: Pixels,
    pub settings_sidebar_item_height: Pixels,
    pub settings_sidebar_icon_box: Pixels,
    pub settings_sidebar_icon: Pixels,
    pub shortcut_key_width: Pixels,
    pub shortcut_copy_width: Pixels,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            divider: px(1.),
            header_top: px(22.),
            header_bottom: px(18.),
            search_height: px(44.),
            search_icon: px(20.),
            recent_application_tile_width: px(84.),
            recent_application_tile_height: px(72.),
            result_row_height: px(58.),
            result_icon_box: px(34.),
            result_icon: px(18.),
            application_icon: px(28.),
            status_bar_height: px(42.),
            clipboard_preview_image_height: px(260.),
            settings_sidebar_width: px(196.),
            settings_sidebar_item_height: px(40.),
            settings_sidebar_icon_box: px(26.),
            settings_sidebar_icon: px(15.),
            shortcut_key_width: px(176.),
            shortcut_copy_width: px(74.),
        }
    }
}
