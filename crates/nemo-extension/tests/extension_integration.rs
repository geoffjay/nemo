//! Integration tests for the extension subsystem.
//!
//! These tests verify Rhai script loading, evaluation, extension discovery,
//! script lifecycle management, and the extension registry.

use nemo_extension::{ExtensionManager, ExtensionType, RhaiConfig};
use std::io::Write;

// ── Rhai engine basic evaluation ─────────────────────────────────────────

#[test]
fn rhai_eval_arithmetic() {
    let em = ExtensionManager::new();
    let result: i64 = em.eval("40 + 2").unwrap();
    assert_eq!(result, 42);
}

#[test]
fn rhai_eval_string() {
    let em = ExtensionManager::new();
    let result: String = em.eval(r#" "hello" + " " + "world" "#).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn rhai_eval_boolean() {
    let em = ExtensionManager::new();
    let result: bool = em.eval("10 > 5").unwrap();
    assert!(result);
}

#[test]
fn rhai_eval_error() {
    let em = ExtensionManager::new();
    let result = em.eval::<i64>("this is not valid rhai");
    assert!(result.is_err());
}

// ── Script loading from temp files ───────────────────────────────────────

#[test]
fn load_and_call_script() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = dir.path().join("math.rhai");
    {
        let mut f = std::fs::File::create(&script_path).unwrap();
        writeln!(f, "fn add(a, b) {{ a + b }}").unwrap();
    }

    let mut em = ExtensionManager::new();
    let id = em.load_script(&script_path).unwrap();

    assert!(!id.is_empty());
    assert!(em.list_scripts().contains(&id));

    let result: i64 = em.call_script(&id, "add", (3_i64, 4_i64)).unwrap();
    assert_eq!(result, 7);
}

#[test]
fn load_script_with_multiple_functions() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = dir.path().join("utils.rhai");
    {
        let mut f = std::fs::File::create(&script_path).unwrap();
        writeln!(
            f,
            r#"
fn double(x) {{ x * 2 }}
fn is_positive(x) {{ x > 0 }}
fn greet(name) {{ "Hello, " + name }}
"#
        )
        .unwrap();
    }

    let mut em = ExtensionManager::new();
    let id = em.load_script(&script_path).unwrap();

    let doubled: i64 = em.call_script(&id, "double", (21_i64,)).unwrap();
    assert_eq!(doubled, 42);

    let positive: bool = em.call_script(&id, "is_positive", (5_i64,)).unwrap();
    assert!(positive);

    let greeting: String = em
        .call_script(&id, "greet", ("World".to_string(),))
        .unwrap();
    assert_eq!(greeting, "Hello, World");
}

// ── Script reload ────────────────────────────────────────────────────────

#[test]
fn reload_script_picks_up_changes() {
    let dir = tempfile::tempdir().unwrap();
    let script_path = dir.path().join("reloadable.rhai");

    // Version 1
    {
        let mut f = std::fs::File::create(&script_path).unwrap();
        writeln!(f, "fn version() {{ 1 }}").unwrap();
    }

    let mut em = ExtensionManager::new();
    let id = em.load_script(&script_path).unwrap();

    let v1: i64 = em.call_script(&id, "version", ()).unwrap();
    assert_eq!(v1, 1);

    // Version 2 (overwrite file)
    {
        let mut f = std::fs::File::create(&script_path).unwrap();
        writeln!(f, "fn version() {{ 2 }}").unwrap();
    }

    em.reload_script(&id).unwrap();

    let v2: i64 = em.call_script(&id, "version", ()).unwrap();
    assert_eq!(v2, 2);
}

// ── Extension discovery ──────────────────────────────────────────────────

#[test]
fn discover_scripts_in_directory() {
    let dir = tempfile::tempdir().unwrap();

    // Create some .rhai files
    for name in &["handlers.rhai", "utils.rhai", "transforms.rhai"] {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "// {}", name).unwrap();
    }

    // Create a non-.rhai file (should be ignored)
    std::fs::File::create(dir.path().join("readme.txt")).unwrap();

    let mut em = ExtensionManager::new();
    em.add_script_path(dir.path());

    let manifests = em.discover().unwrap();
    let script_manifests: Vec<_> = manifests
        .iter()
        .filter(|m| m.extension_type == ExtensionType::Script)
        .collect();

    assert_eq!(script_manifests.len(), 3);
}

#[test]
fn discover_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let mut em = ExtensionManager::new();
    em.add_script_path(dir.path());

    let manifests = em.discover().unwrap();
    assert!(manifests.is_empty());
}

#[test]
fn discover_nonexistent_directory() {
    let mut em = ExtensionManager::new();
    em.add_script_path("/nonexistent/path/to/scripts");

    let manifests = em.discover().unwrap();
    assert!(manifests.is_empty());
}

// ── Script list management ───────────────────────────────────────────────

#[test]
fn list_scripts_empty_initially() {
    let em = ExtensionManager::new();
    assert!(em.list_scripts().is_empty());
}

#[test]
fn list_scripts_after_loading() {
    let dir = tempfile::tempdir().unwrap();
    let s1 = dir.path().join("a.rhai");
    let s2 = dir.path().join("b.rhai");
    {
        let mut f = std::fs::File::create(&s1).unwrap();
        writeln!(f, "fn foo() {{ 1 }}").unwrap();
    }
    {
        let mut f = std::fs::File::create(&s2).unwrap();
        writeln!(f, "fn bar() {{ 2 }}").unwrap();
    }

    let mut em = ExtensionManager::new();
    em.load_script(&s1).unwrap();
    em.load_script(&s2).unwrap();

    assert_eq!(em.list_scripts().len(), 2);
}

// ── Plugin list (no native plugins in tests) ─────────────────────────────

#[test]
fn list_plugins_empty_initially() {
    let em = ExtensionManager::new();
    assert!(em.list_plugins().is_empty());
}

// ── Extension with custom RhaiConfig ─────────────────────────────────────

#[test]
fn custom_rhai_config() {
    let config = RhaiConfig::default();
    let em = ExtensionManager::with_config(config);

    // Should still be able to evaluate
    let result: i64 = em.eval("1 + 1").unwrap();
    assert_eq!(result, 2);
}
