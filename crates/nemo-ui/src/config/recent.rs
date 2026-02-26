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
        self.projects.retain(|p| p.config_path != config_path);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let recent = RecentProjects::default();
        assert!(recent.projects.is_empty());
        assert!(recent.list().is_empty());
    }

    #[test]
    fn test_add_single_project() {
        let mut recent = RecentProjects::default();
        recent.add(PathBuf::from("/home/user/project/app.xml"));
        assert_eq!(recent.projects.len(), 1);
        assert_eq!(
            recent.projects[0].config_path,
            PathBuf::from("/home/user/project/app.xml")
        );
        assert_eq!(recent.projects[0].name, "project");
    }

    #[test]
    fn test_add_derives_name_from_parent_dir() {
        let mut recent = RecentProjects::default();
        recent.add(PathBuf::from("/workspace/my-dashboard/app.xml"));
        assert_eq!(recent.projects[0].name, "my-dashboard");
    }

    #[test]
    fn test_add_deduplicates() {
        let mut recent = RecentProjects::default();
        let path = PathBuf::from("/home/user/project/app.xml");
        recent.add(path.clone());
        recent.add(path.clone());
        assert_eq!(recent.projects.len(), 1);
    }

    #[test]
    fn test_add_moves_duplicate_to_front() {
        let mut recent = RecentProjects::default();
        recent.add(PathBuf::from("/a/app.xml"));
        recent.add(PathBuf::from("/b/app.xml"));
        recent.add(PathBuf::from("/a/app.xml")); // re-add first

        assert_eq!(recent.projects.len(), 2);
        assert_eq!(recent.projects[0].config_path, PathBuf::from("/a/app.xml"));
        assert_eq!(recent.projects[1].config_path, PathBuf::from("/b/app.xml"));
    }

    #[test]
    fn test_add_truncates_at_max() {
        let mut recent = RecentProjects::default();
        for i in 0..10 {
            recent.add(PathBuf::from(format!("/project{}/app.xml", i)));
        }
        assert_eq!(recent.projects.len(), MAX_RECENT_PROJECTS);
        // Most recent should be first
        assert_eq!(
            recent.projects[0].config_path,
            PathBuf::from("/project9/app.xml")
        );
    }

    #[test]
    fn test_list_returns_all() {
        let mut recent = RecentProjects::default();
        recent.add(PathBuf::from("/a/app.xml"));
        recent.add(PathBuf::from("/b/app.xml"));
        assert_eq!(recent.list().len(), 2);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut recent = RecentProjects::default();
        recent.add(PathBuf::from("/test/project/app.xml"));

        let json = serde_json::to_string(&recent).unwrap();
        let deserialized: RecentProjects = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.projects.len(), 1);
        assert_eq!(
            deserialized.projects[0].config_path,
            PathBuf::from("/test/project/app.xml")
        );
        assert_eq!(deserialized.projects[0].name, "project");
    }
}
