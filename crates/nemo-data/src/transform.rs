//! Data transformation pipeline.

use crate::error::{PipelineError, TransformError};
use chrono::{DateTime, Utc};
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context provided to transforms.
#[derive(Debug, Clone)]
pub struct TransformContext {
    /// Source ID that produced the data.
    pub source_id: String,
    /// Timestamp of the data.
    pub timestamp: DateTime<Utc>,
    /// Variables available during transformation.
    pub variables: HashMap<String, Value>,
}

impl Default for TransformContext {
    fn default() -> Self {
        Self {
            source_id: String::new(),
            timestamp: Utc::now(),
            variables: HashMap::new(),
        }
    }
}

/// Trait for data transformations.
pub trait Transform: Send + Sync {
    /// Transform input data.
    fn transform(&self, input: Value, context: &TransformContext) -> Result<Value, TransformError>;

    /// Name of this transform type.
    fn name(&self) -> &str;
}

/// A pipeline of transforms.
pub struct Pipeline {
    transforms: Vec<Box<dyn Transform>>,
}

impl Pipeline {
    /// Creates a new empty pipeline.
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
        }
    }

    /// Adds a transform to the pipeline.
    pub fn add(&mut self, transform: Box<dyn Transform>) {
        self.transforms.push(transform);
    }

    /// Executes the pipeline on input data.
    pub fn execute(&self, input: Value, context: &TransformContext) -> Result<Value, PipelineError> {
        if self.transforms.is_empty() {
            return Ok(input);
        }

        let mut current = input;
        for (i, transform) in self.transforms.iter().enumerate() {
            current = transform
                .transform(current, context)
                .map_err(|error| PipelineError::TransformFailed { stage: i, error })?;
        }
        Ok(current)
    }

    /// Returns the number of transforms in the pipeline.
    pub fn len(&self) -> usize {
        self.transforms.len()
    }

    /// Returns true if the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.transforms.is_empty()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Built-in Transforms ----

/// Map transform - applies an expression to each item.
pub struct MapTransform {
    /// Field mappings (target -> source path).
    pub mappings: HashMap<String, String>,
}

impl MapTransform {
    /// Creates a new map transform.
    pub fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }

    fn get_path(value: &Value, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)?;
                }
                Value::Array(arr) => {
                    let idx: usize = part.parse().ok()?;
                    current = arr.get(idx)?;
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }
}

impl Transform for MapTransform {
    fn transform(&self, input: Value, _context: &TransformContext) -> Result<Value, TransformError> {
        match input {
            Value::Array(items) => {
                let mapped: Vec<Value> = items
                    .into_iter()
                    .map(|item| {
                        let mut obj = indexmap::IndexMap::new();
                        for (target, source) in &self.mappings {
                            if let Some(val) = Self::get_path(&item, source) {
                                obj.insert(target.clone(), val);
                            }
                        }
                        Value::Object(obj)
                    })
                    .collect();
                Ok(Value::Array(mapped))
            }
            other => {
                let mut obj = indexmap::IndexMap::new();
                for (target, source) in &self.mappings {
                    if let Some(val) = Self::get_path(&other, source) {
                        obj.insert(target.clone(), val);
                    }
                }
                Ok(Value::Object(obj))
            }
        }
    }

    fn name(&self) -> &str {
        "map"
    }
}

/// Filter transform - keeps items matching a condition.
pub struct FilterTransform {
    /// Field to check.
    pub field: String,
    /// Expected value.
    pub value: Value,
    /// Whether to check for equality (true) or inequality (false).
    pub equals: bool,
}

impl FilterTransform {
    /// Creates a new filter transform that checks for equality.
    pub fn equals(field: impl Into<String>, value: Value) -> Self {
        Self {
            field: field.into(),
            value,
            equals: true,
        }
    }

    /// Creates a new filter transform that checks for inequality.
    pub fn not_equals(field: impl Into<String>, value: Value) -> Self {
        Self {
            field: field.into(),
            value,
            equals: false,
        }
    }

    fn matches(&self, item: &Value) -> bool {
        let field_value = match item {
            Value::Object(obj) => obj.get(&self.field),
            _ => None,
        };

        let is_equal = field_value == Some(&self.value);
        if self.equals {
            is_equal
        } else {
            !is_equal
        }
    }
}

impl Transform for FilterTransform {
    fn transform(&self, input: Value, _context: &TransformContext) -> Result<Value, TransformError> {
        match input {
            Value::Array(items) => {
                let filtered: Vec<Value> = items
                    .into_iter()
                    .filter(|item| self.matches(item))
                    .collect();
                Ok(Value::Array(filtered))
            }
            other => {
                if self.matches(&other) {
                    Ok(other)
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    fn name(&self) -> &str {
        "filter"
    }
}

/// Select transform - extracts specific fields.
pub struct SelectTransform {
    /// Fields to select.
    pub fields: Vec<String>,
}

impl SelectTransform {
    /// Creates a new select transform.
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }
}

impl Transform for SelectTransform {
    fn transform(&self, input: Value, _context: &TransformContext) -> Result<Value, TransformError> {
        match input {
            Value::Array(items) => {
                let selected: Vec<Value> = items
                    .into_iter()
                    .map(|item| {
                        if let Value::Object(obj) = item {
                            let mut new_obj = indexmap::IndexMap::new();
                            for field in &self.fields {
                                if let Some(val) = obj.get(field) {
                                    new_obj.insert(field.clone(), val.clone());
                                }
                            }
                            Value::Object(new_obj)
                        } else {
                            item
                        }
                    })
                    .collect();
                Ok(Value::Array(selected))
            }
            Value::Object(obj) => {
                let mut new_obj = indexmap::IndexMap::new();
                for field in &self.fields {
                    if let Some(val) = obj.get(field) {
                        new_obj.insert(field.clone(), val.clone());
                    }
                }
                Ok(Value::Object(new_obj))
            }
            other => Ok(other),
        }
    }

    fn name(&self) -> &str {
        "select"
    }
}

/// Sort transform - orders items by a field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortTransform {
    /// Field to sort by.
    pub by: String,
    /// Sort in descending order.
    pub descending: bool,
}

impl SortTransform {
    /// Creates a new ascending sort transform.
    pub fn asc(by: impl Into<String>) -> Self {
        Self {
            by: by.into(),
            descending: false,
        }
    }

    /// Creates a new descending sort transform.
    pub fn desc(by: impl Into<String>) -> Self {
        Self {
            by: by.into(),
            descending: true,
        }
    }
}

impl Transform for SortTransform {
    fn transform(&self, input: Value, _context: &TransformContext) -> Result<Value, TransformError> {
        match input {
            Value::Array(mut items) => {
                let field = self.by.clone();
                let desc = self.descending;

                items.sort_by(|a, b| {
                    let va = match a {
                        Value::Object(obj) => obj.get(&field),
                        _ => None,
                    };
                    let vb = match b {
                        Value::Object(obj) => obj.get(&field),
                        _ => None,
                    };

                    let cmp = match (va, vb) {
                        (Some(Value::Integer(a)), Some(Value::Integer(b))) => a.cmp(b),
                        (Some(Value::Float(a)), Some(Value::Float(b))) => {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
                        _ => std::cmp::Ordering::Equal,
                    };

                    if desc {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });

                Ok(Value::Array(items))
            }
            other => Ok(other),
        }
    }

    fn name(&self) -> &str {
        "sort"
    }
}

/// Take transform - limits the number of items.
pub struct TakeTransform {
    /// Maximum number of items to take.
    pub count: usize,
}

impl TakeTransform {
    /// Creates a new take transform.
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl Transform for TakeTransform {
    fn transform(&self, input: Value, _context: &TransformContext) -> Result<Value, TransformError> {
        match input {
            Value::Array(items) => {
                let taken: Vec<Value> = items.into_iter().take(self.count).collect();
                Ok(Value::Array(taken))
            }
            other => Ok(other),
        }
    }

    fn name(&self) -> &str {
        "take"
    }
}

/// Skip transform - skips a number of items.
pub struct SkipTransform {
    /// Number of items to skip.
    pub count: usize,
}

impl SkipTransform {
    /// Creates a new skip transform.
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl Transform for SkipTransform {
    fn transform(&self, input: Value, _context: &TransformContext) -> Result<Value, TransformError> {
        match input {
            Value::Array(items) => {
                let skipped: Vec<Value> = items.into_iter().skip(self.count).collect();
                Ok(Value::Array(skipped))
            }
            other => Ok(other),
        }
    }

    fn name(&self) -> &str {
        "skip"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_empty() {
        let pipeline = Pipeline::new();
        let ctx = TransformContext::default();
        let input = Value::String("test".into());
        let result = pipeline.execute(input.clone(), &ctx).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_filter_transform() {
        let filter = FilterTransform::equals("status", Value::String("active".into()));
        let ctx = TransformContext::default();

        let mut item1 = indexmap::IndexMap::new();
        item1.insert("status".to_string(), Value::String("active".into()));

        let mut item2 = indexmap::IndexMap::new();
        item2.insert("status".to_string(), Value::String("inactive".into()));

        let input = Value::Array(vec![Value::Object(item1), Value::Object(item2)]);
        let result = filter.transform(input, &ctx).unwrap();

        if let Value::Array(items) = result {
            assert_eq!(items.len(), 1);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_select_transform() {
        let select = SelectTransform::new(vec!["name".to_string()]);
        let ctx = TransformContext::default();

        let mut item = indexmap::IndexMap::new();
        item.insert("name".to_string(), Value::String("test".into()));
        item.insert("age".to_string(), Value::Integer(30));

        let input = Value::Object(item);
        let result = select.transform(input, &ctx).unwrap();

        if let Value::Object(obj) = result {
            assert_eq!(obj.len(), 1);
            assert!(obj.contains_key("name"));
            assert!(!obj.contains_key("age"));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_sort_transform() {
        let sort = SortTransform::asc("value");
        let ctx = TransformContext::default();

        let mut item1 = indexmap::IndexMap::new();
        item1.insert("value".to_string(), Value::Integer(3));

        let mut item2 = indexmap::IndexMap::new();
        item2.insert("value".to_string(), Value::Integer(1));

        let mut item3 = indexmap::IndexMap::new();
        item3.insert("value".to_string(), Value::Integer(2));

        let input = Value::Array(vec![
            Value::Object(item1),
            Value::Object(item2),
            Value::Object(item3),
        ]);
        let result = sort.transform(input, &ctx).unwrap();

        if let Value::Array(items) = result {
            if let Value::Object(first) = &items[0] {
                assert_eq!(first.get("value"), Some(&Value::Integer(1)));
            }
        } else {
            panic!("Expected array");
        }
    }
}
