//! System-level plugin for theme, locale, and panel size preferences.

use crate::{
    i18n::Locale,
    theme::{PanelSize, PanelSizePreset},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemChange {
    Theme(ThemeChoice),
    Locale(Locale),
    LaunchAtLogin(bool),
    PanelSizePreset(PanelSizePreset),
    CustomPanelSize { width: u32, height: u32 },
}

pub struct SystemPlugin {
    theme: ThemeChoice,
    locale: Locale,
    launch_at_login: bool,
    panel_size: PanelSize,
}

impl crate::plugin::api::Plugin for SystemPlugin {
    fn manifest(&self) -> crate::plugin::api::PluginManifest {
        crate::plugin::api::PluginManifest {
            id: crate::plugin::PluginId::System,
            name: "System",
            version: "0.1.0",
            kind: crate::plugin::api::PluginKind::Service,
            permissions: &[crate::plugin::api::PluginPermission::LoginItem],
        }
    }
}

impl SystemPlugin {
    pub fn new(
        theme: ThemeChoice,
        locale: Locale,
        launch_at_login: bool,
        panel_size: PanelSize,
    ) -> Self {
        Self {
            theme,
            locale,
            launch_at_login,
            panel_size: panel_size.normalized(),
        }
    }

    pub fn theme(&self) -> ThemeChoice {
        self.theme
    }
    pub fn locale(&self) -> Locale {
        self.locale
    }
    pub fn launch_at_login(&self) -> bool {
        self.launch_at_login
    }
    pub fn panel_size(&self) -> PanelSize {
        self.panel_size
    }

    pub fn apply(&mut self, change: SystemChange) {
        match change {
            SystemChange::Theme(theme) => self.theme = theme,
            SystemChange::Locale(locale) => self.locale = locale,
            SystemChange::LaunchAtLogin(enabled) => self.launch_at_login = enabled,
            SystemChange::PanelSizePreset(preset) => self.panel_size.preset = preset,
            SystemChange::CustomPanelSize { width, height } => {
                self.panel_size.preset = PanelSizePreset::Custom;
                self.panel_size.custom_width = width;
                self.panel_size.custom_height = height;
                self.panel_size = self.panel_size.normalized();
            }
        }
    }
}
