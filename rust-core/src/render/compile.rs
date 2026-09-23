use std::collections::{BTreeMap, HashMap};

use super::error::RenderError;
use super::nodes::{ChoiceNode, TextGeneratorNode, UnsupportedValueNode, WeightedChoiceNode};
use super::processor::ProcessorRegistry;
use super::structured_template::StructuredLoopTemplate;
use super::template::template_to_node;
use super::value::{CopperlaceNumber, StructuredArrayEntry, StructuredNode};

const WEIGHTED_CHOICE_VALUE_KEY: &str = "value";
const WEIGHTED_CHOICE_WEIGHT_KEY: &str = "weight";

pub(crate) fn value_to_node(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    Ok(match value {
        hocon_rs::Value::String(template) => template_to_node(&template, processors)?,
        hocon_rs::Value::Array(values) => {
            if array_contains_weighted_entry(&values) {
                Box::new(weighted_choice_node(values, processors)?)
            } else {
                let nodes = values
                    .into_iter()
                    .map(|value| value_to_node(value, processors))
                    .collect::<Result<Vec<_>, _>>()?;
                Box::new(ChoiceNode::new(nodes))
            }
        }
        hocon_rs::Value::Object(_) => Box::new(UnsupportedValueNode::new("object".to_string())),
        _ => Box::new(value.to_string()),
    })
}

pub(crate) fn value_to_structured_template_node(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
    loops: &HashMap<String, StructuredLoopTemplate>,
) -> Result<StructuredNode, RenderError> {
    Ok(match value {
        hocon_rs::Value::Object(values) => {
            let mut nodes = BTreeMap::new();
            for (name, value) in values {
                nodes.insert(
                    name,
                    value_to_structured_template_node(value, processors, loops)?,
                );
            }
            StructuredNode::Object(nodes)
        }
        hocon_rs::Value::Array(values) => StructuredNode::Array(
            values
                .into_iter()
                .map(|value| template_array_entry(value, processors, loops))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        hocon_rs::Value::String(template) if loops.contains_key(&template) => {
            return Err(RenderError::InvalidExpression(
                "structured loop blocks must be array entries".to_string(),
            ));
        }
        hocon_rs::Value::String(template) if contains_loop_marker(&template, loops) => {
            return Err(RenderError::InvalidExpression(
                "structured loop blocks must occupy one array entry".to_string(),
            ));
        }
        hocon_rs::Value::String(template) => {
            StructuredNode::Text(template_to_node(&template, processors)?)
        }
        hocon_rs::Value::Number(number) => {
            StructuredNode::Number(CopperlaceNumber::from_json_number(number)?)
        }
        hocon_rs::Value::Boolean(value) => StructuredNode::Boolean(value),
        hocon_rs::Value::Null => StructuredNode::Null,
    })
}

fn template_array_entry(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
    loops: &HashMap<String, StructuredLoopTemplate>,
) -> Result<StructuredArrayEntry, RenderError> {
    if let hocon_rs::Value::String(marker) = &value
        && let Some(template) = loops.get(marker)
    {
        let body = value_to_structured_template_node(
            template.body.value.clone(),
            processors,
            &template.body.loops,
        )?;
        return Ok(StructuredArrayEntry::for_each(
            template.variable_name.clone(),
            template.source_name.clone(),
            body,
        ));
    }

    if let hocon_rs::Value::String(marker) = &value
        && contains_loop_marker(marker, loops)
    {
        return Err(RenderError::InvalidExpression(
            "structured loop blocks must occupy one array entry".to_string(),
        ));
    }

    value_to_structured_template_node(value, processors, loops)
        .map(StructuredArrayEntry::from_value)
}

pub(crate) fn insert_named_text_nodes(
    nodes: &mut HashMap<String, Box<dyn TextGeneratorNode>>,
    name: String,
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
    loops: &HashMap<String, StructuredLoopTemplate>,
) -> Result<(), RenderError> {
    match value {
        hocon_rs::Value::Object(values) => {
            nodes.insert(
                name.clone(),
                Box::new(UnsupportedValueNode::new("object".to_string())),
            );
            for (child_name, child_value) in values {
                insert_structured_child_text_nodes(
                    nodes,
                    format!("{name}.{child_name}"),
                    child_value,
                    processors,
                    loops,
                )?;
            }
        }
        value => {
            nodes.insert(name, value_to_node(value, processors)?);
        }
    }

    Ok(())
}

pub(crate) fn insert_context_text_nodes(
    nodes: &mut HashMap<String, Box<dyn TextGeneratorNode>>,
    name: String,
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
    loops: &HashMap<String, StructuredLoopTemplate>,
) -> Result<(), RenderError> {
    match value {
        hocon_rs::Value::Object(values) => {
            nodes.insert(
                name.clone(),
                Box::new(UnsupportedValueNode::new("object".to_string())),
            );
            for (child_name, child_value) in values {
                insert_context_text_nodes(
                    nodes,
                    format!("{name}.{child_name}"),
                    child_value,
                    processors,
                    loops,
                )?;
            }
        }
        value => {
            nodes.insert(name, value_to_node(value, processors)?);
        }
    }

    Ok(())
}

fn insert_structured_child_text_nodes(
    nodes: &mut HashMap<String, Box<dyn TextGeneratorNode>>,
    name: String,
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
    loops: &HashMap<String, StructuredLoopTemplate>,
) -> Result<(), RenderError> {
    match value {
        hocon_rs::Value::Object(values) => {
            nodes.insert(
                name.clone(),
                Box::new(UnsupportedValueNode::new("object".to_string())),
            );
            for (child_name, child_value) in values {
                insert_structured_child_text_nodes(
                    nodes,
                    format!("{name}.{child_name}"),
                    child_value,
                    processors,
                    loops,
                )?;
            }
        }
        hocon_rs::Value::Array(values) => {
            if values
                .iter()
                .any(|value| contains_template_loop(value, loops))
            {
                nodes.insert(
                    name,
                    Box::new(UnsupportedValueNode::new("array".to_string())),
                );
                return Ok(());
            }
            match value_to_node(hocon_rs::Value::Array(values), processors) {
                Ok(node) => {
                    nodes.insert(name, node);
                }
                Err(RenderError::InvalidWeightedChoice(_)) => {
                    nodes.insert(
                        name,
                        Box::new(UnsupportedValueNode::new("array".to_string())),
                    );
                }
                Err(error) => return Err(error),
            }
        }
        value => {
            nodes.insert(name, value_to_node(value, processors)?);
        }
    }

    Ok(())
}

pub(crate) fn contains_template_loop(
    value: &hocon_rs::Value,
    loops: &HashMap<String, StructuredLoopTemplate>,
) -> bool {
    match value {
        hocon_rs::Value::String(marker) => contains_loop_marker(marker, loops),
        hocon_rs::Value::Array(values) => values
            .iter()
            .any(|value| contains_template_loop(value, loops)),
        hocon_rs::Value::Object(values) => values
            .values()
            .any(|value| contains_template_loop(value, loops)),
        _ => false,
    }
}

fn contains_loop_marker(value: &str, loops: &HashMap<String, StructuredLoopTemplate>) -> bool {
    loops.keys().any(|marker| value.contains(marker))
}

fn array_contains_weighted_entry(values: &[hocon_rs::Value]) -> bool {
    values.iter().any(|value| {
        matches!(
            value,
            hocon_rs::Value::Object(object)
                if object.contains_key(WEIGHTED_CHOICE_VALUE_KEY)
                    || object.contains_key(WEIGHTED_CHOICE_WEIGHT_KEY)
        )
    })
}

fn weighted_choice_node(
    values: Vec<hocon_rs::Value>,
    processors: &ProcessorRegistry,
) -> Result<WeightedChoiceNode, RenderError> {
    let entries = values
        .into_iter()
        .map(|value| weighted_choice_entry(value, processors))
        .collect::<Result<Vec<_>, _>>()?;
    WeightedChoiceNode::new(entries)
}

fn weighted_choice_entry(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
) -> Result<(Box<dyn TextGeneratorNode>, f64), RenderError> {
    let mut object = match value {
        hocon_rs::Value::Object(object) => object,
        value => return Ok((value_to_node(value, processors)?, 1.0)),
    };

    if !(object.contains_key(WEIGHTED_CHOICE_VALUE_KEY)
        || object.contains_key(WEIGHTED_CHOICE_WEIGHT_KEY))
    {
        return Err(RenderError::InvalidWeightedChoice(format!(
            "object entries in weighted arrays must use {WEIGHTED_CHOICE_VALUE_KEY} and {WEIGHTED_CHOICE_WEIGHT_KEY}"
        )));
    }

    if object.len() != 2
        || !object.contains_key(WEIGHTED_CHOICE_VALUE_KEY)
        || !object.contains_key(WEIGHTED_CHOICE_WEIGHT_KEY)
    {
        return Err(RenderError::InvalidWeightedChoice(format!(
            "weighted entries must contain only {WEIGHTED_CHOICE_VALUE_KEY} and {WEIGHTED_CHOICE_WEIGHT_KEY}"
        )));
    }

    let weight = object.remove(WEIGHTED_CHOICE_WEIGHT_KEY).unwrap();
    let value = object.remove(WEIGHTED_CHOICE_VALUE_KEY).unwrap();
    let weight = weight_to_f64(weight)?;
    Ok((value_to_node(value, processors)?, weight))
}

fn weight_to_f64(value: hocon_rs::Value) -> Result<f64, RenderError> {
    let hocon_rs::Value::Number(number) = value else {
        return Err(RenderError::InvalidWeightedChoice(
            "weight must be numeric".to_string(),
        ));
    };

    let Some(weight) = number.as_f64() else {
        return Err(RenderError::InvalidWeightedChoice(
            "weight must be representable as a number".to_string(),
        ));
    };

    if !weight.is_finite() || weight < 0.0 {
        return Err(RenderError::InvalidWeightedChoice(
            "weight must be finite and non-negative".to_string(),
        ));
    }

    Ok(weight)
}
