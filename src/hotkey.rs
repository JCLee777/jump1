//! Global launcher shortcut and user-defined shortcut aliases.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui_kit::Keystroke;

use crate::plugin::shortcuts::ShortcutBinding;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutAction {
    ToggleLauncher,
    RunBinding(u64),
}

pub struct GlobalShortcuts {
    manager: GlobalHotKeyManager,
    launcher: HotKey,
    bindings: Vec<HotKey>,
    actions: Arc<RwLock<HashMap<u32, u64>>>,
    conflicts: Vec<u64>,
}

impl GlobalShortcuts {
    pub fn register(bindings: &[ShortcutBinding]) -> Result<Self, global_hotkey::Error> {
        let manager = GlobalHotKeyManager::new()?;
        let launcher = HotKey::new(Some(Modifiers::ALT), Code::Space);
        manager.register(launcher)?;
        let mut shortcuts = Self {
            manager,
            launcher,
            bindings: Vec::new(),
            actions: Arc::new(RwLock::new(HashMap::new())),
            conflicts: Vec::new(),
        };
        shortcuts.set_bindings(bindings);
        Ok(shortcuts)
    }

    pub fn launcher_id(&self) -> u32 {
        self.launcher.id()
    }

    pub fn actions(&self) -> Arc<RwLock<HashMap<u32, u64>>> {
        self.actions.clone()
    }

    pub fn conflicts(&self) -> &[u64] {
        &self.conflicts
    }

    pub fn set_bindings(&mut self, bindings: &[ShortcutBinding]) {
        for hotkey in self.bindings.drain(..) {
            let _ = self.manager.unregister(hotkey);
        }
        let mut actions = HashMap::new();
        let mut conflicts = Vec::new();
        for binding in bindings {
            if binding.shortcut.is_empty() {
                continue;
            }
            let Ok(hotkey) = binding.shortcut.parse::<HotKey>() else {
                conflicts.push(binding.id);
                continue;
            };
            if hotkey.id() == self.launcher.id() || actions.contains_key(&hotkey.id()) {
                conflicts.push(binding.id);
                continue;
            }
            if self.manager.register(hotkey).is_ok() {
                actions.insert(hotkey.id(), binding.id);
                self.bindings.push(hotkey);
            } else {
                conflicts.push(binding.id);
            }
        }
        self.conflicts = conflicts;
        if let Ok(mut current) = self.actions.write() {
            *current = actions;
        }
    }
}

pub fn take_action(
    launcher_id: u32,
    actions: &RwLock<HashMap<u32, u64>>,
) -> Option<ShortcutAction> {
    GlobalHotKeyEvent::receiver().try_iter().find_map(|event| {
        if event.state != HotKeyState::Pressed {
            return None;
        }
        if event.id == launcher_id {
            return Some(ShortcutAction::ToggleLauncher);
        }
        actions
            .read()
            .ok()?
            .get(&event.id)
            .copied()
            .map(ShortcutAction::RunBinding)
    })
}

pub fn recorded_shortcut(keystroke: &Keystroke) -> Option<String> {
    let key = key_token(&keystroke.key)?;
    let mut parts = Vec::new();
    if keystroke.modifiers.shift {
        parts.push("shift");
    }
    if keystroke.modifiers.control {
        parts.push("control");
    }
    if keystroke.modifiers.alt {
        parts.push("alt");
    }
    if keystroke.modifiers.platform {
        parts.push("super");
    }
    if parts.is_empty() {
        return None;
    }
    parts.push(&key);
    parts
        .join("+")
        .parse::<HotKey>()
        .ok()
        .map(|key| key.to_string())
}

pub fn shortcut_label(value: &str) -> String {
    let Ok(hotkey) = value.parse::<HotKey>() else {
        return value.to_owned();
    };
    let mut parts = Vec::new();
    if cfg!(target_os = "macos") {
        if hotkey.mods.contains(Modifiers::CONTROL) {
            parts.push("⌃".into());
        }
        if hotkey.mods.contains(Modifiers::ALT) {
            parts.push("⌥".into());
        }
        if hotkey.mods.contains(Modifiers::SHIFT) {
            parts.push("⇧".into());
        }
        if hotkey.mods.contains(Modifiers::SUPER) {
            parts.push("⌘".into());
        }
    } else {
        if hotkey.mods.contains(Modifiers::CONTROL) {
            parts.push("Ctrl".into());
        }
        if hotkey.mods.contains(Modifiers::ALT) {
            parts.push("Alt".into());
        }
        if hotkey.mods.contains(Modifiers::SHIFT) {
            parts.push("Shift".into());
        }
        if hotkey.mods.contains(Modifiers::SUPER) {
            parts.push("Super".into());
        }
    }
    parts.push(key_label(hotkey.key));
    parts.join(if cfg!(target_os = "macos") { "" } else { "+" })
}

fn key_token(key: &str) -> Option<String> {
    let normalized = key.to_ascii_lowercase();
    if normalized.len() == 1 {
        let character = normalized.chars().next()?;
        if character.is_ascii_alphabetic() {
            return Some(format!("Key{}", character.to_ascii_uppercase()));
        }
        if character.is_ascii_digit() {
            return Some(format!("Digit{character}"));
        }
    }
    match normalized.as_str() {
        "space" => Some("Space".into()),
        "enter" => Some("Enter".into()),
        "tab" => Some("Tab".into()),
        "/" => Some("Slash".into()),
        "." => Some("Period".into()),
        "," => Some("Comma".into()),
        "-" => Some("Minus".into()),
        "=" => Some("Equal".into()),
        _ => None,
    }
}

fn key_label(code: Code) -> String {
    let value = code.to_string();
    value
        .strip_prefix("Key")
        .or_else(|| value.strip_prefix("Digit"))
        .unwrap_or(&value)
        .to_owned()
}
