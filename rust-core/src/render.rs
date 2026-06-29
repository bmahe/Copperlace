use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::seq::IndexedRandom;

use crate::processors::builtin_processors;

const WEIGHTED_CHOICE_VALUE_KEY: &str = "value";
const WEIGHTED_CHOICE_WEIGHT_KEY: &str = "weight";

/// Error returned while compiling or rendering Copperlace rules.
#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    /// A template referenced a name that is neither bound nor defined as a rule.
    UnknownRule(String),
    /// A template pipeline referenced a processor that is not registered.
    UnknownProcessor(String),
    /// A registered processor rejected the rendered value.
    ProcessorError { processor: String, message: String },
    /// A `{...}` template expression could not be parsed.
    InvalidExpression(String),
    /// An array-backed choice rule had no alternatives.
    EmptyChoice,
    /// A weighted choice config entry is malformed.
    InvalidWeightedChoice(String),
    /// Rendering detected a recursive rule cycle.
    CircularRuleReference(Vec<String>),
    /// A config value type was parsed but is not renderable.
    UnsupportedValue(String),
    /// The root configuration value was not an object.
    InvalidConfigRoot,
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::UnknownRule(rule) => write!(formatter, "unknown rule: {rule}"),
            RenderError::UnknownProcessor(processor) => {
                write!(formatter, "unknown processor: {processor}")
            }
            RenderError::ProcessorError { processor, message } => {
                write!(formatter, "processor {processor} failed: {message}")
            }
            RenderError::InvalidExpression(expression) => {
                write!(formatter, "invalid template expression: {expression}")
            }
            RenderError::EmptyChoice => write!(formatter, "cannot render an empty choice"),
            RenderError::InvalidWeightedChoice(message) => {
                write!(formatter, "invalid weighted choice: {message}")
            }
            RenderError::CircularRuleReference(cycle) => {
                write!(formatter, "circular rule reference: {}", cycle.join(" -> "))
            }
            RenderError::UnsupportedValue(value_type) => {
                write!(formatter, "unsupported value type: {value_type}")
            }
            RenderError::InvalidConfigRoot => write!(formatter, "config root must be an object"),
        }
    }
}

impl std::error::Error for RenderError {}

/// String transformer used in template processor pipelines.
///
/// Processors receive the rendered output of a rule or binding expression and
/// return the transformed value. Returning `Err` stops rendering and surfaces a
/// [`RenderError::ProcessorError`].
pub trait Processor: Send + Sync {
    /// Transforms one rendered value.
    fn process(&self, value: &str) -> Result<String, String>;
}

impl<F> Processor for F
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    fn process(&self, value: &str) -> Result<String, String> {
        self(value)
    }
}

/// Registry mapping processor names to processor implementations.
///
/// Custom processors registered with [`RuleSet::from_config_with_processors`]
/// extend the builtin registry. If a custom processor uses the same name as a
/// builtin, the custom implementation takes precedence.
pub type ProcessorRegistry = HashMap<String, Arc<dyn Processor>>;

/// Initial variable bindings for one render operation.
///
/// Values in this map are available before top-level `context` defaults and
/// named rules. A render may still update them with overwrite bindings such as
/// `{alias:=rule}`.
pub type RenderContext = HashMap<String, String>;

/// Wraps a processor implementation for insertion into a [`ProcessorRegistry`].
pub fn processor<F>(processor: F) -> Arc<dyn Processor>
where
    F: Processor + 'static,
{
    Arc::new(processor)
}

/// Mutable state for one render operation.
///
/// `RuleSet::render_rule` creates a fresh state for each call. The state tracks
/// per-render bindings, the rule call stack used for cycle detection, and the
/// random number generator used by choice nodes.
pub struct RenderState<'a> {
    ruleset: &'a RuleSet,
    context: RenderContext,
    call_stack: Vec<String>,
    rng: rand::rngs::ThreadRng,
}

impl<'a> RenderState<'a> {
    /// Creates an empty render state for a ruleset.
    pub fn new(ruleset: &'a RuleSet) -> Self {
        Self::with_context(ruleset, RenderContext::new())
    }

    /// Creates a render state with initial variable bindings.
    pub fn with_context(ruleset: &'a RuleSet, context: RenderContext) -> Self {
        RenderState {
            ruleset,
            context,
            call_stack: Vec::new(),
            rng: rand::rngs::ThreadRng::default(),
        }
    }
}

/// A renderable text-generating piece of a compiled rule.
///
/// Nodes are produced from config values, template expressions, and template
/// statements. Text generation is driven by `RenderState`, which carries the
/// rule table, bound variables, RNG, and rule call stack for cycle detection.
pub trait TextGeneratorNode {
    /// Generates text using the supplied render state.
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError>;
}

/// Literal text node.
///
/// `String` is used for plain template spans such as `"Hello "` and for scalar
/// config values that do not need further expansion. Rendering returns the
/// string unchanged.
impl TextGeneratorNode for String {
    fn generate_text(&self, _state: &mut RenderState) -> Result<String, RenderError> {
        Ok(self.clone())
    }
}

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
}

/// Numeric scalar used by structured Copperlace values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CopperlaceNumber {
    /// Integer value representable as `i64`.
    Integer(i64),
    /// Floating-point value representable as finite `f64`.
    Float(f64),
}

impl CopperlaceNumber {
    fn from_json_number(number: serde_json::Number) -> Result<Self, RenderError> {
        if let Some(value) = number.as_i64() {
            return Ok(CopperlaceNumber::Integer(value));
        }
        let Some(value) = number.as_f64() else {
            return Err(RenderError::UnsupportedValue(
                "number must be representable as i64 or f64".to_string(),
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
            CopperlaceNumber::Float(value) => serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        }
    }
}

/// Compiled collection of named rules from the config.
///
/// Top-level config entries become startable rules, so callers can render
/// `origin`, `story`, `name`, or any other named entry directly. A top-level
/// `context` object is treated specially: its entries become lazy defaults for
/// bound variables, so `{hero}` can generate and cache `context.hero` on first
/// use within a render.
pub struct RuleSet {
    #[allow(dead_code)]
    pub(crate) document: StructuredNode,
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

    fn render_rule_with_state(
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

    fn render_context_default_with_state(
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

    fn process(&self, processor_name: &str, value: &str) -> Result<String, RenderError> {
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

/// Looks up a previously bound variable in the current render context.
///
/// This node is useful when a template should require a value that was already
/// bound by a `BindNode`. In the current parser, normal `{name}` expressions use
/// `RuleCallNode` instead, because they can mean either a bound variable or a
/// named rule.
pub struct VariableNode {
    name: String,
}

impl VariableNode {
    /// Creates a variable node that reads a bound value by name.
    pub fn new(name: String) -> Self {
        VariableNode { name }
    }
}

impl TextGeneratorNode for VariableNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        state
            .context
            .get(&self.name)
            .cloned()
            .ok_or_else(|| RenderError::UnknownRule(self.name.clone()))
    }
}

/// Calls another named rule, or reuses a bound/context value with the same name.
///
/// This is the node generated for `{rule}` template expressions. Resolution
/// order is:
/// 1. return an existing bound value from the render context;
/// 2. render and cache a lazy `context` default, if one exists;
/// 3. render the named rule from `RuleSet`.
pub struct RuleCallNode {
    name: String,
}

impl RuleCallNode {
    /// Creates a rule call node for a template reference.
    pub fn new(name: String) -> Self {
        RuleCallNode { name }
    }
}

impl TextGeneratorNode for RuleCallNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        if let Some(value) = state.context.get(&self.name) {
            return Ok(value.clone());
        }

        if let Some(value) = state
            .ruleset
            .render_context_default_with_state(&self.name, state)?
        {
            state.context.insert(self.name.clone(), value.clone());
            return Ok(value);
        }

        state.ruleset.render_rule_with_state(&self.name, state)
    }
}

/// Controls whether a binding expression preserves or overwrites an existing
/// value in the render context.
pub enum BindMode {
    /// Preserve an existing binding and bind only when the name is missing.
    IfMissing,
    /// Always render the source and replace any existing binding.
    Overwrite,
}

/// Binds the output of a child node into the render context without emitting it.
///
/// This is the node generated for `{% alias:rule %}` statements. If `alias` is
/// not already bound, it renders `rule` and stores the result under `alias`. It
/// also supports `{% alias:=rule %}` statements, which always render `rule` and
/// overwrite `alias`. Binding statements always return an empty string so later
/// `{alias}` references reuse the generated value.
pub struct BindNode {
    name: String,
    node: Box<dyn TextGeneratorNode>,
    mode: BindMode,
}

impl BindNode {
    /// Creates a binding node for a target name, source node, and binding mode.
    pub fn new(name: String, node: Box<dyn TextGeneratorNode>, mode: BindMode) -> Self {
        BindNode { name, node, mode }
    }
}

impl TextGeneratorNode for BindNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        if matches!(self.mode, BindMode::IfMissing) && state.context.contains_key(&self.name) {
            return Ok(String::new());
        }

        let value = self.node.generate_text(state)?;
        state.context.insert(self.name.clone(), value);
        Ok(String::new())
    }
}

/// Applies named processors to a rendered child value from left to right.
pub struct ProcessorPipelineNode {
    node: Box<dyn TextGeneratorNode>,
    processors: Vec<String>,
}

impl ProcessorPipelineNode {
    /// Creates a pipeline node that applies processors to the rendered child.
    pub fn new(node: Box<dyn TextGeneratorNode>, processors: Vec<String>) -> Self {
        ProcessorPipelineNode { node, processors }
    }
}

impl TextGeneratorNode for ProcessorPipelineNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let mut value = self.node.generate_text(state)?;
        for processor_name in &self.processors {
            value = state.ruleset.process(processor_name, &value)?;
        }
        Ok(value)
    }
}

/// Randomly renders one child node from a list of alternatives.
///
/// This is produced from config arrays. For example, `mood = [happy, sad]`
/// becomes a choice between two literal nodes. If the array is empty, rendering
/// returns `RenderError::EmptyChoice`.
pub struct ChoiceNode {
    nodes: Vec<Box<dyn TextGeneratorNode>>,
}

impl ChoiceNode {
    /// Creates a choice node from renderable alternatives.
    pub fn new(nodes: Vec<Box<dyn TextGeneratorNode>>) -> Self {
        ChoiceNode { nodes }
    }
}

impl TextGeneratorNode for ChoiceNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let random_node = self
            .nodes
            .choose(&mut state.rng)
            .ok_or(RenderError::EmptyChoice)?;
        random_node.generate_text(state)
    }
}

/// Randomly renders one child node using per-child weights.
///
/// Weighted choices are produced from arrays containing at least one weighted
/// object entry, such as `{ value = "common", weight = 9 }`. Plain entries in
/// the same array receive weight `1.0`.
pub struct WeightedChoiceNode {
    nodes: Vec<Box<dyn TextGeneratorNode>>,
    distribution: WeightedIndex<f64>,
}

impl WeightedChoiceNode {
    /// Creates a weighted choice node from renderable alternatives and weights.
    pub fn new(entries: Vec<(Box<dyn TextGeneratorNode>, f64)>) -> Result<Self, RenderError> {
        let (nodes, weights): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let distribution = WeightedIndex::new(weights)
            .map_err(|error| RenderError::InvalidWeightedChoice(error.to_string()))?;
        Ok(WeightedChoiceNode {
            nodes,
            distribution,
        })
    }
}

impl TextGeneratorNode for WeightedChoiceNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let index = self.distribution.sample(&mut state.rng);
        self.nodes[index].generate_text(state)
    }
}

/// Renders a sequence of child nodes and concatenates their output.
///
/// This is produced from string templates after splitting literal text,
/// `{...}` expressions, and `{% ... %}` statements. For example,
/// `"Hello {name}"` becomes a `VecNode` containing a literal `"Hello "` and a
/// `RuleCallNode` for `name`.
pub struct VecNode {
    nodes: Vec<Box<dyn TextGeneratorNode>>,
}

impl VecNode {
    /// Creates a sequence node that renders children in order.
    pub fn new(nodes: Vec<Box<dyn TextGeneratorNode>>) -> Self {
        VecNode { nodes }
    }
}

impl TextGeneratorNode for VecNode {
    fn generate_text(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let mut output = String::new();

        for node in &self.nodes {
            output.push_str(&node.generate_text(state)?);
        }

        Ok(output)
    }
}

fn value_to_node(
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

fn value_to_structured_node(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
) -> Result<StructuredNode, RenderError> {
    Ok(match value {
        hocon_rs::Value::Object(values) => {
            let mut nodes = BTreeMap::new();
            for (name, value) in values {
                nodes.insert(name, value_to_structured_node(value, processors)?);
            }
            StructuredNode::Object(nodes)
        }
        hocon_rs::Value::Array(values) => StructuredNode::Array(
            values
                .into_iter()
                .map(|value| value_to_structured_node(value, processors))
                .collect::<Result<Vec<_>, _>>()?,
        ),
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

fn insert_named_text_nodes(
    nodes: &mut HashMap<String, Box<dyn TextGeneratorNode>>,
    name: String,
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
) -> Result<(), RenderError> {
    match value {
        hocon_rs::Value::Object(values) => {
            nodes.insert(
                name.clone(),
                Box::new(UnsupportedValueNode::new("object".to_string())),
            );
            for (child_name, child_value) in values {
                insert_named_text_nodes(
                    nodes,
                    format!("{name}.{child_name}"),
                    child_value,
                    processors,
                )?;
            }
        }
        value => {
            nodes.insert(name, value_to_node(value, processors)?);
        }
    }

    Ok(())
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

fn template_to_node(
    template: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let mut nodes: Vec<Box<dyn TextGeneratorNode>> = Vec::new();
    let mut literal = String::new();
    let mut chars = template.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '\\' => match chars.peek() {
                Some((_, next_character)) if matches!(next_character, '{' | '}') => {
                    literal.push(*next_character);
                    chars.next();
                }
                _ => literal.push(character),
            },
            '{' => {
                if !literal.is_empty() {
                    nodes.push(Box::new(std::mem::take(&mut literal)));
                }

                if let Some((_, '%')) = chars.peek() {
                    chars.next();
                    let statement_start = index + character.len_utf8() + '%'.len_utf8();
                    let mut statement_end = None;
                    let mut previous_percent_index = None;
                    for (statement_index, statement_character) in chars.by_ref() {
                        if statement_character == '}'
                            && let Some(percent_index) = previous_percent_index
                        {
                            statement_end = Some(percent_index);
                            break;
                        }
                        previous_percent_index = if statement_character == '%' {
                            Some(statement_index)
                        } else {
                            None
                        };
                    }

                    let Some(statement_end) = statement_end else {
                        return Err(RenderError::InvalidExpression(
                            "unmatched opening statement delimiter in template".to_string(),
                        ));
                    };

                    let statement = template[statement_start..statement_end].trim();
                    nodes.push(statement_to_node(statement, processors)?);
                } else {
                    let expression_start = index + character.len_utf8();
                    let mut expression_end = None;
                    for (expression_index, expression_character) in chars.by_ref() {
                        if expression_character == '}' {
                            expression_end = Some(expression_index);
                            break;
                        }
                    }

                    let Some(expression_end) = expression_end else {
                        return Err(RenderError::InvalidExpression(
                            "unmatched opening brace in template".to_string(),
                        ));
                    };

                    let expression = template[expression_start..expression_end].trim();
                    nodes.push(expression_to_node(expression, processors)?);
                }
            }
            '%' => {
                if let Some((_, '}')) = chars.peek() {
                    return Err(RenderError::InvalidExpression(
                        "unmatched closing statement delimiter in template".to_string(),
                    ));
                } else {
                    literal.push(character);
                }
            }
            '}' => {
                return Err(RenderError::InvalidExpression(
                    "unmatched closing brace in template".to_string(),
                ));
            }
            _ => literal.push(character),
        }
    }

    if !literal.is_empty() {
        nodes.push(Box::new(literal));
    }

    Ok(Box::new(VecNode::new(nodes)))
}

fn statement_to_node(
    statement: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let (base_expression, processor_names) = parse_pipeline(statement, processors)?;

    if let Some((name, source)) = base_expression.split_once(":=") {
        return bind_node(
            statement,
            name,
            source,
            processor_names,
            BindMode::Overwrite,
        );
    }

    if let Some((name, source)) = base_expression.split_once(':') {
        return bind_node(
            statement,
            name,
            source,
            processor_names,
            BindMode::IfMissing,
        );
    }

    Err(RenderError::InvalidExpression(statement.to_string()))
}

fn expression_to_node(
    expression: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let (base_expression, processor_names) = parse_pipeline(expression, processors)?;

    if base_expression.contains(':') {
        return Err(RenderError::InvalidExpression(expression.to_string()));
    }

    let name = base_expression.trim();
    if name.is_empty() {
        return Err(RenderError::InvalidExpression(expression.to_string()));
    }
    Ok(pipeline_node(
        Box::new(RuleCallNode::new(name.to_string())),
        processor_names,
    ))
}

fn parse_pipeline<'a>(
    expression: &'a str,
    processors: &ProcessorRegistry,
) -> Result<(&'a str, Vec<String>), RenderError> {
    let mut parts = expression.split('|').map(str::trim);
    let base_expression = parts
        .next()
        .filter(|base_expression| !base_expression.is_empty())
        .ok_or_else(|| RenderError::InvalidExpression(expression.to_string()))?;
    let processor_names = parts
        .map(|processor_name| {
            if processor_name.is_empty() {
                return Err(RenderError::InvalidExpression(expression.to_string()));
            }
            if !processors.contains_key(processor_name) {
                return Err(RenderError::UnknownProcessor(processor_name.to_string()));
            }
            Ok(processor_name.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((base_expression, processor_names))
}

fn bind_node(
    expression: &str,
    name: &str,
    source: &str,
    processor_names: Vec<String>,
    mode: BindMode,
) -> Result<Box<dyn TextGeneratorNode>, RenderError> {
    let name = name.trim();
    let source = source.trim();
    if name.is_empty() || source.is_empty() {
        return Err(RenderError::InvalidExpression(expression.to_string()));
    }
    let node = pipeline_node(
        Box::new(RuleCallNode::new(source.to_string())),
        processor_names,
    );
    Ok(Box::new(BindNode::new(name.to_string(), node, mode)))
}

fn pipeline_node(
    node: Box<dyn TextGeneratorNode>,
    processors: Vec<String>,
) -> Box<dyn TextGeneratorNode> {
    if processors.is_empty() {
        node
    } else {
        Box::new(ProcessorPipelineNode::new(node, processors))
    }
}

/// Placeholder node for config value types that are not renderable yet.
///
/// Object values currently compile to this node unless they are the special
/// top-level `context` object handled by `RuleSet::from_config`. Rendering this
/// node returns `RenderError::UnsupportedValue`.
pub struct UnsupportedValueNode {
    value_type: String,
}

impl UnsupportedValueNode {
    /// Creates a node that reports an unsupported config value type at render time.
    pub fn new(value_type: String) -> Self {
        UnsupportedValueNode { value_type }
    }
}

impl TextGeneratorNode for UnsupportedValueNode {
    fn generate_text(&self, _state: &mut RenderState) -> Result<String, RenderError> {
        Err(RenderError::UnsupportedValue(self.value_type.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ruleset(config: &str) -> RuleSet {
        let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None).unwrap();
        RuleSet::from_config(value).unwrap()
    }

    fn root_object(rules: &RuleSet) -> &BTreeMap<String, StructuredNode> {
        let StructuredNode::Object(values) = &rules.document else {
            panic!("expected root object");
        };
        values
    }

    #[test]
    fn top_level_list_compiles_to_structured_array_and_text_choice() {
        let rules = ruleset(
            r#"
            origin = ["red", "blue"]
            "#,
        );

        let root = root_object(&rules);
        let StructuredNode::Array(values) = root.get("origin").unwrap() else {
            panic!("expected structured array");
        };
        assert_eq!(values.len(), 2);
        assert!(matches!(values[0], StructuredNode::Text(_)));
        assert!(matches!(values[1], StructuredNode::Text(_)));

        let output = rules.render_rule("origin").unwrap();
        assert!(["red", "blue"].contains(&output.as_str()));
    }

    #[test]
    fn object_values_compile_to_structured_objects_and_dotted_text_rules() {
        let rules = ruleset(
            r#"
            origin {
                title = "Scene"
                nested {
                    mood = "Quiet"
                }
            }
            "#,
        );

        let root = root_object(&rules);
        let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
            panic!("expected structured object");
        };
        assert!(matches!(
            origin.get("title").unwrap(),
            StructuredNode::Text(_)
        ));
        assert!(matches!(
            origin.get("nested").unwrap(),
            StructuredNode::Object(_)
        ));

        assert_eq!(rules.render_rule("origin.title").unwrap(), "Scene");
        assert_eq!(rules.render_rule("origin.nested.mood").unwrap(), "Quiet");
        assert_eq!(
            rules.render_rule("origin"),
            Err(RenderError::UnsupportedValue("object".to_string()))
        );
    }

    #[test]
    fn structured_arrays_inside_objects_do_not_compile_as_choices() {
        let rules = ruleset(
            r#"
            origin {
                entries = [
                    { value = "common", weight = 1 },
                    { value = "rare", weight = 2 }
                ]
            }
            "#,
        );

        let root = root_object(&rules);
        let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
            panic!("expected structured object");
        };
        let StructuredNode::Array(entries) = origin.get("entries").unwrap() else {
            panic!("expected structured array");
        };
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], StructuredNode::Object(_)));
        assert!(matches!(entries[1], StructuredNode::Object(_)));
    }

    #[test]
    fn structured_scalars_compile_to_native_scalar_nodes() {
        let rules = ruleset(
            r#"
            origin {
                count = 3
                ratio = 2.5
                active = true
                missing = null
            }
            "#,
        );

        let root = root_object(&rules);
        let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
            panic!("expected structured object");
        };
        assert!(matches!(
            origin.get("count").unwrap(),
            StructuredNode::Number(CopperlaceNumber::Integer(3))
        ));
        assert!(matches!(
            origin.get("ratio").unwrap(),
            StructuredNode::Number(CopperlaceNumber::Float(2.5))
        ));
        assert!(matches!(
            origin.get("active").unwrap(),
            StructuredNode::Boolean(true)
        ));
        assert!(matches!(
            origin.get("missing").unwrap(),
            StructuredNode::Null
        ));
    }

    #[test]
    fn structured_text_leaves_use_text_generator_nodes() {
        let rules = ruleset(
            r#"
            name = "Mia"
            origin {
                title = "Hello {name}"
            }
            "#,
        );

        let root = root_object(&rules);
        let StructuredNode::Object(origin) = root.get("origin").unwrap() else {
            panic!("expected structured object");
        };
        let StructuredNode::Text(node) = origin.get("title").unwrap() else {
            panic!("expected text leaf");
        };
        let mut state = RenderState::new(&rules);
        assert_eq!(node.generate_text(&mut state).unwrap(), "Hello Mia");
    }

    #[test]
    fn copperlace_value_converts_to_json_values() {
        let mut nested = BTreeMap::new();
        nested.insert(
            "array".to_string(),
            CopperlaceValue::Array(vec![
                CopperlaceValue::String("Mia".to_string()),
                CopperlaceValue::Number(CopperlaceNumber::Integer(3)),
                CopperlaceValue::Number(CopperlaceNumber::Float(2.5)),
                CopperlaceValue::Boolean(true),
                CopperlaceValue::Null,
            ]),
        );
        let value = CopperlaceValue::Object(nested);

        assert_eq!(
            value.to_json_value(),
            serde_json::json!({
                "array": ["Mia", 3, 2.5, true, null]
            })
        );
        assert_eq!(
            value.into_json_value(),
            serde_json::json!({
                "array": ["Mia", 3, 2.5, true, null]
            })
        );
    }
}
