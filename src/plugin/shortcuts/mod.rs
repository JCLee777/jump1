//! Global shortcut aliases that feed text into the launcher.

use crate::plugin::{
    PluginId,
    api::{Plugin, PluginKind, PluginManifest, PluginPermission},
    storage::PluginStorage,
};

const BINDINGS_FILE: &str = "bindings.conf";

#[derive(Clone, Debug)]
pub struct ShortcutBinding {
    pub id: u64,
    pub shortcut: String,
    pub query: String,
    pub auto_copy: bool,
}

pub struct ShortcutPlugin {
    bindings: Vec<ShortcutBinding>,
    next_id: u64,
    storage: Option<PluginStorage>,
}

impl ShortcutPlugin {
    pub fn load(storage: Option<PluginStorage>) -> Self {
        let bindings = storage
            .as_ref()
            .and_then(|storage| storage.read_text(BINDINGS_FILE))
            .map(|value| decode_bindings(&value))
            .unwrap_or_default();
        let next_id = bindings
            .iter()
            .map(|binding| binding.id)
            .max()
            .unwrap_or_default()
            .saturating_add(1);
        Self {
            bindings,
            next_id,
            storage,
        }
    }

    pub fn bindings(&self) -> &[ShortcutBinding] {
        &self.bindings
    }

    pub fn binding(&self, id: u64) -> Option<&ShortcutBinding> {
        self.bindings.iter().find(|binding| binding.id == id)
    }

    pub fn add_binding(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.bindings.push(ShortcutBinding {
            id,
            shortcut: String::new(),
            query: String::new(),
            auto_copy: false,
        });
        self.persist();
        id
    }

    pub fn set_shortcut(&mut self, id: u64, shortcut: String) -> bool {
        if self
            .bindings
            .iter()
            .any(|binding| binding.id != id && !shortcut.is_empty() && binding.shortcut == shortcut)
        {
            return false;
        }
        let Some(binding) = self.bindings.iter_mut().find(|binding| binding.id == id) else {
            return false;
        };
        binding.shortcut = shortcut;
        self.persist();
        true
    }

    pub fn set_query(&mut self, id: u64, query: String) -> bool {
        let Some(binding) = self.bindings.iter_mut().find(|binding| binding.id == id) else {
            return false;
        };
        binding.query = query;
        self.persist();
        true
    }

    pub fn set_auto_copy(&mut self, id: u64, auto_copy: bool) -> bool {
        let Some(binding) = self.bindings.iter_mut().find(|binding| binding.id == id) else {
            return false;
        };
        binding.auto_copy = auto_copy;
        self.persist();
        true
    }

    pub fn remove_binding(&mut self, id: u64) -> bool {
        let before = self.bindings.len();
        self.bindings.retain(|binding| binding.id != id);
        if self.bindings.len() == before {
            return false;
        }
        self.persist();
        true
    }

    fn persist(&self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let contents = self
            .bindings
            .iter()
            .map(|binding| {
                format!(
                    "{}\t{}\t{}\t{}",
                    binding.id,
                    encode(binding.shortcut.as_bytes()),
                    encode(binding.query.as_bytes()),
                    u8::from(binding.auto_copy),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let _ = storage.write_text_atomic(BINDINGS_FILE, &contents);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutBindingEvent {
    Add,
    Record(u64),
    EditQuery(u64),
    SetAutoCopy(u64, bool),
    Remove(u64),
}

fn decode_bindings(contents: &str) -> Vec<ShortcutBinding> {
    contents
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let id = fields.next()?.parse().ok()?;
            let shortcut = String::from_utf8(decode(fields.next()?)?).ok()?;
            let query = String::from_utf8(decode(fields.next()?)?).ok()?;
            let auto_copy = fields.next() == Some("1");
            Some(ShortcutBinding {
                id,
                shortcut,
                query,
                auto_copy,
            })
        })
        .collect()
}

fn encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)? as u8;
            let low = (pair[1] as char).to_digit(16)? as u8;
            Some((high << 4) | low)
        })
        .collect()
}

impl Plugin for ShortcutPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: PluginId::Shortcuts,
            name: "Shortcut Aliases",
            version: "0.1.0",
            kind: PluginKind::Service,
            permissions: &[
                PluginPermission::ProcessLaunch,
                PluginPermission::InputSimulation,
            ],
        }
    }
}
