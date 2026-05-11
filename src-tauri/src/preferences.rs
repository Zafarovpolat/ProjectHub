//! User preferences — app-wide settings that don't belong to any
//! individual project.
//!
//! Persisted at `%APPDATA%\ProjectHub\preferences.json` next to
//! `projects.json`. Currently holds only the customisable dock-toggle
//! hotkey; intentionally tiny so adding new fields stays additive.

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default dock-toggle combo. Format mirrors
/// `tauri_plugin_global_shortcut::Shortcut`'s `FromStr` (e.g.
/// `Ctrl+Alt+Space`, `Ctrl+Shift+KeyM`, `Meta+F2`).
pub const DEFAULT_DOCK_TOGGLE_COMBO: &str = "Ctrl+Alt+Space";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesFile {
    pub version: u32,
    /// User-chosen combo for `toggle_dock`. When `None`, falls back to
    /// [`DEFAULT_DOCK_TOGGLE_COMBO`].
    #[serde(default)]
    pub dock_toggle_hotkey: Option<String>,
}

impl Default for PreferencesFile {
    fn default() -> Self {
        Self {
            version: 1,
            dock_toggle_hotkey: None,
        }
    }
}

#[derive(Clone)]
pub struct PreferencesStore {
    path: PathBuf,
    inner: Arc<RwLock<PreferencesFile>>,
}

impl PreferencesStore {
    pub fn default_path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("failed to resolve OS config dir")?;
        Ok(dir.join("ProjectHub").join("preferences.json"))
    }

    pub fn load_or_init(path: PathBuf) -> Result<Self> {
        let inner = if path.exists() {
            let bytes = fs::read(&path)
                .with_context(|| format!("read {}", path.display()))?;
            serde_json::from_slice::<PreferencesFile>(&bytes)
                .with_context(|| format!("parse {}", path.display()))?
        } else {
            PreferencesFile::default()
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        Ok(Self {
            path,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub fn snapshot(&self) -> PreferencesFile {
        self.inner.read().clone()
    }

    /// Returns the effective dock-toggle combo: the user override if
    /// set, otherwise [`DEFAULT_DOCK_TOGGLE_COMBO`].
    pub fn dock_toggle_combo(&self) -> String {
        self.inner
            .read()
            .dock_toggle_hotkey
            .clone()
            .unwrap_or_else(|| DEFAULT_DOCK_TOGGLE_COMBO.to_string())
    }

    pub fn set_dock_toggle_hotkey(&self, combo: Option<String>) -> Result<()> {
        {
            let mut guard = self.inner.write();
            guard.dock_toggle_hotkey = combo.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
        }
        self.flush()
    }

    fn flush(&self) -> Result<()> {
        let bytes = {
            let guard = self.inner.read();
            serde_json::to_vec_pretty(&*guard)?
        };
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&self.path, &bytes)
            .with_context(|| format!("write {}", self.path.display()))
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
