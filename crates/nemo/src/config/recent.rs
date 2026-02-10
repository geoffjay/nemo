use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::debug;

const MAX_RECENT_PROJECTS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub name: String,
    pub config_path: PathBuf,
    pub last_opened: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    pub projects: Vec<RecentProject>,
}

impl RecentProjects {
    /// Load from the state directory (falls back to data directory).
    pub fn load() -> Self {
        Self::file_path()
            .and_then(|path| {
                if path.exists() {
                    std::fs::read_to_string(&path).ok()
                } else {
                    None
                }
            })
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save to the state directory.
    pub fn save(&self) {
        let Some(path) = Self::file_path() else {
            debug!("Could not determine state directory for recent projects");
            return;
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                debug!("Failed to create recent projects directory: {}", e);
                return;
            }
        }

        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    debug!("Failed to write recent projects: {}", e);
                }
            }
            Err(e) => debug!("Failed to serialize recent projects: {}", e),
        }
    }

    /// Add or update a project entry. Keeps at most MAX_RECENT_PROJECTS entries,
    /// sorted by last_opened descending.
    pub fn add(&mut self, config_path: PathBuf) {
        let name = config_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                config_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            });

        // Remove existing entry for this path
        self.projects
            .retain(|p| p.config_path != config_path);

        self.projects.insert(
            0,
            RecentProject {
                name,
                config_path,
                last_opened: Utc::now(),
            },
        );

        self.projects.truncate(MAX_RECENT_PROJECTS);
    }

    /// Return up to MAX_RECENT_PROJECTS recent projects, sorted by last_opened descending.
    pub fn list(&self) -> &[RecentProject] {
        &self.projects
    }

    fn file_path() -> Option<PathBuf> {
        dirs::state_dir()
            .or_else(dirs::data_dir)
            .map(|p| p.join("nemo").join("recent_projects.json"))
    }
}
