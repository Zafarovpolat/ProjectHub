//! Domain types for ProjectHub.
//!
//! A `Project` is a named bundle of external windows the user wants to treat
//! as a single workspace. The hub remembers each window by `(exe_path,
//! title_pattern, class_name)` so it can re-discover the actual HWND after
//! windows are reopened across reboots.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Reference to a single external top-level window that belongs to a project.
///
/// HWNDs are not stable across process restarts, so we store the descriptive
/// fields (exe, title pattern, class name) and re-discover the live HWND at
/// runtime via `EnumWindows`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRef {
    pub id: Uuid,
    /// Title at the moment the window was attached. Used as a substring
    /// pattern for re-discovery.
    pub title_snapshot: String,
    /// Substring pattern actually used for matching. Defaults to a normalized
    /// version of the title snapshot but the user can edit it later.
    pub title_pattern: String,
    /// Full path to the owning executable.
    pub exe_path: String,
    /// Win32 class name of the window, when available.
    pub class_name: Option<String>,
    /// For Chrome: extracted profile-directory hint, when detectable.
    /// (Not implemented in v0.1; reserved.)
    pub chrome_profile: Option<String>,
    /// Last HWND value we associated with this window. Volatile; not used
    /// across process restarts.
    #[serde(skip)]
    pub last_seen_hwnd: Option<isize>,
    /// Whether the window is currently live (re-bindable to a real HWND).
    /// Volatile in-memory state, updated by the background pruner.
    #[serde(skip, default = "default_live")]
    pub live: bool,
    /// Number of consecutive pruner ticks the window has been missing.
    /// When this reaches `GRACE_PERIOD_TICKS`, the window is auto-removed.
    #[serde(skip)]
    pub missed_ticks: u8,
}

fn default_live() -> bool {
    true
}

impl WindowRef {
    pub fn new(title: String, exe_path: String, class_name: Option<String>, hwnd: isize) -> Self {
        let normalized = title.trim().to_string();
        Self {
            id: Uuid::new_v4(),
            title_pattern: normalized.clone(),
            title_snapshot: normalized,
            exe_path,
            class_name,
            chrome_profile: None,
            last_seen_hwnd: Some(hwnd),
            live: true,
            missed_ticks: 0,
        }
    }
}

/// A command the hub can execute to launch a missing window. Reserved for
/// v0.2 — captured here so the JSON shape is stable from v0.1 onward.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaunchCommand {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// A user-defined project: a name + a set of windows + (optional) launch hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    /// Hex accent colour (e.g. `#6366f1`).
    pub color: String,
    /// 1-2 character identifier shown inside the icon tile.
    pub initials: String,
    /// Hotkey index (1-based), or `None` if no hotkey is bound.
    pub hotkey_index: Option<u8>,
    pub windows: Vec<WindowRef>,
    #[serde(default)]
    pub launch_commands: Vec<LaunchCommand>,
    /// `tg://resolve?...` URI for jumping to a chat. Reserved for v0.2.
    #[serde(default)]
    pub tg_chat_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// When the project was last activated. Used for "5m ago" badges.
    #[serde(default)]
    pub last_activated_at: Option<DateTime<Utc>>,
}

impl Project {
    pub fn new(name: String, color: String, windows: Vec<WindowRef>) -> Self {
        let now = Utc::now();
        let initials = derive_initials(&name);
        Self {
            id: Uuid::new_v4(),
            name,
            color,
            initials,
            hotkey_index: None,
            windows,
            launch_commands: Vec::new(),
            tg_chat_uri: None,
            created_at: now,
            updated_at: now,
            last_activated_at: None,
        }
    }
}

fn derive_initials(name: &str) -> String {
    let mut chars: Vec<char> = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .collect();
    chars.truncate(2);
    if chars.is_empty() {
        "·".to_string()
    } else {
        chars.iter().collect::<String>().to_uppercase()
    }
}

/// Pretty palette assigned to projects in round-robin fashion when the user
/// doesn't explicitly choose a colour.
pub const DEFAULT_PALETTE: &[&str] = &[
    "#6366f1", // indigo
    "#22c55e", // green
    "#f97316", // orange
    "#a855f7", // violet
    "#06b6d4", // cyan
    "#ef4444", // red
    "#eab308", // amber
    "#ec4899", // pink
];

pub fn palette_for_index(index: usize) -> String {
    DEFAULT_PALETTE[index % DEFAULT_PALETTE.len()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_two_words() {
        assert_eq!(derive_initials("Devin Task"), "DT");
    }

    #[test]
    fn initials_one_word() {
        assert_eq!(derive_initials("Личное"), "Л");
    }

    #[test]
    fn initials_empty() {
        assert_eq!(derive_initials(""), "·");
    }
}
