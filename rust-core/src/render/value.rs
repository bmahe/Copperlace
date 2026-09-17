use std::collections::BTreeMap;

use serde::Serialize;

use super::error::RenderError;
use super::nodes::TextGeneratorNode;
use super::state::RenderState;

/// Compiled structured document tree.
///
/// Text leaves reuse the same text-generation nodes as existing string render
/// APIs. Arrays and objects remain structural here even when equivalent
/// top-level entries are also indexed as text choice rules for compatibility.
pub enum StructuredNode {
    /// Object entries keyed by field name.
    Object(BTreeMap<String, StructuredNode>),
    /// Array entries in source order.
    Array(Vec<StructuredNode>),
    /// A text-generating template leaf.
    Text(Box<dyn TextGeneratorNode>),
    /// Numeric scalar.
    Number(CopperlaceNumber),
    /// Boolean scalar.
    Boolean(bool),
    /// Null scalar.
    Null,
}

impl StructuredNode {
    pub(crate) fn child(&self, path: &str) -> Option<&StructuredNode> {
        if path.is_empty() {
            return Some(self);
        }

        let mut node = self;
        for segment in path.split('.') {
            let StructuredNode::Object(values) = node else {
                return None;
            };
            node = values.get(segment)?;
        }
        Some(node)
    }

    pub(crate) fn value_type(&self) -> &'static str {
        match self {
            StructuredNode::Object(_) => "object",
            StructuredNode::Array(_) => "array",
            StructuredNode::Text(_) => "string",
            StructuredNode::Number(_) => "number",
            StructuredNode::Boolean(_) => "boolean",
            StructuredNode::Null => "null",
        }
    }

    pub(crate) fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        match self {
            StructuredNode::Text(node) => node.generate_text(state),
            StructuredNode::Number(value) => Ok(value.to_string()),
            StructuredNode::Boolean(value) => Ok(value.to_string()),
            StructuredNode::Null => Ok("null".to_string()),
            StructuredNode::Object(_) => Err(RenderError::UnsupportedValue("object".to_string())),
            StructuredNode::Array(_) => Err(RenderError::UnsupportedValue("array".to_string())),
        }
    }

    pub(crate) fn generate_value(
        &self,
        state: &mut RenderState,
    ) -> Result<CopperlaceValue, RenderError> {
        match self {
            StructuredNode::Object(values) => values
                .iter()
                .map(|(key, value)| Ok((key.clone(), value.generate_value(state)?)))
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(CopperlaceValue::Object),
            StructuredNode::Array(values) => values
                .iter()
                .map(|value| value.generate_value(state))
                .collect::<Result<Vec<_>, _>>()
                .map(CopperlaceValue::Array),
            StructuredNode::Text(node) => node.generate_text(state).map(CopperlaceValue::String),
            StructuredNode::Number(value) => Ok(CopperlaceValue::Number(*value)),
            StructuredNode::Boolean(value) => Ok(CopperlaceValue::Boolean(*value)),
            StructuredNode::Null => Ok(CopperlaceValue::Null),
        }
    }
}

/// Native Copperlace structured render result.
#[derive(Debug, Clone, PartialEq)]
pub enum CopperlaceValue {
    /// Object entries keyed by field name.
    Object(BTreeMap<String, CopperlaceValue>),
    /// Array entries in render order.
    Array(Vec<CopperlaceValue>),
    /// String scalar.
    String(String),
    /// Numeric scalar.
    Number(CopperlaceNumber),
    /// Boolean scalar.
    Boolean(bool),
    /// Null scalar.
    Null,
}

impl CopperlaceValue {
    pub(crate) fn child(&self, path: &str) -> Option<&CopperlaceValue> {
        if path.is_empty() {
            return Some(self);
        }

        let mut value = self;
        for segment in path.split('.') {
            let CopperlaceValue::Object(values) = value else {
                return None;
            };
            value = values.get(segment)?;
        }
        Some(value)
    }

    pub(crate) fn value_type(&self) -> &'static str {
        match self {
            CopperlaceValue::Object(_) => "object",
            CopperlaceValue::Array(_) => "array",
            CopperlaceValue::String(_) => "string",
            CopperlaceValue::Number(_) => "number",
            CopperlaceValue::Boolean(_) => "boolean",
            CopperlaceValue::Null => "null",
        }
    }

    pub(crate) fn to_rendered_text(&self) -> Result<String, RenderError> {
        match self {
            CopperlaceValue::String(value) => Ok(value.clone()),
            CopperlaceValue::Number(value) => Ok(value.to_string()),
            CopperlaceValue::Boolean(value) => Ok(value.to_string()),
            CopperlaceValue::Null => Ok("null".to_string()),
            CopperlaceValue::Object(_) => Err(RenderError::UnsupportedValue("object".to_string())),
            CopperlaceValue::Array(_) => Err(RenderError::UnsupportedValue("array".to_string())),
        }
    }

    /// Converts this value into a JSON value.
    pub fn into_json_value(self) -> serde_json::Value {
        match self {
            CopperlaceValue::Object(values) => serde_json::Value::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, value.into_json_value()))
                    .collect(),
            ),
            CopperlaceValue::Array(values) => serde_json::Value::Array(
                values
                    .into_iter()
                    .map(CopperlaceValue::into_json_value)
                    .collect(),
            ),
            CopperlaceValue::String(value) => serde_json::Value::String(value),
            CopperlaceValue::Number(value) => value.into_json_number(),
            CopperlaceValue::Boolean(value) => serde_json::Value::Bool(value),
            CopperlaceValue::Null => serde_json::Value::Null,
        }
    }

    /// Converts this value into a JSON value without consuming it.
    pub fn to_json_value(&self) -> serde_json::Value {
        self.clone().into_json_value()
    }

    /// Serializes this value as compact JSON.
    pub fn to_compact_json(&self) -> Result<String, RenderError> {
        serde_json::to_string(&self.to_json_value())
            .map_err(|error| RenderError::JsonSerialization(error.to_string()))
    }

    /// Serializes this value as formatted JSON using tabs for indentation.
    pub fn to_formatted_json(&self) -> Result<String, RenderError> {
        let mut output = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
        let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
        self.to_json_value()
            .serialize(&mut serializer)
            .map_err(|error| RenderError::JsonSerialization(error.to_string()))?;
        String::from_utf8(output).map_err(|error| RenderError::JsonSerialization(error.to_string()))
    }
}

/// Numeric scalar used by structured Copperlace values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CopperlaceNumber {
    /// Integer value representable as `i64`.
    Integer(i64),
    /// Unsigned integer value larger than `i64::MAX`.
    Unsigned(u64),
    /// Floating-point value representable as finite `f64`.
    Float(f64),
}

impl std::fmt::Display for CopperlaceNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopperlaceNumber::Integer(value) => value.fmt(formatter),
            CopperlaceNumber::Unsigned(value) => value.fmt(formatter),
            CopperlaceNumber::Float(value) => {
                let Some(number) = serde_json::Number::from_f64(*value) else {
                    return Err(std::fmt::Error);
                };
                number.fmt(formatter)
            }
        }
    }
}

impl CopperlaceNumber {
    pub(crate) fn from_json_number(number: serde_json::Number) -> Result<Self, RenderError> {
        if let Some(value) = number.as_i64() {
            return Ok(CopperlaceNumber::Integer(value));
        }
        if let Some(value) = number.as_u64() {
            return Ok(CopperlaceNumber::Unsigned(value));
        }
        let Some(value) = number.as_f64() else {
            return Err(RenderError::UnsupportedValue(
                "number must be representable as i64, u64, or f64".to_string(),
            ));
        };
        if !value.is_finite() {
            return Err(RenderError::UnsupportedValue(
                "number must be finite".to_string(),
            ));
        }
        Ok(CopperlaceNumber::Float(value))
    }

    fn into_json_number(self) -> serde_json::Value {
        match self {
            CopperlaceNumber::Integer(value) => serde_json::Value::Number(value.into()),
            CopperlaceNumber::Unsigned(value) => serde_json::Value::Number(value.into()),
            CopperlaceNumber::Float(value) => serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        }
    }
}
