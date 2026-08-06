//! `nemo new` — scaffold a new project from an embedded template.
//!
//! Templates live in `crates/nemo/templates/<name>/` and are embedded into the
//! binary at build time with `include_str!`, so scaffolding works from a
//! standalone `nemo` with no dependency on the repository (and without adding a
//! crate dependency). Template text files may contain the `{{PROJECT_NAME}}`
//! placeholder, which is replaced with the project name.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::args::NewArgs;

/// One file within a template: destination-relative path + embedded contents.
struct TemplateFile {
    path: &'static str,
    contents: &'static str,
}

/// An embedded project template.
struct Template {
    name: &'static str,
    files: &'static [TemplateFile],
}

/// Placeholder substituted with the project name in template files.
const PLACEHOLDER: &str = "{{PROJECT_NAME}}";

/// Concisely declare a template file embedded from `templates/<...>`.
macro_rules! tfile {
    ($path:literal) => {
        TemplateFile {
            path: $path,
            contents: include_str!(concat!("../../templates/", $path)),
        }
    };
}

static TEMPLATES: &[Template] = &[
    Template {
        name: "basic",
        files: &[
            tfile!("basic/app.nemo"),
            tfile!("basic/nemo.toml"),
            tfile!("basic/scripts/handlers.rhai"),
        ],
    },
    Template {
        name: "calculator",
        files: &[
            tfile!("calculator/app.nemo"),
            tfile!("calculator/nemo.toml"),
            tfile!("calculator/scripts/handlers.rhai"),
        ],
    },
    Template {
        name: "data-binding",
        files: &[
            tfile!("data-binding/app.nemo"),
            tfile!("data-binding/nemo.toml"),
            tfile!("data-binding/scripts/handlers.rhai"),
            tfile!("data-binding/docker-compose.yml"),
            tfile!("data-binding/mosquitto.conf"),
        ],
    },
    Template {
        name: "complete",
        files: &[
            tfile!("complete/app.nemo"),
            tfile!("complete/scripts/handlers.rhai"),
            tfile!("complete/scripts/transforms.rhai"),
            tfile!("complete/templates/nav.xml"),
            tfile!("complete/templates/cards.xml"),
            tfile!("complete/templates/data.xml"),
        ],
    },
];

pub fn run(args: NewArgs) -> Result<()> {
    if args.list {
        println!("Available templates:");
        for template in TEMPLATES {
            println!("  {}", template.name);
        }
        return Ok(());
    }

    let Some(target) = args.name.clone() else {
        bail!("`nemo new` requires a project name, e.g. `nemo new my-app` (see `nemo new --list`)");
    };

    let template = TEMPLATES
        .iter()
        .find(|t| t.name == args.template)
        .with_context(|| {
            format!(
                "unknown template '{}'. Available: {}",
                args.template,
                TEMPLATES
                    .iter()
                    .map(|t| t.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    // Human-facing project name derived from the final path component.
    let project_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(args.template.as_str())
        .to_string();

    // Refuse to scaffold into a non-empty directory unless --force is given.
    if target.exists() {
        let non_empty = std::fs::read_dir(&target)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
        if non_empty && !args.force {
            bail!(
                "target directory '{}' already exists and is not empty (use --force to overwrite)",
                target.display()
            );
        }
    }

    std::fs::create_dir_all(&target)
        .with_context(|| format!("failed to create '{}'", target.display()))?;

    // The embedded paths are `<template>/<rel>`; strip the template prefix.
    let prefix = format!("{}/", template.name);
    for file in template.files {
        let rel = file.path.strip_prefix(&prefix).unwrap_or(file.path);
        let dest = target.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        let rendered = file.contents.replace(PLACEHOLDER, &project_name);
        std::fs::write(&dest, rendered)
            .with_context(|| format!("failed to write '{}'", dest.display()))?;
    }
    scaffold_extras(&target, &project_name, &args.template)?;

    println!("\nCreated Nemo project at {}", target.display());
    println!("\nNext steps:");
    println!("  cd {}", target.display());
    // The `complete` template still uses app.xml (multi-file <include>); the
    // others are .nemo SFC entries.
    let entry = if args.template == "complete" {
        "app.xml"
    } else {
        "app.nemo"
    };
    println!("  nemo dev --app-config {entry}");
    Ok(())
}

/// Create the files common to every scaffold that are not part of a template
/// body: a `plugins/` directory, a `.gitignore`, and a generated `README.md`.
fn scaffold_extras(dest: &Path, project_name: &str, template: &str) -> Result<()> {
    let plugins = dest.join("plugins");
    std::fs::create_dir_all(&plugins)?;
    std::fs::write(plugins.join(".gitkeep"), "")?;

    let gitignore = dest.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, "/target\n.DS_Store\n*.log\n")?;
    }

    let readme = dest.join("README.md");
    if !readme.exists() {
        std::fs::write(&readme, render_readme(project_name, template))?;
    }
    Ok(())
}
fn render_readme(project_name: &str, template: &str) -> String {
    let entry = if template == "complete" {
        "app.xml"
    } else {
        "app.nemo"
    };
    format!(
        "# {project_name}\n\n\
         A [Nemo](https://github.com/geoffjay/nemo) application scaffolded from the \
         `{template}` template.\n\n\
         ## Run\n\n\
         ```bash\n\
         nemo dev --app-config {entry}\n\
         ```\n\n\
         `nemo dev` hot-reloads the app when you edit `{entry}` or files under `scripts/`.\n\n\
         ## Validate\n\n\
         ```bash\n\
         nemo validate {entry} --strict\n\
         ```\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::NewArgs;
    use nemo_config::{ConfigurationLoader, SchemaRegistry};

    fn new_args(target: &Path, template: &str, force: bool) -> NewArgs {
        NewArgs {
            name: Some(target.to_path_buf()),
            template: template.to_string(),
            force,
            list: false,
        }
    }

    #[test]
    fn scaffolds_and_validates_every_template() {
        for template in TEMPLATES {
            let tmp = tempfile::tempdir().unwrap();
            let proj = tmp.path().join("proj");
            run(new_args(&proj, template.name, false)).unwrap();

            let name = template.name;
            let entry = proj.join("app.nemo");
            assert!(entry.exists(), "{name}: no app.nemo");
            assert!(proj.join("README.md").exists(), "{name}: no README");
            assert!(proj.join(".gitignore").exists(), "{name}: no .gitignore");
            assert!(
                proj.join("plugins/.gitkeep").exists(),
                "{name}: no plugins/.gitkeep"
            );

            let xml = std::fs::read_to_string(&entry).unwrap();
            assert!(
                !xml.contains(PLACEHOLDER),
                "{name}: placeholder not substituted"
            );

            // The scaffolded config must parse + resolve (same path as `nemo validate`).
            let loader = ConfigurationLoader::new(std::sync::Arc::new(SchemaRegistry::new()));
            loader
                .load(&entry)
                .unwrap_or_else(|e| panic!("{name} scaffold failed to validate: {e}"));
        }
    }

    #[test]
    fn refuses_nonempty_dir_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("existing.txt"), "keep").unwrap();

        assert!(run(new_args(&proj, "basic", false)).is_err());
        // --force allows scaffolding into the non-empty directory.
        run(new_args(&proj, "basic", true)).unwrap();
        assert!(proj.join("app.nemo").exists());
    }

    #[test]
    fn unknown_template_errors() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(run(new_args(&tmp.path().join("p"), "does-not-exist", false)).is_err());
    }
}
