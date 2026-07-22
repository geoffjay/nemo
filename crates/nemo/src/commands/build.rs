//! `nemo build` — compile components ahead-of-time.
//!
//! The command handles three targets:
//!
//! * **A single `.nemo` file** — compiled to one JSON component artifact at
//!   `<out>/components/<tag>.json` (Phase 1, the unit a library ships).
//! * **A component-library project** (a `nemo.toml` with a `[package]` table) —
//!   every exported component is compiled to an artifact (Phase 1).
//! * **A plain app project** — a dry-run build plan is printed; compiling a whole
//!   project to a loadable `dist/` tree is Phase 2.
//!
//! Compilation reuses the runtime's own SFC transforms ahead-of-time — style-fold
//! (`crate::runtime::fold_sfc_styles`) then handler-ref rewrite
//! (`crate::runtime::rewrite_sfc_handlers`) — so an artifact's template is the
//! same `TemplateMap` entry `parse_layout_config` builds from source (a component
//! with no nested SFC tags needs no tag-rewrite, which is a whole-project
//! composition concern deferred to load time).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use nemo_config::{
    find_project_root, sfc_default_tag, sfc_definition_to_value, PackageConfig, ProjectManifest,
    Value, XmlParser, MANIFEST_FILE,
};
use serde::{Deserialize, Serialize};

use crate::args::BuildArgs;

pub fn run(args: BuildArgs) -> Result<()> {
    let start = match args.target {
        Some(target) => target,
        None => std::env::current_dir().context("resolving the current directory")?,
    };

    // A single `.nemo` file is compiled directly, with no project required.
    if start.is_file() && start.extension().map(|e| e == "nemo").unwrap_or(false) {
        return build_single_component(&start);
    }

    // Otherwise resolve the project root + manifest.
    let root = find_project_root(&start).with_context(|| {
        format!(
            "no {MANIFEST_FILE} found in {} or any parent directory",
            start.display()
        )
    })?;
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest = ProjectManifest::load(&manifest_path)
        .with_context(|| format!("loading {}", manifest_path.display()))?;

    match &manifest.package {
        Some(pkg) => build_package(&root, &manifest, pkg),
        None => print_project_plan(&root, &manifest),
    }
}

/// Compiles one `.nemo` file to `<out>/components/<tag>.json`. When the file
/// lives inside a project, `<out>` is the manifest's build dir; otherwise it is
/// `<file-parent>/dist`.
fn build_single_component(file: &Path) -> Result<()> {
    let out_base = out_base_for(file);
    let component = compile_component(file)?;
    let written = write_artifact(&out_base, &component)?;
    println!("Compiled {} → {}", component.tag, written.display());
    Ok(())
}

/// Compiles every exported component of a library project. With no `exports`
/// listed, the convention is every top-level `.nemo` file in the project root.
fn build_package(root: &Path, manifest: &ProjectManifest, pkg: &PackageConfig) -> Result<()> {
    let out_base = root.join(&manifest.build.out);
    let files = top_level_nemo_files(root)?;
    if files.is_empty() {
        eprintln!("warning: no .nemo files found in {}", root.display());
    }

    let mut built: Vec<String> = Vec::new();
    for file in &files {
        let component = compile_component(file)?;
        // When `exports` is set, only emit the listed tags.
        if !pkg.exports.is_empty() && !pkg.exports.contains(&component.tag) {
            continue;
        }
        let written = write_artifact(&out_base, &component)?;
        println!("Compiled {} → {}", component.tag, written.display());
        built.push(component.tag);
    }

    // Surface any export named in the manifest that matched no source file.
    for export in &pkg.exports {
        if !built.contains(export) {
            eprintln!("warning: exported component '{export}' has no matching .nemo source");
        }
    }

    println!(
        "Built {} component artifact(s) for package '{}'.",
        built.len(),
        manifest.name
    );
    Ok(())
}

/// Phase-0 dry run for a plain app project (compiling to a loadable `dist/` is
/// Phase 2).
fn print_project_plan(root: &Path, manifest: &ProjectManifest) -> Result<()> {
    let entry = root.join(&manifest.entry);
    let out = root.join(&manifest.build.out);

    println!("Build plan (dry run — project compilation is Phase 2)");
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
        "\nnote: this is an app project (no [package]); building it to a loadable \
         dist/ tree is Phase 2. To compile an individual component now, run \
         `nemo build <file.nemo>`."
    );
    Ok(())
}

/// Resolves the output base for a lone component file: the enclosing project's
/// build dir if the file is inside a project, else `<file-parent>/dist`.
fn out_base_for(file: &Path) -> PathBuf {
    find_project_root(file)
        .and_then(|root| {
            ProjectManifest::load(&root.join(MANIFEST_FILE))
                .ok()
                .map(|m| root.join(m.build.out))
        })
        .unwrap_or_else(|| file.parent().unwrap_or(Path::new(".")).join("dist"))
}

/// Lists top-level `.nemo` files in a directory, sorted for deterministic order.
fn top_level_nemo_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map(|e| e == "nemo").unwrap_or(false))
        .collect();
    files.sort();
    Ok(files)
}

/// Parses and compiles one `.nemo` file into a [`CompiledComponent`].
fn compile_component(file: &Path) -> Result<CompiledComponent> {
    let content =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let parser = XmlParser::new().with_source_name(file.display().to_string());
    let sfc = parser
        .parse_sfc(&content)
        .map_err(|e| anyhow!("parsing {}: {e}", file.display()))?;

    let tag = sfc_default_tag(sfc.name.as_deref(), file)
        .with_context(|| format!("could not determine a tag for {}", file.display()))?;
    let name = sfc.name.clone().unwrap_or_default();

    // Ahead-of-time transforms: style-fold, then handler-ref rewrite — the same
    // steps parse_layout_config runs to build the TemplateMap entry.
    let template = match sfc.style.as_deref() {
        Some(css) => crate::runtime::fold_sfc_styles(&sfc.template, css, &tag),
        None => sfc.template.clone(),
    };
    let template = crate::runtime::rewrite_sfc_handlers(&template, &tag);

    // Reuse the canonical flatten for script/props/slots so they match the
    // `config["sfc"][tag]` shape a loader reads back.
    let source = file.display().to_string();
    let flat = sfc_definition_to_value(sfc, &source);
    let script = flat
        .get("script")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let props = array_field(&flat, "props");
    let slots = array_field(&flat, "slots");

    Ok(CompiledComponent {
        tag,
        template,
        script,
        props,
        slots,
        meta: ComponentMeta { name, source },
    })
}

fn array_field(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Serializes a compiled component to `<out>/components/<tag>.json` and returns
/// the written path.
fn write_artifact(out_base: &Path, component: &CompiledComponent) -> Result<PathBuf> {
    let dir = out_base.join("components");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{}.json", component.tag));
    let json = serde_json::to_string_pretty(component).context("serializing component artifact")?;
    std::fs::write(&path, format!("{json}\n"))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// A compiled `.nemo` component artifact — the unit a component library ships.
///
/// `template` is the style-folded, handler-rewritten body (the `TemplateMap`
/// entry). `props`/`slots` carry the declared typed-prop and slot specs (same
/// shape as `config["sfc"][tag]`) so consumers can validate usage.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CompiledComponent {
    tag: String,
    template: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    props: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    slots: Vec<Value>,
    meta: ComponentMeta,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ComponentMeta {
    name: String,
    source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // No `<template name>`, so the tag derives from the filename stem — which lets
    // each test control the tag via the file it writes.
    const CARD: &str = r#"<props>
          <prop name="title" type="string" required="true" />
        </props>
        <template>
          <panel id="root">
            <label id="t" content="${title}" on_click="handleClick" />
            <slot />
          </panel>
        </template>
        <style>
          panel { padding: 16px; }
          #t { padding: 8px; }
        </style>
        <script>
          fn handleClick(component_id, event_data) { }
        </script>"#;

    fn write_tmp(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nemo_build_{}_{}", name, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.nemo"));
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn compiles_tag_style_and_handlers() {
        let file = write_tmp("labeled-card", CARD);
        let c = compile_component(&file).unwrap();

        // Tag from the filename stem (kebab→snake); no `<template name>`.
        assert_eq!(c.tag, "labeled_card");
        assert_eq!(c.meta.name, "");
        assert!(c.script.as_deref().unwrap().trim().starts_with("fn"));
        // Typed prop carried through.
        assert_eq!(c.props.len(), 1);

        // Type selector folds onto the root node.
        let root = &c.template;
        assert_eq!(
            root.get("padding").and_then(|v| v.as_i64()),
            Some(16),
            "type selector folded onto the root node (px stripped)"
        );

        // Id selector folds onto the keyed child; the bare handler is rewritten.
        let label = root
            .get("component")
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("t"))
            .expect("label child by id");
        assert_eq!(
            label.get("padding").and_then(|v| v.as_i64()),
            Some(8),
            "#id selector folded onto the child node"
        );
        assert_eq!(
            label.get("on_click").and_then(|v| v.as_str()),
            Some("sfc:labeled_card::handleClick"),
            "bare on_click rewritten to sfc:<tag>::<fn>"
        );

        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    #[test]
    fn artifact_round_trips_through_json() {
        let file = write_tmp("rt", CARD);
        let c = compile_component(&file).unwrap();
        let json = serde_json::to_string_pretty(&c).unwrap();
        let back: CompiledComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(
            c, back,
            "artifact survives a JSON serialize/deserialize cycle"
        );
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    // The compiled template must equal the TemplateMap entry the runtime builds
    // from source: load the same file through the full config path, then apply
    // the same transforms the runtime applies, and compare.
    #[test]
    fn template_matches_runtime_templatemap_entry() {
        let file = write_tmp("match", CARD);
        let artifact = compile_component(&file).unwrap();

        // Load via the config path: an app.xml importing the component.
        let dir = file.parent().unwrap();
        let xml = format!(
            r#"<nemo>
                 <imports><import src="./{}" /></imports>
                 <layout type="stack"><match id="m" title="x" /></layout>
               </nemo>"#,
            file.file_name().unwrap().to_string_lossy()
        );
        let config = nemo_config::ConfigurationLoader::new(std::sync::Arc::new(
            nemo_config::SchemaRegistry::new(),
        ))
        .load_xml_string(&xml, "app.xml", Some(dir))
        .unwrap();

        let entry = config
            .get("sfc")
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("match"))
            .expect("sfc entry");
        let body = entry.get("template").unwrap();
        let css = entry.get("style").and_then(|v| v.as_str()).unwrap();
        // Same transforms parse_layout_config runs (no nested SFC tags here, so
        // the tag-rewrite step is a no-op).
        let expected = crate::runtime::fold_sfc_styles(body, css, "match");
        let expected = crate::runtime::rewrite_sfc_handlers(&expected, "match");

        assert_eq!(
            artifact.template, expected,
            "artifact template equals the runtime's TemplateMap entry"
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
