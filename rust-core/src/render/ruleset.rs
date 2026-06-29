use std::collections::{BTreeMap, HashMap};

use crate::processors::builtin_processors;

use super::compile::{insert_named_text_nodes, value_to_structured_node};
use super::error::RenderError;
use super::nodes::TextGeneratorNode;
use super::processor::ProcessorRegistry;
use super::state::{RenderContext, RenderState};
use super::value::{CopperlaceValue, StructuredNode};

/// Compiled collection of named rules from the config.
///
/// Top-level config entries become startable rules, so callers can render
/// `origin`, `story`, `name`, or any other named entry directly. A top-level
/// `context` object is treated specially: its entries become lazy defaults for
/// bound variables, so `{hero}` can generate and cache `context.hero` on first
/// use within a render.
pub struct RuleSet {
    document: StructuredNode,
    text_rules: HashMap<String, Box<dyn TextGeneratorNode>>,
    context_defaults: HashMap<String, Box<dyn TextGeneratorNode>>,
    processors: ProcessorRegistry,
}

impl RuleSet {
    /// Compiles a parsed configuration root value using the builtin processor registry.
    ///
    /// The root value must be a configuration object. Top-level entries become named
    /// rules, except a top-level object named `context`, whose entries become
    /// lazy defaults available to template references.
    pub fn from_config(config: hocon_rs::Value) -> Result<Self, RenderError> {
        Self::from_config_with_processors(config, ProcessorRegistry::new())
    }

    /// Compiles a parsed configuration root value with additional custom processors.
    ///
    /// Custom processors are merged into the builtin registry before templates
    /// are compiled, so unknown processor names fail during compilation. A
    /// custom processor with the same name as a builtin overrides the builtin.
    pub fn from_config_with_processors(
        config: hocon_rs::Value,
        custom_processors: ProcessorRegistry,
    ) -> Result<Self, RenderError> {
        let hocon_rs::Value::Object(values) = config else {
            return Err(RenderError::InvalidConfigRoot);
        };

        let mut processors = builtin_processors();
        processors.extend(custom_processors);

        let mut document_values = BTreeMap::new();
        let mut text_rules = HashMap::new();
        let mut context_defaults = HashMap::new();

        for (name, value) in values {
            document_values.insert(
                name.clone(),
                value_to_structured_node(value.clone(), &processors)?,
            );
            if name == "context" {
                if let hocon_rs::Value::Object(context_values) = value {
                    for (context_name, context_value) in context_values {
                        insert_named_text_nodes(
                            &mut context_defaults,
                            context_name,
                            context_value,
                            &processors,
                        )?;
                    }
                } else {
                    insert_named_text_nodes(&mut text_rules, name, value, &processors)?;
                }
            } else {
                insert_named_text_nodes(&mut text_rules, name, value, &processors)?;
            }
        }

        Ok(RuleSet {
            document: StructuredNode::Object(document_values),
            text_rules,
            context_defaults,
            processors,
        })
    }

    /// Renders a named rule from this ruleset.
    ///
    /// Each call starts with a fresh render context. Bindings and lazy context
    /// defaults are cached within one render, but not shared with later calls.
    pub fn render_rule(&self, rule_name: &str) -> Result<String, RenderError> {
        self.render_rule_with_context(rule_name, RenderContext::new())
    }

    /// Renders a named rule with initial render context values.
    ///
    /// Initial context values resolve before lazy `context` defaults and named
    /// rules. They are scoped to this render call and are not stored on the
    /// ruleset.
    pub fn render_rule_with_context(
        &self,
        rule_name: &str,
        context: RenderContext,
    ) -> Result<String, RenderError> {
        let mut state = RenderState::with_context(self, context);
        self.render_rule_with_state(rule_name, &mut state)
    }

    /// Renders an object-valued rule as a native structured value.
    ///
    /// Each call starts with a fresh render context. Text leaves within the
    /// structured object share one render state, so bindings and lazy context
    /// defaults are stable within the structured render.
    pub fn render_rule_structured(&self, rule_name: &str) -> Result<CopperlaceValue, RenderError> {
        self.render_rule_structured_with_context(rule_name, RenderContext::new())
    }

    /// Renders an object-valued rule as a native structured value with initial context.
    pub fn render_rule_structured_with_context(
        &self,
        rule_name: &str,
        context: RenderContext,
    ) -> Result<CopperlaceValue, RenderError> {
        let node = self.structured_node(rule_name)?;
        if !matches!(node, StructuredNode::Object(_)) {
            return Err(RenderError::UnsupportedStructuredTarget(
                rule_name.to_string(),
            ));
        }
        let mut state = RenderState::with_context(self, context);
        node.generate_value(&mut state)
    }

    /// Returns the compiled structured document tree.
    pub fn structured_document(&self) -> &StructuredNode {
        &self.document
    }

    fn structured_node(&self, rule_name: &str) -> Result<&StructuredNode, RenderError> {
        let mut node = &self.document;
        for segment in rule_name.split('.') {
            if segment.is_empty() {
                return Err(RenderError::UnknownRule(rule_name.to_string()));
            }
            let StructuredNode::Object(values) = node else {
                return Err(RenderError::UnknownRule(rule_name.to_string()));
            };
            let Some(next_node) = values.get(segment) else {
                return Err(RenderError::UnknownRule(rule_name.to_string()));
            };
            node = next_node;
        }
        Ok(node)
    }

    pub(crate) fn render_rule_with_state(
        &self,
        rule_name: &str,
        state: &mut RenderState,
    ) -> Result<String, RenderError> {
        let Some(rule) = self
            .text_rules
            .get(rule_name)
            .or_else(|| self.context_defaults.get(rule_name))
        else {
            return Err(RenderError::UnknownRule(rule_name.to_string()));
        };

        if state.call_stack.iter().any(|name| name == rule_name) {
            let mut cycle = state.call_stack.clone();
            cycle.push(rule_name.to_string());
            return Err(RenderError::CircularRuleReference(cycle));
        }

        state.call_stack.push(rule_name.to_string());
        let result = rule.generate_text(state);
        state.call_stack.pop();
        result
    }

    pub(crate) fn render_context_default_with_state(
        &self,
        name: &str,
        state: &mut RenderState,
    ) -> Result<Option<String>, RenderError> {
        let Some(rule) = self.context_defaults.get(name) else {
            return Ok(None);
        };

        if state.call_stack.iter().any(|rule_name| rule_name == name) {
            let mut cycle = state.call_stack.clone();
            cycle.push(name.to_string());
            return Err(RenderError::CircularRuleReference(cycle));
        }

        state.call_stack.push(name.to_string());
        let result = rule.generate_text(state);
        state.call_stack.pop();
        result.map(Some)
    }

    pub(crate) fn process(&self, processor_name: &str, value: &str) -> Result<String, RenderError> {
        let Some(processor) = self.processors.get(processor_name) else {
            return Err(RenderError::UnknownProcessor(processor_name.to_string()));
        };

        processor
            .process(value)
            .map_err(|message| RenderError::ProcessorError {
                processor: processor_name.to_string(),
                message,
            })
    }
}

/// Compiles a parsed configuration root value and renders one rule.
///
/// This is a one-shot helper around [`RuleSet::from_config`] and
/// [`RuleSet::render_rule`]. Use [`RuleSet`] directly for repeated renders.
pub fn render_config_rule(config: hocon_rs::Value, rule_name: &str) -> Result<String, RenderError> {
    render_config_rule_with_context(config, rule_name, RenderContext::new())
}

/// Compiles a parsed configuration root value and renders one rule with initial context.
pub fn render_config_rule_with_context(
    config: hocon_rs::Value,
    rule_name: &str,
    context: RenderContext,
) -> Result<String, RenderError> {
    let ruleset = RuleSet::from_config(config)?;
    ruleset.render_rule_with_context(rule_name, context)
}

/// Compiles a parsed configuration root value and renders one object-valued rule.
pub fn render_config_rule_structured(
    config: hocon_rs::Value,
    rule_name: &str,
) -> Result<CopperlaceValue, RenderError> {
    render_config_rule_structured_with_context(config, rule_name, RenderContext::new())
}

/// Compiles a parsed configuration root value and renders one object-valued rule with initial context.
pub fn render_config_rule_structured_with_context(
    config: hocon_rs::Value,
    rule_name: &str,
    context: RenderContext,
) -> Result<CopperlaceValue, RenderError> {
    let ruleset = RuleSet::from_config(config)?;
    ruleset.render_rule_structured_with_context(rule_name, context)
}
