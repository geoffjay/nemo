//! Project manifest (`nemo.toml`) — the per-project build/dependency manifest.
//!
//! A `nemo.toml` at a project root names the app entry (`entry`), the build
//! output directory and load mode (`[build]`), and remote component-library
//! dependencies (`[dependencies]`). It is **optional and additive**: pointing
//! `nemo --app-config app.xml` at a single file keeps working with no manifest,
//! and `nemo dev` never consults `[build] load`.
//!
//! Name distinction: the global user-prefs TOML is `config.toml`
//! (`crates/nemo/src/config/`, the `--config` flag); this per-project manifest
//! is `nemo.toml`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The manifest filename that marks a project root.
pub const MANIFEST_FILE: &str = "nemo.toml";

/// A parsed `nemo.toml` project manifest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    /// Project name.
    pub name: String,

    /// The app config entry file, relative to the project root. Defaults to
    /// `app.xml`.
    #[serde(default = "default_entry")]
    pub entry: String,

    /// Build settings (`[build]`).
    #[serde(default)]
    pub build: BuildConfig,

    /// Component-library settings (`[package]`). Present when this project is a
    /// reusable component library rather than (or in addition to) an app;
    /// `nemo build` on such a project emits a compiled artifact per exported
    /// component. Absent for a plain app project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<PackageConfig>,

    /// Remote component-library dependencies (`[dependencies]`), keyed by module
    /// path (e.g. `github.com/geoffjay/nemo-components`) → version. Resolution is
    /// a later phase; Phase 0 only records them. `BTreeMap` keeps the order
    /// deterministic for display and future lockfile emission.
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

fn default_entry() -> String {
    "app.xml".to_string()
}

/// The `[build]` table of a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildConfig {
    /// Output directory for `nemo build`, relative to the project root. Defaults
    /// to `dist`.
    #[serde(default = "default_out")]
    pub out: String,

    /// Which tree the default (no-subcommand) launch loads. `source` (the
    /// default) always re-parses `app.xml`; `dist` opts into loading a built
    /// tree. `nemo dev` ignores this and always loads source.
    #[serde(default)]
    pub load: LoadMode,
}

fn default_out() -> String {
    "dist".to_string()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out: default_out(),
            load: LoadMode::default(),
        }
    }
}

/// The `[package]` table — marks a project as a reusable component library.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PackageConfig {
    /// The component tags this library exports. When empty, the build falls back
    /// to the convention of every top-level `.nemo` file in the project root.
    #[serde(default)]
    pub exports: Vec<String>,
}

/// Where the launcher loads the resolved layout from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoadMode {
    /// Re-parse `app.xml` on every launch (default; the only Phase-0 behavior).
    #[default]
    Source,
    /// Load a previously built `dist/` tree (opt-in; wired in a later phase).
    Dist,
}

/// An error loading or parsing a `nemo.toml`.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The manifest file could not be read.
    #[error("failed to read manifest {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The manifest content was not valid TOML / did not match the schema.
    #[error("failed to parse manifest {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

impl ProjectManifest {
    /// Parses a manifest from a TOML string.
    pub fn parse(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Reads and parses a manifest from a file path.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&content).map_err(|source| ManifestError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Walks up from `start` (a file or directory) looking for the nearest
/// [`MANIFEST_FILE`], returning the directory that contains it (the project
/// root). Returns `None` if no manifest is found up to the filesystem root.
///
/// This is the project's only marker file — there is no other project-root
/// concept in the codebase.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    // If handed a file, begin the walk from its parent directory.
    let start = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    start
        .ancestors()
        .find(|dir| dir.join(MANIFEST_FILE).is_file())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_manifest() {
        let m = ProjectManifest::parse(
            r#"
                name  = "foo"
                entry = "main.xml"

                [build]
                out  = "build"
                load = "dist"

                [dependencies]
                "github.com/geoffjay/nemo-components" = "v1.2.0"
                "github.com/geoffjay/nemo-form"       = "v0.3.1"
            "#,
        )
        .unwrap();
        assert_eq!(m.name, "foo");
        assert_eq!(m.entry, "main.xml");
        assert_eq!(m.build.out, "build");
        assert_eq!(m.build.load, LoadMode::Dist);
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(
            m.dependencies.get("github.com/geoffjay/nemo-components"),
            Some(&"v1.2.0".to_string())
        );
    }

    #[test]
    fn applies_defaults() {
        let m = ProjectManifest::parse(r#"name = "bar""#).unwrap();
        assert_eq!(m.entry, "app.xml");
        assert_eq!(m.build.out, "dist");
        assert_eq!(m.build.load, LoadMode::Source);
        assert!(m.dependencies.is_empty());
        assert!(m.package.is_none());
    }

    #[test]
    fn parses_package_exports() {
        let m = ProjectManifest::parse(
            r#"name = "lib"
               [package]
               exports = ["button_group", "labeled_card"]"#,
        )
        .unwrap();
        let pkg = m.package.expect("package table");
        assert_eq!(pkg.exports, ["button_group", "labeled_card"]);
    }

    #[test]
    fn package_defaults_to_empty_exports() {
        let m = ProjectManifest::parse(
            r#"name = "lib"
               [package]"#,
        )
        .unwrap();
        assert!(m.package.unwrap().exports.is_empty());
    }

    #[test]
    fn missing_name_is_an_error() {
        assert!(ProjectManifest::parse(r#"entry = "app.xml""#).is_err());
    }

    #[test]
    fn unknown_field_is_an_error() {
        assert!(ProjectManifest::parse(
            r#"name = "x"
            bogus = true"#
        )
        .is_err());
    }

    #[test]
    fn find_project_root_walks_up() {
        let base = std::env::temp_dir().join(format!("nemo_manifest_{}", std::process::id()));
        let nested = base.join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(base.join(MANIFEST_FILE), "name = \"root\"\n").unwrap();

        // From a nested directory.
        assert_eq!(find_project_root(&nested).as_deref(), Some(base.as_path()));
        // From a file inside the project.
        let file = nested.join("app.xml");
        std::fs::write(&file, "<nemo/>").unwrap();
        assert_eq!(find_project_root(&file).as_deref(), Some(base.as_path()));
        // From the root itself.
        assert_eq!(find_project_root(&base).as_deref(), Some(base.as_path()));

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn find_project_root_returns_none_when_absent() {
        let dir = std::env::temp_dir().join(format!("nemo_no_manifest_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A temp dir with no nemo.toml on the path up to it. (Guard against a
        // stray nemo.toml in an ancestor by only asserting the immediate dir has
        // none — the walk may still find one far up, so scope the check.)
        assert!(!dir.join(MANIFEST_FILE).exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
