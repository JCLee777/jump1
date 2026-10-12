//! Application design tokens built on GPUI Kit's semantic theme.

use gpui_kit::component::theme::{ActiveTheme, SemanticThemeTokens, Theme};
use gpui_kit::{App, Hsla, Pixels, Size, px, rgb, size};

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccentColor {
    #[default]
    Default,
    Blue,
    Purple,
    Pink,
    Orange,
    Green,
    Teal,
}

impl AccentColor {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
            Self::Orange => "orange",
            Self::Green => "green",
            Self::Teal => "teal",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "blue" => Some(Self::Blue),
            "purple" => Some(Self::Purple),
            "pink" => Some(Self::Pink),
            "orange" => Some(Self::Orange),
            "green" => Some(Self::Green),
            "teal" => Some(Self::Teal),
            _ => None,
        }
    }

    pub fn color(self) -> Option<Hsla> {
        let color = match self {
            Self::Default => return None,
            Self::Blue => 0x2563eb,
            Self::Purple => 0x7c3aed,
            Self::Pink => 0xdb2777,
            Self::Orange => 0xc2410c,
            Self::Green => 0x15803d,
            Self::Teal => 0x0e7490,
        };
        Some(rgb(color).into())
    }
}

pub fn apply_accent_color(accent: AccentColor, cx: &mut App) {
    let Some(color) = accent.color() else {
        let mode = Theme::global(cx).mode;
        Theme::change(mode, None, cx);
        return;
    };

    let foreground = contrast_foreground(color);
    Theme::update(cx, |theme| {
        theme.accent = color;
        theme.accent_foreground = foreground;
        theme.primary = color;
        theme.primary_active = color;
        theme.primary_foreground = foreground;
        theme.primary_hover = color;
        theme.sidebar_accent = color;
        theme.sidebar_accent_foreground = foreground;
        theme.list_active = color;
        theme.list_active_border = color;
        theme.list_hover = color.opacity(0.14);
        theme.tab_active = color;
        theme.tab_active_foreground = foreground;
        theme.button_primary = color;
        theme.button_primary_active = color;
        theme.button_primary_foreground = foreground;
        theme.button_primary_hover = color;
        theme.slider_bar = color;
        theme.slider_thumb = color;
        theme.ring = color;
    });
}

pub(crate) fn contrast_foreground(background: Hsla) -> Hsla {
    let channels = background.to_rgb();
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * linear(channels.r) + 0.7152 * linear(channels.g) + 0.0722 * linear(channels.b);
    if luminance > 0.179 {
        rgb(0x000000).into()
    } else {
        rgb(0xffffff).into()
    }
}

pub fn window_size(panel_size: PanelSize) -> Size<Pixels> {
    let (width, height) = panel_size.dimensions();
    size(px(width as f32), px(height as f32))
}

pub fn window_min_size() -> Size<Pixels> {
    size(px(MIN_PANEL_WIDTH as f32), px(MIN_PANEL_HEIGHT as f32))
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct Materials {
    pub window: Hsla,
    pub surface: Hsla,
    pub sidebar: Hsla,
    pub hover: Hsla,
    pub selection: Hsla,
    pub selection_foreground: Hsla,
    pub selection_control: Hsla,
    pub control: Hsla,
}

impl Materials {
    fn resolve(semantic: &SemanticThemeTokens) -> Self {
        let colors = semantic.colors;
        let is_light = colors.foreground.l < 0.5;
        let surface_tint = if is_light { -0.025 } else { 0.06 };
        let control_tint = if is_light { -0.10 } else { 0.09 };
        let sidebar_tint = if is_light { -0.02 } else { 0.03 };
        Self {
            // Keep the native blurred window light and use deeper gray for
            // cards and controls to make the important regions stand out.
            window: colors.background.opacity(0.72),
            surface: adjust_lightness(colors.surface, surface_tint).opacity(0.92),
            sidebar: adjust_lightness(colors.muted, sidebar_tint).opacity(0.78),
            hover: colors.accent.opacity(0.14),
            selection: colors.accent,
            selection_foreground: colors.accent_foreground,
            selection_control: colors.accent_foreground.opacity(0.14),
            control: adjust_lightness(colors.muted, control_tint).opacity(0.90),
        }
    }
}

fn adjust_lightness(color: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (color.l + amount).clamp(0.0, 1.0),
        ..color
    }
}

#[derive(Clone)]
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
    pub setting_row_height: Pixels,
    pub accent_swatch_size: Pixels,
    pub accent_swatch_hit_size: Pixels,
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
            setting_row_height: px(68.),
            accent_swatch_size: px(18.),
            accent_swatch_hit_size: px(32.),
            shortcut_key_width: px(176.),
            shortcut_copy_width: px(74.),
        }
    }
}
