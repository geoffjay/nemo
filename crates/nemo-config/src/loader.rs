//! Configuration loader - main entry point for loading configurations.

use crate::error::ConfigError;
use crate::registry::SchemaRegistry;
use crate::resolver::{ConfigResolver, ResolveContext};
use crate::validator::{ConfigValidator, ValidationResult};
use crate::xml_parser::XmlParser;
use crate::Value;
use std::path::Path;
use std::sync::Arc;

/// The serialized resolved-config file inside a built `dist/` tree.
pub const DIST_LAYOUT_FILE: &str = "layout.json";

/// Main entry point for loading and processing configuration files.
pub struct ConfigurationLoader {
    validator: ConfigValidator,
    resolver: ConfigResolver,
    #[allow(dead_code)]
    schema_registry: Arc<SchemaRegistry>,
}

impl ConfigurationLoader {
    /// Creates a new configuration loader.
    pub fn new(schema_registry: Arc<SchemaRegistry>) -> Self {
        ConfigurationLoader {
            validator: ConfigValidator::new(Arc::clone(&schema_registry)),
            resolver: ConfigResolver::new(),
            schema_registry,
        }
    }

    /// Loads a configuration file.
    ///
    /// A `.nemo` entry is an app SFC: it is compiled to the same `Value` tree
    /// [`load_xml_string`](Self::load_xml_string) produces for an equivalent
    /// `app.xml`, then run through the same directive-compile + `${}`
    /// resolution. Anything else (`.xml`, no extension) is parsed as XML.
    pub fn load(&self, path: &Path) -> Result<Value, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

        let source_name = path.display().to_string();
        let base_dir = path.parent();

        if path.extension().map(|e| e == "nemo").unwrap_or(false) {
            return self.load_nemo_string(&content, &source_name, base_dir);
        }

        self.load_xml_string(&content, &source_name, base_dir)
    }

    /// Loads configuration from an XML string.
    pub fn load_xml_string(
        &self,
        content: &str,
        source_name: &str,
        base_dir: Option<&Path>,
    ) -> Result<Value, ConfigError> {
        let parser = self.parser_for(source_name, base_dir);
        let mut raw_value = parser.parse(content).map_err(ConfigError::Parse)?;
        self.compile_resolve(&mut raw_value)
    }

    /// Loads an `app.nemo` SFC string: compiles it to the same `Value` tree
    /// [`load_xml_string`](Self::load_xml_string) produces for an equivalent
    /// `app.xml`, then runs the same directive-compile + `${}` resolution.
    pub fn load_nemo_string(
        &self,
        content: &str,
        source_name: &str,
        base_dir: Option<&Path>,
    ) -> Result<Value, ConfigError> {
        let parser = self.parser_for(source_name, base_dir);
        let mut raw_value = parser
            .compile_app_sfc(content)
            .map_err(ConfigError::Parse)?;
        self.compile_resolve(&mut raw_value)
    }

    /// Builds an [`XmlParser`] configured with the source name, base directory,
    /// and (when inside a project) the `.nemo/packages` cache + `nemo.lock`
    /// versions so remote module imports resolve.
    fn parser_for(&self, source_name: &str, base_dir: Option<&Path>) -> XmlParser {
        let mut parser = XmlParser::new().with_source_name(source_name);
        if let Some(dir) = base_dir {
            parser = parser.with_base_dir(dir);
            if let Some(root) = crate::manifest::find_project_root(dir) {
                let versions = crate::pkg::Lockfile::load(&root.join(crate::pkg::LOCKFILE))
                    .unwrap_or_default()
                    .versions();
                parser = parser.with_packages(crate::pkg::packages_dir(&root), versions);
            }
        }
        parser
    }

    /// Runs the shared post-parse pipeline: directive compilation then `${}`
    /// expression resolution.
    fn compile_resolve(&self, raw_value: &mut Value) -> Result<Value, ConfigError> {
        // Compile control-flow directives (n:if / n:for) in the layout and
        // SFC templates before resolution — the pass rewrites the Value tree
        // into ordinary nodes (or list-container nodes for live-data n:for).
        crate::compile_directives(raw_value);

        // Build resolve context from the parsed config, then resolve expressions.
        let context = self.build_context(raw_value);
        let resolved = self
            .resolver
            .resolve(std::mem::take(raw_value), &context)
            .map_err(ConfigError::Resolve)?;
        Ok(resolved)
    }

    /// Loads a pre-built `dist/` tree produced by `nemo build`: deserializes
    /// `<dir>/layout.json` (a serialized, fully-resolved config `Value`) back into
    /// the same `Value` the source path produces — with no XML parse, `${}`
    /// resolution, or `<import>`/`<include>` file reads. This is opt-in; the
    /// default launch path stays on [`load`], and `nemo dev` never uses it.
    pub fn load_from_dist(&self, dir: &Path) -> Result<Value, ConfigError> {
        let path = dir.join(DIST_LAYOUT_FILE);
        let content = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        serde_json::from_str(&content).map_err(|e| ConfigError::Io {
            path: path.display().to_string(),
            message: format!("invalid dist {DIST_LAYOUT_FILE}: {e}"),
        })
    }

    /// Loads and validates configuration against a schema.
    pub fn load_validated(&self, path: &Path, schema_name: &str) -> Result<Value, ConfigError> {
        let value = self.load(path)?;
        let result = self.validator.validate(&value, schema_name);

        if !result.valid {
            return Err(ConfigError::Validation {
                errors: result.errors,
            });
        }

        Ok(value)
    }

    /// Validates a configuration file.
    pub fn validate(&self, path: &Path, schema_name: &str) -> ValidationResult {
        match self.load(path) {
            Ok(value) => self.validator.validate(&value, schema_name),
            Err(e) => {
                let mut result = ValidationResult::default();
                result
                    .errors
                    .push(crate::ValidationError::load_error(&e.to_string()));
                result
            }
        }
    }

    /// Builds a resolve context from parsed configuration.
    fn build_context(&self, config: &Value) -> ResolveContext {
        let mut context = ResolveContext::with_system_env();
        context.config = config.clone();

        // Extract variables from "variable" blocks
        if let Some(variables) = config.get("variable").and_then(|v| v.as_object()) {
            for (name, var_config) in variables.iter() {
                if let Some(default) = var_config.get("default") {
                    context.variables.insert(name.clone(), default.clone());
                }
            }
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ConfigSchema, PropertySchema};

    fn create_test_loader() -> ConfigurationLoader {
        let registry = Arc::new(SchemaRegistry::new());

        let schema = ConfigSchema::new("application")
            .property("name", PropertySchema::string())
            .property("version", PropertySchema::string().with_default("1.0.0"))
            .require("name");

        registry.register(schema).unwrap();

        ConfigurationLoader::new(registry)
    }

    #[test]
    fn test_load_xml_string() {
        let loader = create_test_loader();

        let content = r#"
        <nemo>
            <app title="XML App">
                <window title="Test" />
                <theme name="kanagawa" mode="dark" />
            </app>
            <script src="./scripts" />
            <layout type="stack">
                <label id="header" text="Hello XML" />
            </layout>
        </nemo>
        "#;

        let value = loader.load_xml_string(content, "test.xml", None).unwrap();
        let app = value.get("app").unwrap();
        assert_eq!(
            app.get("title"),
            Some(&Value::String("XML App".to_string()))
        );

        let layout = value.get("layout").unwrap();
        assert_eq!(
            layout.get("type"),
            Some(&Value::String("stack".to_string()))
        );
    }

    #[test]
    fn test_load_from_dist_round_trips() {
        let loader = create_test_loader();
        let content = r#"
        <nemo>
            <app title="Dist App"><theme name="nord" mode="dark" /></app>
            <layout type="stack">
                <label id="header" text="Hello" />
            </layout>
        </nemo>
        "#;
        let source = loader.load_xml_string(content, "app.xml", None).unwrap();

        // Emulate `nemo build`: serialize the resolved config to a dist tree.
        let dir = std::env::temp_dir().join(format!("nemo_dist_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(DIST_LAYOUT_FILE),
            serde_json::to_string_pretty(&source).unwrap(),
        )
        .unwrap();

        let loaded = loader.load_from_dist(&dir).unwrap();
        assert_eq!(
            source, loaded,
            "dist reload equals the resolved source config"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_module_import_resolves_from_package_cache() {
        let loader = create_test_loader();
        let root = std::env::temp_dir().join(format!("nemo_pkg_import_{}", std::process::id()));
        // Marker file makes this a project root.
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("nemo.toml"), "name = \"app\"\n").unwrap();
        // A cached package with one component, pinned by nemo.lock.
        let pkg = root.join(".nemo/packages/example.com/lib@v1.0.0");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("card.nemo"),
            "<template name=\"card\"><panel><slot /></panel></template>",
        )
        .unwrap();
        std::fs::write(
            root.join("nemo.lock"),
            "[[package]]\nmodule = \"example.com/lib\"\nversion = \"v1.0.0\"\ncommit = \"abc\"\n",
        )
        .unwrap();

        let xml = r#"<nemo>
            <imports><import src="example.com/lib" as="nf" /></imports>
            <layout type="stack"><nf-card id="c" /></layout>
        </nemo>"#;
        let config = loader
            .load_xml_string(xml, "app.xml", Some(root.as_path()))
            .unwrap();

        let sfc = config.get("sfc").and_then(|v| v.as_object()).unwrap();
        assert!(
            sfc.contains_key("nf_card"),
            "namespaced module component registered; got {:?}",
            sfc.keys().collect::<Vec<_>>()
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_load_from_dist_missing_errors() {
        let loader = create_test_loader();
        let dir = std::env::temp_dir().join(format!("nemo_dist_missing_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(
            loader.load_from_dist(&dir).is_err(),
            "missing layout.json errors"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_xml_with_variables() {
        let loader = create_test_loader();

        let content = r#"
        <nemo>
            <variable name="greeting" default="Hello World" />
            <layout type="stack">
                <label id="lbl" text="${var.greeting}" />
            </layout>
        </nemo>
        "#;

        let value = loader.load_xml_string(content, "test.xml", None).unwrap();
        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let lbl = components.get("lbl").unwrap();
        assert_eq!(
            lbl.get("text"),
            Some(&Value::String("Hello World".to_string()))
        );
    }
}
