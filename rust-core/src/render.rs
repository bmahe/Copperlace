use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::seq::IndexedRandom;
use regex::Regex;

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
    /// The root HOCON value was not an object.
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

/// A renderable piece of a compiled rule.
///
/// Nodes are produced from config values and template expressions. Rendering is
/// driven by `RenderState`, which carries the rule table, bound variables, RNG,
/// and rule call stack for cycle detection.
pub trait Node {
    /// Renders this node using the supplied render state.
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError>;
}

/// Literal text node.
///
/// `String` is used for plain template spans such as `"Hello "` and for scalar
/// config values that do not need further expansion. Rendering returns the
/// string unchanged.
impl Node for String {
    fn render(&self, _state: &mut RenderState) -> Result<String, RenderError> {
        Ok(self.clone())
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
    rules: HashMap<String, Box<dyn Node>>,
    context_defaults: HashMap<String, Box<dyn Node>>,
    processors: ProcessorRegistry,
}

impl RuleSet {
    /// Compiles a HOCON root value using the builtin processor registry.
    ///
    /// The root value must be a HOCON object. Top-level entries become named
    /// rules, except a top-level object named `context`, whose entries become
    /// lazy defaults available to template references.
    pub fn from_config(config: hocon_rs::Value) -> Result<Self, RenderError> {
        Self::from_config_with_processors(config, ProcessorRegistry::new())
    }

    /// Compiles a HOCON root value with additional custom processors.
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

        let mut rules = HashMap::new();
        let mut context_defaults = HashMap::new();

        for (name, value) in values {
            if name == "context" {
                if let hocon_rs::Value::Object(context_values) = value {
                    for (context_name, context_value) in context_values {
                        insert_named_nodes(
                            &mut context_defaults,
                            context_name,
                            context_value,
                            &processors,
                        )?;
                    }
                } else {
                    insert_named_nodes(&mut rules, name, value, &processors)?;
                }
            } else {
                insert_named_nodes(&mut rules, name, value, &processors)?;
            }
        }

        Ok(RuleSet {
            rules,
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
            .rules
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
        let result = rule.render(state);
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
        let result = rule.render(state);
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

/// Compiles a HOCON root value and renders one rule.
///
/// This is a one-shot helper around [`RuleSet::from_config`] and
/// [`RuleSet::render_rule`]. Use [`RuleSet`] directly for repeated renders.
pub fn render_config_rule(config: hocon_rs::Value, rule_name: &str) -> Result<String, RenderError> {
    render_config_rule_with_context(config, rule_name, RenderContext::new())
}

/// Compiles a HOCON root value and renders one rule with initial context.
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

impl Node for VariableNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
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

impl Node for RuleCallNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
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
/// This is the node generated for `{alias:rule}` expressions. If `alias` is not
/// already bound, it renders `rule` and stores the result under `alias`. It
/// also supports `{alias:=rule}` expressions, which always render `rule` and
/// overwrite `alias`. Binding expressions always return an empty string so
/// later `{alias}` references reuse the generated value.
pub struct BindNode {
    name: String,
    node: Box<dyn Node>,
    mode: BindMode,
}

impl BindNode {
    /// Creates a binding node for a target name, source node, and binding mode.
    pub fn new(name: String, node: Box<dyn Node>, mode: BindMode) -> Self {
        BindNode { name, node, mode }
    }
}

impl Node for BindNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
        if matches!(self.mode, BindMode::IfMissing) && state.context.contains_key(&self.name) {
            return Ok(String::new());
        }

        let value = self.node.render(state)?;
        state.context.insert(self.name.clone(), value);
        Ok(String::new())
    }
}

/// Applies named processors to a rendered child value from left to right.
pub struct ProcessorPipelineNode {
    node: Box<dyn Node>,
    processors: Vec<String>,
}

impl ProcessorPipelineNode {
    /// Creates a pipeline node that applies processors to the rendered child.
    pub fn new(node: Box<dyn Node>, processors: Vec<String>) -> Self {
        ProcessorPipelineNode { node, processors }
    }
}

impl Node for ProcessorPipelineNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let mut value = self.node.render(state)?;
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
    nodes: Vec<Box<dyn Node>>,
}

impl ChoiceNode {
    /// Creates a choice node from renderable alternatives.
    pub fn new(nodes: Vec<Box<dyn Node>>) -> Self {
        ChoiceNode { nodes }
    }
}

impl Node for ChoiceNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let random_node = self
            .nodes
            .choose(&mut state.rng)
            .ok_or(RenderError::EmptyChoice)?;
        random_node.render(state)
    }
}

/// Randomly renders one child node using per-child weights.
///
/// Weighted choices are produced from arrays containing at least one weighted
/// object entry, such as `{ value = "common", weight = 9 }`. Plain entries in
/// the same array receive weight `1.0`.
pub struct WeightedChoiceNode {
    nodes: Vec<Box<dyn Node>>,
    distribution: WeightedIndex<f64>,
}

impl WeightedChoiceNode {
    /// Creates a weighted choice node from renderable alternatives and weights.
    pub fn new(entries: Vec<(Box<dyn Node>, f64)>) -> Result<Self, RenderError> {
        let (nodes, weights): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let distribution = WeightedIndex::new(weights)
            .map_err(|error| RenderError::InvalidWeightedChoice(error.to_string()))?;
        Ok(WeightedChoiceNode {
            nodes,
            distribution,
        })
    }
}

impl Node for WeightedChoiceNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let index = self.distribution.sample(&mut state.rng);
        self.nodes[index].render(state)
    }
}

/// Renders a sequence of child nodes and concatenates their output.
///
/// This is produced from string templates after splitting literal text and
/// `{...}` expressions. For example, `"Hello {name}"` becomes a `VecNode`
/// containing a literal `"Hello "` and a `RuleCallNode` for `name`.
pub struct VecNode {
    nodes: Vec<Box<dyn Node>>,
}

impl VecNode {
    /// Creates a sequence node that renders children in order.
    pub fn new(nodes: Vec<Box<dyn Node>>) -> Self {
        VecNode { nodes }
    }
}

impl Node for VecNode {
    fn render(&self, state: &mut RenderState) -> Result<String, RenderError> {
        let mut output = String::new();

        for node in &self.nodes {
            output.push_str(&node.render(state)?);
        }

        Ok(output)
    }
}

fn value_to_node(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn Node>, RenderError> {
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

fn insert_named_nodes(
    nodes: &mut HashMap<String, Box<dyn Node>>,
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
                insert_named_nodes(
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
) -> Result<(Box<dyn Node>, f64), RenderError> {
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
) -> Result<Box<dyn Node>, RenderError> {
    let re = Regex::new(r"\{\s*(?<expression>[^\}]*)\s*\}").unwrap();
    let mut nodes: Vec<Box<dyn Node>> = Vec::new();
    let mut cursor = 0;

    for captures in re.captures_iter(template) {
        let Some(full_match) = captures.get(0) else {
            continue;
        };

        if full_match.start() > cursor {
            nodes.push(Box::new(template[cursor..full_match.start()].to_string()));
        }

        let expression = captures
            .name("expression")
            .map(|value| value.as_str().trim())
            .unwrap_or_default();
        nodes.push(expression_to_node(expression, processors)?);
        cursor = full_match.end();
    }

    if cursor < template.len() {
        nodes.push(Box::new(template[cursor..].to_string()));
    }

    Ok(Box::new(VecNode::new(nodes)))
}

fn expression_to_node(
    expression: &str,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn Node>, RenderError> {
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

    if let Some((name, source)) = base_expression.split_once(":=") {
        let name = name.trim();
        let source = source.trim();
        if name.is_empty() || source.is_empty() {
            return Err(RenderError::InvalidExpression(expression.to_string()));
        }
        let node = pipeline_node(
            Box::new(RuleCallNode::new(source.to_string())),
            processor_names,
        );
        return Ok(Box::new(BindNode::new(
            name.trim().to_string(),
            node,
            BindMode::Overwrite,
        )));
    }

    if let Some((name, source)) = base_expression.split_once(':') {
        let name = name.trim();
        let source = source.trim();
        if name.is_empty() || source.is_empty() {
            return Err(RenderError::InvalidExpression(expression.to_string()));
        }
        let node = pipeline_node(
            Box::new(RuleCallNode::new(source.to_string())),
            processor_names,
        );
        return Ok(Box::new(BindNode::new(
            name.trim().to_string(),
            node,
            BindMode::IfMissing,
        )));
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

fn pipeline_node(node: Box<dyn Node>, processors: Vec<String>) -> Box<dyn Node> {
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

impl Node for UnsupportedValueNode {
    fn render(&self, _state: &mut RenderState) -> Result<String, RenderError> {
        Err(RenderError::UnsupportedValue(self.value_type.clone()))
    }
}
