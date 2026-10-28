//! Generates `PhosphorIcon` and the per-weight SVG tables from `assets/`.
//!
//! Everything is derived from the bundled SVG files, so the build needs no
//! network access. Run `script/sync-phosphor.sh` to update the icons.

use std::{collections::BTreeMap, env, fmt::Write, fs, path::PathBuf};

const WEIGHTS: [&str; 6] = ["thin", "light", "regular", "bold", "fill", "duotone"];

fn pascal_case(name: &str) -> String {
    name.split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let first = chars.next().unwrap();
            format!(
                "{}{}",
                first.to_ascii_uppercase(),
                chars.as_str().to_ascii_lowercase()
            )
        })
        .collect()
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let assets = manifest_dir.join("assets");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=build.rs");

    // variant -> kebab-case name, taken from the regular weight.
    let mut icons = BTreeMap::new();
    for entry in fs::read_dir(assets.join("regular")).expect("assets/regular directory") {
        let path = entry.expect("read icon directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("svg") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("UTF-8 icon name")
            .to_owned();
        let variant = pascal_case(&name);
        assert!(
            variant.starts_with(|c: char| c.is_ascii_alphabetic())
                && variant.chars().all(|c| c.is_ascii_alphanumeric()),
            "icon name does not map to a Rust identifier: {name}"
        );
        if let Some(previous) = icons.insert(variant.clone(), name.clone()) {
            panic!("icons `{previous}` and `{name}` both map to `{variant}`");
        }
    }
    assert!(!icons.is_empty(), "no icons found in assets/regular");

    // Every weight must ship the complete set, so any (icon, weight) pair is valid.
    for weight in WEIGHTS {
        for name in icons.values() {
            let file = assets.join(weight).join(format!("{name}.svg"));
            assert!(file.is_file(), "missing {}", file.display());
        }
    }

    let mut code = String::new();

    // The enum.
    code.push_str(
        "/// Every icon in the Phosphor set.\n\
         ///\n\
         /// On its own a `PhosphorIcon` uses the [`Weight::Regular`] weight; call\n\
         /// [`PhosphorIcon::weight`] or one of the shorthands such as\n\
         /// [`PhosphorIcon::bold`] to pick another one.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]\n\
         pub enum PhosphorIcon {\n",
    );
    for (variant, name) in &icons {
        writeln!(
            code,
            "    #[doc = \"[`{name}`](https://phosphoricons.com/?q={name})\"]\n    {variant},"
        )
        .unwrap();
    }
    code.push_str("}\n\n");

    // ALL, name(), and the name lookup table.
    code.push_str("impl PhosphorIcon {\n    /// Every icon, in variant-name order.\n    pub const ALL: &'static [Self] = &[\n");
    for variant in icons.keys() {
        writeln!(code, "        Self::{variant},").unwrap();
    }
    code.push_str("    ];\n\n    /// The icon's Phosphor name, e.g. `\"address-book\"`.\n    pub const fn name(self) -> &'static str {\n        match self {\n");
    for (variant, name) in &icons {
        writeln!(code, "            Self::{variant} => {name:?},").unwrap();
    }
    code.push_str("        }\n    }\n}\n\n");

    let mut by_name: Vec<_> = icons.iter().map(|(v, n)| (n, v)).collect();
    by_name.sort();
    code.push_str("/// `(name, icon)` pairs sorted by name, for binary search.\npub(crate) const BY_NAME: &[(&str, PhosphorIcon)] = &[\n");
    for (name, variant) in by_name {
        writeln!(code, "    ({name:?}, PhosphorIcon::{variant}),").unwrap();
    }
    code.push_str("];\n\n");

    // One `(path, bytes)` constant per icon and weight. Constants are only
    // emitted into a binary when something refers to them.
    code.push_str("#[doc(hidden)]\n#[allow(non_upper_case_globals)]\npub mod embedded {\n");
    for weight in WEIGHTS {
        writeln!(code, "    pub mod {weight} {{").unwrap();
        for (variant, name) in &icons {
            let asset = format!("phosphor/{weight}/{name}.svg");
            writeln!(
                code,
                "        pub const {variant}: (&str, &[u8]) = ({asset:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/{weight}/{name}.svg\")));"
            )
            .unwrap();
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n\n");

    // Whole-weight lookups, compiled only for weights enabled as features.
    for weight in WEIGHTS {
        writeln!(
            code,
            "#[cfg(feature = {weight:?})]\npub(crate) fn {weight}(icon: PhosphorIcon) -> (&'static str, &'static [u8]) {{\n    match icon {{"
        )
        .unwrap();
        for variant in icons.keys() {
            writeln!(
                code,
                "        PhosphorIcon::{variant} => embedded::{weight}::{variant},"
            )
            .unwrap();
        }
        code.push_str("    }\n}\n\n");
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    fs::write(out.join("phosphor.rs"), code).expect("write generated icon code");
}
