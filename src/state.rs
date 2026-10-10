//! Persisted application-wide preferences.

use std::fs;

use crate::{
    i18n::Locale,
    platform,
    plugin::system::ThemeChoice,
    theme::{AccentColor, PanelSize, PanelSizePreset},
};

#[derive(Clone, Copy)]
pub struct Preferences {
    pub theme: ThemeChoice,
    pub accent_color: AccentColor,
    pub locale: Locale,
    pub panel_size: PanelSize,
}

impl Preferences {
    pub fn load() -> Self {
        let mut preferences = Self {
            theme: ThemeChoice::System,
            accent_color: AccentColor::Default,
            locale: Locale::from_environment(),
            panel_size: PanelSize::default(),
        };
        let Some(path) = platform::preferences_path() else {
            return preferences;
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return preferences;
        };
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (key, value) {
                ("accent_color", value) => {
                    if let Some(accent) = AccentColor::parse(value) {
                        preferences.accent_color = accent;
                    }
                }
                ("theme", "system") => preferences.theme = ThemeChoice::System,
                ("theme", "light") => preferences.theme = ThemeChoice::Light,
                ("theme", "dark") => preferences.theme = ThemeChoice::Dark,
                ("locale", "zh-CN") => preferences.locale = Locale::ZhCn,
                ("locale", "en-US") => preferences.locale = Locale::EnUs,
                ("panel_size", value) => {
                    if let Some(preset) = PanelSizePreset::parse(value) {
                        preferences.panel_size.preset = preset;
                    }
                }
                ("panel_width", value) => {
                    if let Ok(width) = value.parse() {
                        preferences.panel_size.custom_width = width;
                    }
                }
                ("panel_height", value) => {
                    if let Ok(height) = value.parse() {
                        preferences.panel_size.custom_height = height;
                    }
                }
                _ => {}
            }
        }
        preferences.panel_size = preferences.panel_size.normalized();
        preferences
    }

    pub fn save(self) {
        let Some(path) = platform::preferences_path() else {
            return;
        };
        let Some(parent) = path.parent() else { return };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let theme = match self.theme {
            ThemeChoice::System => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        };
        let locale = match self.locale {
            Locale::ZhCn => "zh-CN",
            Locale::EnUs => "en-US",
        };
        let accent_color = self.accent_color.key();
        let panel_size = self.panel_size.normalized();
        let temporary = path.with_extension("tmp");
        if fs::write(
            &temporary,
            format!(
                "version=3\ntheme={theme}\naccent_color={accent_color}\nlocale={locale}\npanel_size={}\npanel_width={}\npanel_height={}\n",
                panel_size.preset.key(),
                panel_size.custom_width,
                panel_size.custom_height,
            ),
        )
        .is_err()
            || fs::rename(&temporary, &path).is_err()
        {
            let _ = fs::remove_file(temporary);
        }
    }
}
