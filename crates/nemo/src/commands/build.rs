//! `nemo build` — compile a project ahead-of-time.
//!
//! Phase 0 is a **dry run**: it resolves the project root (the nearest
//! `nemo.toml` walking up from the target), parses the manifest, and prints the
//! build plan. The actual ahead-of-time compilation — running the same
//! parse → SFC-register → style-fold → tag/handler-rewrite → expand pipeline the
//! runtime uses at startup, then serializing the resolved tree to `<out>/` — lands
//! in later phases. See the build-system plan.

use anyhow::{Context, Result};
use nemo_config::{find_project_root, ProjectManifest, MANIFEST_FILE};

use crate::args::BuildArgs;

pub fn run(args: BuildArgs) -> Result<()> {
    // The search starts at the explicit target, else the current directory.
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
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest = ProjectManifest::load(&manifest_path)
        .with_context(|| format!("loading {}", manifest_path.display()))?;

    let entry = root.join(&manifest.entry);
    let out = root.join(&manifest.build.out);

    println!("Build plan (dry run — Phase 0)");
    println!("  project:  {}", manifest.name);
    println!("  root:     {}", root.display());
    println!("  entry:    {}", entry.display());
    println!("  output:   {}", out.display());
    println!("  load:     {:?}", manifest.build.load);
    if !manifest.dependencies.is_empty() {
        println!("  dependencies:");
        for (module, version) in &manifest.dependencies {
            println!("    {module} = {version}");
        }
    }

    if !entry.is_file() {
        eprintln!(
            "\nwarning: entry file {} does not exist yet",
            entry.display()
        );
    }

    println!(
        "\nnote: ahead-of-time compilation is not implemented yet; Phase 0 resolves \
         the manifest and prints this plan only."
    );

    Ok(())
}
