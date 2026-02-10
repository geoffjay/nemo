//! Configuration expression resolver.

use crate::error::ResolveError;
use crate::Value;
use indexmap::IndexMap;
use std::collections::HashMap;

/// Context for resolving expressions.
#[derive(Debug, Clone, Default)]
pub struct ResolveContext {
    /// Variables available for interpolation.
    pub variables: HashMap<String, Value>,
    /// The full configuration (for self-references).
    pub config: Value,
    /// Environment variables.
    pub env: HashMap<String, String>,
}

impl ResolveContext {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a context with system environment variables.
    pub fn with_system_env() -> Self {
        let mut ctx = Self::new();
        for (key, value) in std::env::vars() {
            ctx.env.insert(key, value);
        }
        ctx
    }

    /// Adds a variable to the context.
    pub fn with_variable(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Gets a variable by name.
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Gets an environment variable.
    pub fn get_env(&self, name: &str) -> Option<&String> {
        self.env.get(name)
    }
}

/// Resolver for configuration expressions and interpolations.
pub struct ConfigResolver {
    functions: HashMap<String, Box<dyn ConfigFunction>>,
}

/// Trait for custom configuration functions.
pub trait ConfigFunction: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError>;
}

impl ConfigResolver {
    /// Creates a new resolver with built-in functions.
    pub fn new() -> Self {
        let mut resolver = ConfigResolver {
            functions: HashMap::new(),
        };
        resolver.register_builtins();
        resolver
    }

    /// Registers built-in functions.
    fn register_builtins(&mut self) {
        self.register_function(Box::new(UpperFunction));
        self.register_function(Box::new(LowerFunction));
        self.register_function(Box::new(TrimFunction));
        self.register_function(Box::new(LengthFunction));
        self.register_function(Box::new(CoalesceFunction));
        self.register_function(Box::new(EnvFunction));
    }

    /// Registers a custom function.
    pub fn register_function(&mut self, func: Box<dyn ConfigFunction>) {
        self.functions.insert(func.name().to_string(), func);
    }

    /// Resolves all expressions in a value.
    pub fn resolve(&self, value: Value, context: &ResolveContext) -> Result<Value, ResolveError> {
        match value {
            Value::String(s) => self.resolve_string(&s, context),
            Value::Array(arr) => {
                let resolved: Result<Vec<_>, _> =
                    arr.into_iter().map(|v| self.resolve(v, context)).collect();
                Ok(Value::Array(resolved?))
            }
            Value::Object(obj) => {
                let resolved: Result<IndexMap<_, _>, _> = obj
                    .into_iter()
                    .map(|(k, v)| self.resolve(v, context).map(|rv| (k, rv)))
                    .collect();
                Ok(Value::Object(resolved?))
            }
            other => Ok(other),
        }
    }

    /// Resolves a string that may contain interpolations.
    fn resolve_string(&self, s: &str, context: &ResolveContext) -> Result<Value, ResolveError> {
        // Check for ${...} patterns
        if !s.contains("${") {
            return Ok(Value::String(s.to_string()));
        }

        // Simple interpolation: "${var.name}" -> resolve the whole thing
        if s.starts_with("${") && s.ends_with("}") && s.matches("${").count() == 1 {
            let expr = &s[2..s.len() - 1];
            return self.resolve_expression(expr, context);
        }

        // Complex interpolation: "Hello ${var.name}!" -> string replacement
        let mut result = s.to_string();
        let mut start = 0;

        while let Some(pos) = result[start..].find("${") {
            let expr_start = start + pos + 2;
            if let Some(end) = result[expr_start..].find('}') {
                let expr_end = expr_start + end;
                let expr = &result[expr_start..expr_end];
                let resolved = self.resolve_expression(expr, context)?;
                let replacement = resolved.to_string();
                result = format!(
                    "{}{}{}",
                    &result[..start + pos],
                    replacement,
                    &result[expr_end + 1..]
                );
                start = start + pos + replacement.len();
            } else {
                break;
            }
        }

        Ok(Value::String(result))
    }

    /// Resolves a single expression.
    pub fn resolve_expression(
        &self,
        expr: &str,
        context: &ResolveContext,
    ) -> Result<Value, ResolveError> {
        let expr = expr.trim();

        // String literal
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return Ok(Value::String(expr[1..expr.len() - 1].to_string()));
        }

        // Number literal
        if let Ok(i) = expr.parse::<i64>() {
            return Ok(Value::Integer(i));
        }
        if let Ok(f) = expr.parse::<f64>() {
            return Ok(Value::Float(f));
        }

        // Boolean literal
        if expr == "true" {
            return Ok(Value::Bool(true));
        }
        if expr == "false" {
            return Ok(Value::Bool(false));
        }

        // null
        if expr == "null" {
            return Ok(Value::Null);
        }

        // Function call: func(args)
        if let Some(paren_pos) = expr.find('(') {
            if expr.ends_with(')') {
                let func_name = &expr[..paren_pos];
                let args_str = &expr[paren_pos + 1..expr.len() - 1];
                return self.call_function(func_name, args_str, context);
            }
        }

        // Conditional: cond ? true_val : false_val
        // Must be checked before var./env. prefixes so expressions like
        // "var.enabled ? \"yes\" : \"no\"" are split before the prefix match.
        if let Some(q_pos) = expr.find('?') {
            if let Some(c_pos) = expr.find(':') {
                let cond = &expr[..q_pos].trim();
                let true_val = &expr[q_pos + 1..c_pos].trim();
                let false_val = &expr[c_pos + 1..].trim();

                let cond_result = self.resolve_expression(cond, context)?;
                let is_true = match cond_result {
                    Value::Bool(b) => b,
                    Value::Null => false,
                    _ => true,
                };

                return if is_true {
                    self.resolve_expression(true_val, context)
                } else {
                    self.resolve_expression(false_val, context)
                };
            }
        }

        // Comparison operators
        // Must be checked before var./env. prefixes for the same reason.
        for op in &[" == ", " != ", " >= ", " <= ", " > ", " < "] {
            if let Some(pos) = expr.find(op) {
                let left = self.resolve_expression(&expr[..pos], context)?;
                let right = self.resolve_expression(&expr[pos + op.len()..], context)?;
                let result = match *op {
                    " == " => left == right,
                    " != " => left != right,
                    " >= " => compare_values(&left, &right)
                        .map(|c| c >= 0)
                        .unwrap_or(false),
                    " <= " => compare_values(&left, &right)
                        .map(|c| c <= 0)
                        .unwrap_or(false),
                    " > " => compare_values(&left, &right)
                        .map(|c| c > 0)
                        .unwrap_or(false),
                    " < " => compare_values(&left, &right)
                        .map(|c| c < 0)
                        .unwrap_or(false),
                    _ => unreachable!(),
                };
                return Ok(Value::Bool(result));
            }
        }

        // Variable reference: var.name or env.NAME
        if expr.starts_with("var.") {
            let var_path = &expr[4..];
            return self.resolve_variable_path(var_path, context);
        }

        if expr.starts_with("env.") {
            let env_name = &expr[4..];
            return Ok(context
                .get_env(env_name)
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null));
        }

        Err(ResolveError::UndefinedVariable {
            name: expr.to_string(),
        })
    }

    /// Resolves a variable path (e.g., "user.name" from variables).
    fn resolve_variable_path(
        &self,
        path: &str,
        context: &ResolveContext,
    ) -> Result<Value, ResolveError> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Err(ResolveError::InvalidPath {
                path: path.to_string(),
                message: "empty path".to_string(),
            });
        }

        let mut current = context.get_variable(parts[0]).cloned().ok_or_else(|| {
            ResolveError::UndefinedVariable {
                name: parts[0].to_string(),
            }
        })?;

        for part in &parts[1..] {
            current = current
                .get(part)
                .cloned()
                .ok_or_else(|| ResolveError::InvalidPath {
                    path: path.to_string(),
                    message: format!("key '{}' not found", part),
                })?;
        }

        Ok(current)
    }

    /// Calls a function with arguments.
    fn call_function(
        &self,
        name: &str,
        args_str: &str,
        context: &ResolveContext,
    ) -> Result<Value, ResolveError> {
        let func = self
            .functions
            .get(name)
            .ok_or_else(|| ResolveError::UnknownFunction {
                name: name.to_string(),
            })?;

        let args = self.parse_function_args(args_str, context)?;
        func.call(args)
    }

    /// Parses function arguments.
    fn parse_function_args(
        &self,
        args_str: &str,
        context: &ResolveContext,
    ) -> Result<Vec<Value>, ResolveError> {
        if args_str.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Simple split by comma (doesn't handle nested commas)
        let args: Result<Vec<_>, _> = args_str
            .split(',')
            .map(|arg| self.resolve_expression(arg.trim(), context))
            .collect();

        args
    }
}

impl Default for ConfigResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Compares two values, returning ordering if comparable.
fn compare_values(left: &Value, right: &Value) -> Option<i32> {
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Some(a.cmp(b) as i32),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).map(|o| o as i32),
        (Value::Integer(a), Value::Float(b)) => (*a as f64).partial_cmp(b).map(|o| o as i32),
        (Value::Float(a), Value::Integer(b)) => a.partial_cmp(&(*b as f64)).map(|o| o as i32),
        (Value::String(a), Value::String(b)) => Some(a.cmp(b) as i32),
        _ => None,
    }
}

// Built-in functions

struct UpperFunction;
impl ConfigFunction for UpperFunction {
    fn name(&self) -> &str {
        "upper"
    }
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError> {
        let s =
            args.first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| ResolveError::InvalidArgument {
                    function: "upper".to_string(),
                    message: "expected string argument".to_string(),
                })?;
        Ok(Value::String(s.to_uppercase()))
    }
}

struct LowerFunction;
impl ConfigFunction for LowerFunction {
    fn name(&self) -> &str {
        "lower"
    }
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError> {
        let s =
            args.first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| ResolveError::InvalidArgument {
                    function: "lower".to_string(),
                    message: "expected string argument".to_string(),
                })?;
        Ok(Value::String(s.to_lowercase()))
    }
}

struct TrimFunction;
impl ConfigFunction for TrimFunction {
    fn name(&self) -> &str {
        "trim"
    }
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError> {
        let s =
            args.first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| ResolveError::InvalidArgument {
                    function: "trim".to_string(),
                    message: "expected string argument".to_string(),
                })?;
        Ok(Value::String(s.trim().to_string()))
    }
}

struct LengthFunction;
impl ConfigFunction for LengthFunction {
    fn name(&self) -> &str {
        "length"
    }
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError> {
        let arg = args.first().ok_or_else(|| ResolveError::InvalidArgument {
            function: "length".to_string(),
            message: "expected argument".to_string(),
        })?;
        let len = match arg {
            Value::String(s) => s.len(),
            Value::Array(a) => a.len(),
            Value::Object(o) => o.len(),
            _ => {
                return Err(ResolveError::InvalidArgument {
                    function: "length".to_string(),
                    message: "expected string, array, or object".to_string(),
                })
            }
        };
        Ok(Value::Integer(len as i64))
    }
}

struct CoalesceFunction;
impl ConfigFunction for CoalesceFunction {
    fn name(&self) -> &str {
        "coalesce"
    }
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError> {
        for arg in args {
            if !arg.is_null() {
                return Ok(arg);
            }
        }
        Ok(Value::Null)
    }
}

struct EnvFunction;
impl ConfigFunction for EnvFunction {
    fn name(&self) -> &str {
        "env"
    }
    fn call(&self, args: Vec<Value>) -> Result<Value, ResolveError> {
        let name =
            args.first()
                .and_then(|v| v.as_str())
                .ok_or_else(|| ResolveError::InvalidArgument {
                    function: "env".to_string(),
                    message: "expected string argument".to_string(),
                })?;
        Ok(std::env::var(name)
            .map(Value::String)
            .unwrap_or(Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_variable() {
        let resolver = ConfigResolver::new();
        let context = ResolveContext::new().with_variable("name", "test");

        let result = resolver.resolve_expression("var.name", &context).unwrap();
        assert_eq!(result, Value::String("test".to_string()));
    }

    #[test]
    fn test_resolve_string_interpolation() {
        let resolver = ConfigResolver::new();
        let context = ResolveContext::new().with_variable("name", "World");

        let value = Value::String("Hello, ${var.name}!".to_string());
        let result = resolver.resolve(value, &context).unwrap();
        assert_eq!(result, Value::String("Hello, World!".to_string()));
    }

    #[test]
    fn test_resolve_function() {
        let resolver = ConfigResolver::new();
        let context = ResolveContext::new();

        let result = resolver
            .resolve_expression("upper(\"hello\")", &context)
            .unwrap();
        assert_eq!(result, Value::String("HELLO".to_string()));
    }

    #[test]
    fn test_resolve_conditional() {
        let resolver = ConfigResolver::new();
        let context = ResolveContext::new().with_variable("enabled", true);

        let result = resolver
            .resolve_expression("var.enabled ? \"yes\" : \"no\"", &context)
            .unwrap();
        assert_eq!(result, Value::String("yes".to_string()));
    }
}
