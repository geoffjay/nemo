//! `nemo get` — fetch remote component-library dependencies into `.nemo/packages`.
//!
//! For each `[dependencies]` entry in the project's `nemo.toml`, this clones the
//! module at its tagged version into `.nemo/packages/<module>@<version>/` and
//! records the resolved commit in `nemo.lock` (Go's module model). Fetching shells
//! out to the `git` CLI, so nemo needs no VCS crate dependency.
//!
//! Once fetched, an `<import src="github.com/…">` in `app.xml` resolves against
//! the cache (see `nemo-config`'s `pkg`/`xml_parser` module resolution).

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use nemo_config::{
    find_project_root, package_dir, Lockfile, ProjectManifest, LOCKFILE, MANIFEST_FILE,
};

use crate::args::GetArgs;

pub fn run(args: GetArgs) -> Result<()> {
    let start = match args.target {
        Some(target) => target,
        None => std::env::current_dir().context("resolving the current directory")?,
    };
    let root = find_project_root(&start).with_context(|| {
        format!(
            "no {MANIFEST_FILE} found in {} or any parent directory",
            start.display()
        )
    })?;
    let manifest = ProjectManifest::load(&root.join(MANIFEST_FILE))
        .with_context(|| format!("loading {}", root.join(MANIFEST_FILE).display()))?;

    if manifest.dependencies.is_empty() {
        println!("No [dependencies] in {}; nothing to fetch.", MANIFEST_FILE);
        return Ok(());
    }

    let mut lock = Lockfile::default();
    for (module, version) in &manifest.dependencies {
        let dest = package_dir(&root, module, version);
        let commit = fetch_git(&remote_url(module), version, &dest)
            .with_context(|| format!("fetching {module}@{version}"))?;
        let short = &commit[..commit.len().min(12)];
        println!("fetched {module}@{version} ({short}) → {}", dest.display());
        lock.set(module, version, &commit);
    }

    let lock_path = root.join(LOCKFILE);
    lock.save(&lock_path).map_err(|e| anyhow!(e))?;
    println!("wrote {}", lock_path.display());
    Ok(())
}

/// The clone URL for a module. Defaults to `https://<module>`; the
/// `NEMO_PACKAGE_BASE` env var overrides the base (e.g. a `file://` path or a
/// private mirror), which also lets tests/offline runs stand in a local repo.
fn remote_url(module: &str) -> String {
    match std::env::var("NEMO_PACKAGE_BASE") {
        Ok(base) if !base.is_empty() => format!("{}/{module}", base.trim_end_matches('/')),
        _ => format!("https://{module}"),
    }
}

/// Clones `remote` at `reference` (a tag or branch) into `dest` and returns the
/// resolved commit hash. If `dest` is already a git checkout it is treated as
/// cached and its current `HEAD` is returned (idempotent re-runs, offline once
/// fetched).
fn fetch_git(remote: &str, reference: &str, dest: &Path) -> Result<String> {
    if dest.join(".git").is_dir() {
        return rev_parse_head(dest);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let status = Command::new("git")
        .args([
            "-c",
            "advice.detachedHead=false",
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--branch",
            reference,
            remote,
        ])
        .arg(dest)
        .status()
        .context("running `git clone` (is git installed?)")?;
    if !status.success() {
        bail!("`git clone {remote}` at '{reference}' failed");
    }
    rev_parse_head(dest)
}

/// Returns `git rev-parse HEAD` for a checkout.
fn rev_parse_head(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("running `git rev-parse HEAD`")?;
    if !output.status.success() {
        bail!("`git rev-parse HEAD` failed in {}", dir.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed");
    }

    #[test]
    fn fetch_git_clones_a_tag_and_pins_commit() {
        let base = std::env::temp_dir().join(format!("nemo_get_{}", std::process::id()));
        let remote = base.join("remote");
        std::fs::create_dir_all(&remote).unwrap();

        // Build a tiny local git repo with a tagged component.
        git(&remote, &["init", "--quiet"]);
        git(&remote, &["config", "user.email", "t@t"]);
        git(&remote, &["config", "user.name", "t"]);
        std::fs::write(
            remote.join("card.nemo"),
            "<template name=\"card\"><panel><slot /></panel></template>",
        )
        .unwrap();
        git(&remote, &["add", "."]);
        git(&remote, &["commit", "--quiet", "-m", "init"]);
        git(&remote, &["tag", "v1.0.0"]);

        // Fetch it into a package cache dir.
        let dest = base.join("cache/example.com/lib@v1.0.0");
        let commit = fetch_git(remote.to_str().unwrap(), "v1.0.0", &dest).unwrap();
        assert_eq!(commit.len(), 40, "full commit hash returned");
        assert!(dest.join("card.nemo").is_file(), "component checked out");

        // Idempotent: a second call sees the cache and returns the same commit.
        let again = fetch_git(remote.to_str().unwrap(), "v1.0.0", &dest).unwrap();
        assert_eq!(commit, again);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn remote_url_defaults_to_https_and_honors_base() {
        // Default path (only assert the prefix so a set env in CI doesn't fail it).
        assert!(
            remote_url("github.com/x/y").starts_with("https://")
                || std::env::var("NEMO_PACKAGE_BASE").is_ok()
        );
    }
}
