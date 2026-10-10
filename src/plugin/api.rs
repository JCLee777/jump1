//! Stable in-process plugin contract used by the built-in registry.

#![allow(dead_code)]

use super::PluginId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginPermission {
    ClipboardRead,
    ClipboardWrite,
    Filesystem,
    InputSimulation,
    LoginItem,
    Network,
    ProcessLaunch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginKind {
    Page,
    Service,
    Hybrid,
}

#[derive(Clone, Debug)]
pub struct PluginManifest {
    pub id: PluginId,
    pub name: &'static str,
    pub version: &'static str,
    pub kind: PluginKind,
    pub permissions: &'static [PluginPermission],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginLifecycle {
    Registered,
    Loaded,
    Active,
    Suspended,
    Failed(String),
    Stopped,
}

pub type PluginResult = Result<(), String>;

pub trait Plugin {
    fn manifest(&self) -> PluginManifest;

    fn on_load(&mut self) -> PluginResult {
        Ok(())
    }

    fn on_activate(&mut self) -> PluginResult {
        Ok(())
    }

    fn on_suspend(&mut self) -> PluginResult {
        Ok(())
    }

    fn on_shutdown(&mut self) -> PluginResult {
        Ok(())
    }
}
