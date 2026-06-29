use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::seq::IndexedRandom;

use super::error::RenderError;
use super::state::RenderState;

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
