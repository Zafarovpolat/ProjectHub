//! Append-only JSON-Lines event log.
//!
//! Lives at `%APPDATA%\ProjectHub\events.jsonl`. Every line is one
//! `Event` JSON object. Designed for cheap appends (one open/append/close per
//! event) so we can crash-safe-ish without locking. The log is consumed by
//! v2+ AI features that read recent events to answer questions like
//! "сколько я работал над проектом X сегодня?".
//!
//! Rotation: when the active file exceeds `MAX_BYTES`, it is renamed to
//! `events.jsonl.<timestamp>` and a fresh file is opened.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub kind: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    AppStarted,
    AppShutdown,
    ProjectCreated {
        name: String,
    },
    ProjectDeleted {
        name: String,
    },
    ProjectUpdated {
        name: String,
    },
    ProjectActivated {
        name: String,
        from: Option<Uuid>,
        duration_in_prev_ms: Option<u64>,
        windows_focused: usize,
        windows_minimized: usize,
    },
    WindowReattached {
        project_name: String,
        title: String,
    },
    WindowMissing {
        project_name: String,
        title: String,
    },
    HotkeyTriggered {
        combo: String,
    },
    DockToggled {
        visible: bool,
    },
}

#[derive(Clone)]
pub struct EventLog {
    path: PathBuf,
    /// Serialised access so two threads can't interleave writes mid-line.
    write_lock: Arc<Mutex<()>>,
}

impl EventLog {
    pub fn default_path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("config_dir")?;
        Ok(dir.join("ProjectHub").join("events.jsonl"))
    }

    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self {
            path,
            write_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn append(&self, kind: EventKind, project_id: Option<Uuid>) {
        let event = Event {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            kind,
            project_id,
        };
        if let Err(err) = self.append_inner(&event) {
            // Logging is best-effort: never panic the app for a log write.
            tracing::warn!(?err, "event log append failed");
        }
    }

    fn append_inner(&self, event: &Event) -> Result<()> {
        let _guard = self.write_lock.lock();
        self.maybe_rotate()?;
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let mut line = serde_json::to_string(event)?;
        line.push('\n');
        file.write_all(line.as_bytes())?;
        Ok(())
    }

    fn maybe_rotate(&self) -> Result<()> {
        if let Ok(meta) = fs::metadata(&self.path) {
            if meta.len() >= MAX_BYTES {
                let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
                let rotated = path_with_suffix(&self.path, &stamp);
                fs::rename(&self.path, &rotated).ok();
            }
        }
        Ok(())
    }

    /// Read up to `limit` most recent events from the active log file.
    /// Linear scan; fine for the scales we expect (~50-200 events/day).
    pub fn read_recent(&self, limit: usize) -> Result<Vec<Event>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&self.path)?;
        let mut events: Vec<Event> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Event>(l).ok())
            .collect();
        events.reverse();
        events.truncate(limit);
        Ok(events)
    }
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".");
    s.push(suffix);
    PathBuf::from(s)
}
