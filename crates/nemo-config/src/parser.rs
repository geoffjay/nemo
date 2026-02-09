//! HCL parser implementation.

use crate::error::ParseError;
use crate::location::SourceLocation;
use crate::Value;
use indexmap::IndexMap;

/// Parser for HCL configuration files.
pub struct HclParser {
    source_name: String,
}

impl HclParser {
    /// Creates a new HCL parser.
    pub fn new() -> Self {
        HclParser {
            source_name: "<input>".to_string(),
        }
    }

    /// Sets the source name for error messages.
    pub fn with_source_name(mut self, name: impl Into<String>) -> Self {
        self.source_name = name.into();
        self
    }

    /// Parses HCL content into a Value.
    pub fn parse(&self, content: &str) -> Result<Value, ParseError> {
        let body: hcl::Body = hcl::from_str(content).map_err(|e| {
            ParseError::new(e.to_string(), self.location_from_error(&e, content))
                .with_source(content.to_string())
        })?;

        Ok(self.body_to_value(&body))
    }

    /// Converts an HCL body to a Value.
    fn body_to_value(&self, body: &hcl::Body) -> Value {
        let mut map = IndexMap::new();

        for attr in body.attributes() {
            let key = attr.key.as_str().to_string();
            let value = self.expression_to_value(&attr.expr);
            map.insert(key, value);
        }

        for block in body.blocks() {
            let block_type = block.identifier.as_str().to_string();
            let block_value = self.block_to_value(block);

            // Handle labeled blocks
            if block.labels.is_empty() {
                // Unlabeled block - treat as nested object
                match map.get_mut(&block_type) {
                    Some(Value::Array(arr)) => arr.push(block_value),
                    Some(_) => {
                        let existing = map.shift_remove(&block_type).unwrap();
                        map.insert(block_type, Value::Array(vec![existing, block_value]));
                    }
                    None => {
                        map.insert(block_type, block_value);
                    }
                }
            } else {
                // Labeled block - use first label as key
                let label = block.labels[0].as_str().to_string();
                let block_map = map
                    .entry(block_type)
                    .or_insert_with(|| Value::Object(IndexMap::new()));

                if let Value::Object(obj) = block_map {
                    obj.insert(label, block_value);
                }
            }
        }

        Value::Object(map)
    }

    /// Converts an HCL block to a Value.
    fn block_to_value(&self, block: &hcl::Block) -> Value {
        self.body_to_value(&block.body)
    }

    /// Converts an HCL expression to a Value.
    fn expression_to_value(&self, expr: &hcl::Expression) -> Value {
        match expr {
            hcl::Expression::Null => Value::Null,
            hcl::Expression::Bool(b) => Value::Bool(*b),
            hcl::Expression::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            hcl::Expression::String(s) => Value::String(s.to_string()),
            hcl::Expression::Array(arr) => {
                Value::Array(arr.iter().map(|e| self.expression_to_value(e)).collect())
            }
            hcl::Expression::Object(obj) => {
                let mut map = IndexMap::new();
                for (k, v) in obj.iter() {
                    let key = match k {
                        hcl::ObjectKey::Identifier(id) => id.as_str().to_string(),
                        hcl::ObjectKey::Expression(e) => self.expression_to_value(e).to_string(),
                        &_ => format!("{:?}", k),
                    };
                    map.insert(key, self.expression_to_value(v));
                }
                Value::Object(map)
            }
            hcl::Expression::TemplateExpr(template) => {
                // Parse template into elements and reconstruct with ${...} markers
                // that the resolver can handle
                let Ok(tpl) = hcl::template::Template::from_expr(template) else {
                    return Value::String(format!("{}", template));
                };
                let mut result = String::new();
                for element in tpl.elements() {
                    match element {
                        hcl::template::Element::Literal(s) => result.push_str(&s),
                        hcl::template::Element::Interpolation(interp) => {
                            let val = self.expression_to_value(&interp.expr);
                            if let Value::String(s) = val {
                                result.push_str(&s);
                            } else {
                                result.push_str(&val.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                Value::String(result)
            }
            hcl::Expression::Variable(var) => {
                // Keep variable references as strings with markers
                Value::String(format!("${{{}}}", var.as_str()))
            }
            hcl::Expression::Traversal(traversal) => {
                // Keep traversals as strings - build path from operators
                let root = format!("{}", traversal.expr);
                let ops: Vec<String> = traversal.operators.iter().map(|op| {
                    match op {
                        hcl::expr::TraversalOperator::GetAttr(ident) => ident.as_str().to_string(),
                        hcl::expr::TraversalOperator::Index(expr) => {
                            format!("[{}]", self.expression_to_value(expr))
                        }
                        hcl::expr::TraversalOperator::LegacyIndex(idx) => format!("[{}]", idx),
                        hcl::expr::TraversalOperator::AttrSplat => "*".to_string(),
                        hcl::expr::TraversalOperator::FullSplat => "[*]".to_string(),
                    }
                }).collect();
                let path = if ops.is_empty() {
                    root
                } else {
                    format!("{}.{}", root, ops.join("."))
                };
                Value::String(format!("${{{}}}", path))
            }
            hcl::Expression::FuncCall(func) => {
                // Keep function calls as strings
                Value::String(format!("${{{}(...)}}", func.name))
            }
            hcl::Expression::Conditional(cond) => {
                // Keep conditionals as strings
                Value::String(format!("${{if {:?} then ... else ...}}", cond.cond_expr))
            }
            hcl::Expression::Operation(op) => {
                // Keep operations as strings
                Value::String(format!("${{operation: {:?}}}", op))
            }
            hcl::Expression::ForExpr(_) => Value::String("${for ...}".to_string()),
            hcl::Expression::Parenthesis(inner) => self.expression_to_value(inner),
            _ => Value::String(format!("{:?}", expr)),
        }
    }

    /// Extracts location from an HCL error.
    fn location_from_error(&self, _error: &hcl::Error, _content: &str) -> SourceLocation {
        // hcl-rs doesn't provide detailed location info, so we return unknown
        SourceLocation::new(&self.source_name, 1, 1)
    }
}

impl Default for HclParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let parser = HclParser::new();
        let content = r#"
            name = "test"
            count = 42
            enabled = true
        "#;

        let value = parser.parse(content).unwrap();
        assert_eq!(value.get("name"), Some(&Value::String("test".into())));
        assert_eq!(value.get("count"), Some(&Value::Integer(42)));
        assert_eq!(value.get("enabled"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_parse_nested_block() {
        let parser = HclParser::new();
        let content = r#"
            application {
                name = "MyApp"
                version = "1.0.0"
            }
        "#;

        let value = parser.parse(content).unwrap();
        let app = value.get("application").unwrap();
        assert_eq!(app.get("name"), Some(&Value::String("MyApp".into())));
    }

    #[test]
    fn test_parse_labeled_block() {
        let parser = HclParser::new();
        let content = r#"
            variable "app_name" {
                default = "Test"
            }
        "#;

        let value = parser.parse(content).unwrap();
        let vars = value.get("variable").unwrap();
        let app_name = vars.get("app_name").unwrap();
        assert_eq!(app_name.get("default"), Some(&Value::String("Test".into())));
    }

    #[test]
    fn test_parse_array() {
        let parser = HclParser::new();
        let content = r#"
            ports = [80, 443, 8080]
        "#;

        let value = parser.parse(content).unwrap();
        let ports = value.get("ports").unwrap().as_array().unwrap();
        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0], Value::Integer(80));
    }
}
