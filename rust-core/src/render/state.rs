use std::collections::{HashMap, HashSet};

use super::error::RenderError;
use super::ruleset::RuleSet;
use super::value::{CopperlaceNumber, CopperlaceValue, StructuredNode};

/// Initial variable bindings for one render operation.
///
/// Values in this map are available before top-level `context` defaults and
/// named rules. A render may still update them with overwrite bindings such as
/// `{alias:=rule}`.
pub type RenderContext = HashMap<String, String>;

/// Render-time options that affect rule expansion behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderOptions {
    /// Maximum recursive re-entries allowed for one rule name.
    ///
    /// A value of `0` preserves the default behavior: re-entering a rule that is
    /// already on the call stack returns `CircularRuleReference`. Values greater
    /// than zero allow that many recursive re-entries before recursive calls
    /// return an empty string.
    pub max_recursion_depth: usize,
}

/// Mutable state for one render operation.
///
/// `RuleSet::render_rule` creates a fresh state for each call. The state tracks
/// per-render bindings, the rule call stack used for cycle detection, and the
/// random number generator used by choice nodes.
pub struct RenderState<'a> {
    pub(crate) ruleset: &'a RuleSet,
    context: RenderContext,
    scopes: Vec<IterationScope<'a>>,
    pub(crate) options: RenderOptions,
    pub(crate) call_stack: Vec<String>,
    scoped_call_stack: Vec<usize>,
    pub(crate) unique_choices: HashMap<String, HashSet<usize>>,
    pub(crate) rng: rand::rngs::ThreadRng,
}

#[derive(Clone, Copy)]
pub(crate) struct LoopMetadata {
    pub(crate) index: usize,
    pub(crate) length: usize,
}

struct IterationScope<'a> {
    variable_name: String,
    value: IterationValue<'a>,
    metadata: LoopMetadata,
    bindings: RenderContext,
}

enum ScopedValue<'a> {
    Text(String),
    Node(&'a StructuredNode),
    Runtime(CopperlaceValue),
    MetadataObject,
}

pub(crate) enum IterationValue<'a> {
    Compiled(&'a StructuredNode),
    Runtime(CopperlaceValue),
}

impl<'a> RenderState<'a> {
    /// Creates an empty render state for a ruleset.
    pub fn new(ruleset: &'a RuleSet) -> Self {
        Self::with_context(ruleset, RenderContext::new())
    }

    /// Creates a render state with initial variable bindings.
    pub fn with_context(ruleset: &'a RuleSet, context: RenderContext) -> Self {
        Self::with_context_and_options(ruleset, context, RenderOptions::default())
    }

    /// Creates a render state with initial variable bindings and render options.
    pub fn with_context_and_options(
        ruleset: &'a RuleSet,
        context: RenderContext,
        options: RenderOptions,
    ) -> Self {
        RenderState {
            ruleset,
            context,
            scopes: Vec::new(),
            options,
            call_stack: Vec::new(),
            scoped_call_stack: Vec::new(),
            unique_choices: HashMap::new(),
            rng: rand::rngs::ThreadRng::default(),
        }
    }

    pub(crate) fn resolve_bound_text(&mut self, name: &str) -> Result<Option<String>, RenderError> {
        let Some(value) = self.scoped_value(name)? else {
            return Ok(None);
        };

        match value {
            ScopedValue::Text(value) => Ok(Some(value)),
            ScopedValue::Node(node) => self.render_scoped_node(name, node).map(Some),
            ScopedValue::Runtime(value) => value.to_rendered_text().map(Some),
            ScopedValue::MetadataObject => Err(RenderError::UnsupportedValue("object".to_string())),
        }
    }

    pub(crate) fn contains_bound_value(&self, name: &str) -> Result<bool, RenderError> {
        self.scoped_value(name).map(|value| value.is_some())
    }

    pub(crate) fn bind(
        &mut self,
        name: &str,
        value: String,
        overwrite: bool,
    ) -> Result<(), RenderError> {
        if overwrite && self.is_immutable_loop_name(name) {
            return Err(RenderError::ImmutableLoopBinding(name.to_string()));
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), value);
        } else {
            self.context.insert(name.to_string(), value);
        }
        Ok(())
    }

    pub(crate) fn ensure_mutable_binding(&self, name: &str) -> Result<(), RenderError> {
        if self.is_immutable_loop_name(name) {
            return Err(RenderError::ImmutableLoopBinding(name.to_string()));
        }
        Ok(())
    }

    pub(crate) fn cache_context_default(&mut self, name: &str, value: String) {
        self.context.insert(name.to_string(), value);
    }

    pub(crate) fn iterable_elements(
        &self,
        source: &str,
    ) -> Result<Vec<IterationValue<'a>>, RenderError> {
        let value = match self.scoped_value(source)? {
            Some(value) => value,
            None => {
                if let Some(node) = self.ruleset.structured_context_node(source) {
                    ScopedValue::Node(node)
                } else {
                    ScopedValue::Node(self.ruleset.structured_node(source)?)
                }
            }
        };

        match value {
            ScopedValue::Node(StructuredNode::Array(values)) => values
                .iter()
                .map(|entry| {
                    entry
                        .value_node()
                        .map(IterationValue::Compiled)
                        .ok_or_else(|| RenderError::UnsupportedIterationSource {
                            source: source.to_string(),
                            value_type: "array with generated entries".to_string(),
                        })
                })
                .collect(),
            ScopedValue::Node(node) => Err(RenderError::UnsupportedIterationSource {
                source: source.to_string(),
                value_type: node.value_type().to_string(),
            }),
            ScopedValue::Text(_) => Err(RenderError::UnsupportedIterationSource {
                source: source.to_string(),
                value_type: "string".to_string(),
            }),
            ScopedValue::MetadataObject => Err(RenderError::UnsupportedIterationSource {
                source: source.to_string(),
                value_type: "object".to_string(),
            }),
            ScopedValue::Runtime(CopperlaceValue::Array(values)) => {
                Ok(values.into_iter().map(IterationValue::Runtime).collect())
            }
            ScopedValue::Runtime(value) => Err(RenderError::UnsupportedIterationSource {
                source: source.to_string(),
                value_type: value.value_type().to_string(),
            }),
        }
    }

    pub(crate) fn push_iteration_scope(
        &mut self,
        variable_name: &str,
        value: IterationValue<'a>,
        metadata: LoopMetadata,
    ) {
        self.scopes.push(IterationScope {
            variable_name: variable_name.to_string(),
            value,
            metadata,
            bindings: RenderContext::new(),
        });
    }

    pub(crate) fn pop_iteration_scope(&mut self) {
        self.scopes.pop();
    }

    fn scoped_value(&self, name: &str) -> Result<Option<ScopedValue<'a>>, RenderError> {
        for scope in self.scopes.iter().rev() {
            if name == scope.variable_name {
                return Ok(Some(match &scope.value {
                    IterationValue::Compiled(value) => ScopedValue::Node(value),
                    IterationValue::Runtime(value) => ScopedValue::Runtime(value.clone()),
                }));
            }
            if let Some(path) = name
                .strip_prefix(&scope.variable_name)
                .and_then(|suffix| suffix.strip_prefix('.'))
            {
                return match &scope.value {
                    IterationValue::Compiled(value) => value
                        .child(path)
                        .map(ScopedValue::Node)
                        .map(Some)
                        .ok_or_else(|| RenderError::UnknownRule(name.to_string())),
                    IterationValue::Runtime(value) => value
                        .child(path)
                        .cloned()
                        .map(ScopedValue::Runtime)
                        .map(Some)
                        .ok_or_else(|| RenderError::UnknownRule(name.to_string())),
                };
            }

            if name == "loop" {
                return Ok(Some(ScopedValue::MetadataObject));
            }
            if let Some(field) = name.strip_prefix("loop.") {
                return self.metadata_value(name, field, scope.metadata).map(Some);
            }

            if let Some(value) = scope.bindings.get(name) {
                return Ok(Some(ScopedValue::Text(value.clone())));
            }
        }

        Ok(self.context.get(name).cloned().map(ScopedValue::Text))
    }

    fn metadata_value(
        &self,
        full_name: &str,
        field: &str,
        metadata: LoopMetadata,
    ) -> Result<ScopedValue<'a>, RenderError> {
        let value = match field {
            "index" => {
                CopperlaceValue::Number(CopperlaceNumber::Unsigned((metadata.index + 1) as u64))
            }
            "index0" => CopperlaceValue::Number(CopperlaceNumber::Unsigned(metadata.index as u64)),
            "length" => CopperlaceValue::Number(CopperlaceNumber::Unsigned(metadata.length as u64)),
            "first" => CopperlaceValue::Boolean(metadata.index == 0),
            "last" => CopperlaceValue::Boolean(metadata.index + 1 == metadata.length),
            _ => return Err(RenderError::UnknownRule(full_name.to_string())),
        };
        Ok(ScopedValue::Runtime(value))
    }

    fn render_scoped_node(
        &mut self,
        name: &str,
        node: &'a StructuredNode,
    ) -> Result<String, RenderError> {
        let identity = node as *const StructuredNode as usize;
        let existing_calls = self
            .scoped_call_stack
            .iter()
            .filter(|entry| **entry == identity)
            .count();
        if self.options.max_recursion_depth == 0 && existing_calls > 0 {
            let mut cycle = self.call_stack.clone();
            cycle.push(name.to_string());
            return Err(RenderError::CircularRuleReference(cycle));
        }
        if existing_calls > self.options.max_recursion_depth {
            return Ok(String::new());
        }

        self.scoped_call_stack.push(identity);
        self.call_stack.push(name.to_string());
        let result = node.generate_text(self);
        self.call_stack.pop();
        self.scoped_call_stack.pop();
        result
    }

    fn is_immutable_loop_name(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| {
            name == scope.variable_name
                || name.starts_with(&format!("{}.", scope.variable_name))
                || name == "loop"
                || name.starts_with("loop.")
        })
    }

    pub(crate) fn used_unique_choice_indices(&self, rule_name: &str) -> Option<&HashSet<usize>> {
        self.unique_choices.get(rule_name)
    }

    pub(crate) fn mark_unique_choice_index(&mut self, rule_name: &str, index: usize) {
        self.unique_choices
            .entry(rule_name.to_string())
            .or_default()
            .insert(index);
    }
}
