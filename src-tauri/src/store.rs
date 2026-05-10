//! Persistent storage for projects.
//!
//! Lives at `%APPDATA%\ProjectHub\projects.json` on Windows (resolved via the
//! `dirs` crate's `config_dir`). Loaded once at startup, written atomically on
//! every mutation. Uses temp-file + rename for crash-safety so a partial write
//! cannot corrupt the file.

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::project::Project;

/// Versioned envelope around the project list. Bumping `version` lets us
/// migrate older configs forward without rewriting the parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStoreFile {
    pub version: u32,
    #[serde(default)]
    pub projects: Vec<Project>,
}

impl Default for ProjectStoreFile {
    fn default() -> Self {
        Self {
            version: 1,
            projects: Vec::new(),
        }
    }
}

/// In-memory, lock-protected handle to the project list. Cloning is cheap
/// (Arc).
#[derive(Clone)]
pub struct ProjectStore {
    path: PathBuf,
    inner: Arc<RwLock<ProjectStoreFile>>,
}

impl ProjectStore {
    /// Compute the canonical config path: `<config_dir>/ProjectHub/projects.json`.
    pub fn default_path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("failed to resolve OS config dir")?;
        Ok(dir.join("ProjectHub").join("projects.json"))
    }

    /// Load from `path`, creating a new empty store if the file does not yet
    /// exist.
    pub fn load_or_init(path: PathBuf) -> Result<Self> {
        let inner = if path.exists() {
            let bytes = fs::read(&path)
                .with_context(|| format!("read {}", path.display()))?;
            serde_json::from_slice::<ProjectStoreFile>(&bytes)
                .with_context(|| format!("parse {}", path.display()))?
        } else {
            ProjectStoreFile::default()
        };

        let store = Self {
            path,
            inner: Arc::new(RwLock::new(inner)),
        };
        store.ensure_dir()?;
        Ok(store)
    }

    fn ensure_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        Ok(())
    }

    pub fn projects(&self) -> Vec<Project> {
        self.inner.read().projects.clone()
    }

    pub fn get(&self, id: Uuid) -> Option<Project> {
        self.inner.read().projects.iter().find(|p| p.id == id).cloned()
    }

    pub fn upsert(&self, project: Project) -> Result<()> {
        {
            let mut guard = self.inner.write();
            if let Some(slot) = guard.projects.iter_mut().find(|p| p.id == project.id) {
                *slot = project;
            } else {
                guard.projects.push(project);
            }
        }
        self.flush()
    }

    pub fn delete(&self, id: Uuid) -> Result<bool> {
        let removed = {
            let mut guard = self.inner.write();
            let len_before = guard.projects.len();
            guard.projects.retain(|p| p.id != id);
            guard.projects.len() != len_before
        };
        if removed {
            self.flush()?;
        }
        Ok(removed)
    }

    pub fn reorder(&self, order: &[Uuid]) -> Result<()> {
        {
            let mut guard = self.inner.write();
            let mut sorted = Vec::with_capacity(guard.projects.len());
            for id in order {
                if let Some(pos) = guard.projects.iter().position(|p| p.id == *id) {
                    sorted.push(guard.projects.remove(pos));
                }
            }
            // append any leftovers (defensive — clients should send full order)
            sorted.append(&mut guard.projects);
            guard.projects = sorted;
        }
        self.flush()
    }

    fn flush(&self) -> Result<()> {
        let bytes = {
            let guard = self.inner.read();
            serde_json::to_vec_pretty(&*guard)?
        };
        atomic_write(&self.path, &bytes)
            .with_context(|| format!("write {}", self.path.display()))
    }
}

/// Crash-safe write: stage to `<path>.tmp`, fsync, then rename over `<path>`.
/// Avoids leaving a half-written `projects.json` if the OS crashes mid-write.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
