use std::collections::HashMap;
use std::fmt;

use rand::seq::IndexedRandom;
use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    UnknownRule(String),
    EmptyChoice,
    CircularRuleReference(Vec<String>),
    UnsupportedValue(String),
    InvalidConfigRoot,
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::UnknownRule(rule) => write!(formatter, "unknown rule: {rule}"),
            RenderError::EmptyChoice => write!(formatter, "cannot render an empty choice"),
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

pub struct RenderState<'a> {
    ruleset: &'a RuleSet,
    context: HashMap<String, String>,
    call_stack: Vec<String>,
    rng: rand::rngs::ThreadRng,
}

impl<'a> RenderState<'a> {
    fn new(ruleset: &'a RuleSet) -> Self {
        RenderState {
            ruleset,
            context: HashMap::new(),
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
}

impl RuleSet {
    pub fn from_config(config: hocon_rs::Value) -> Result<Self, RenderError> {
        let hocon_rs::Value::Object(values) = config else {
            return Err(RenderError::InvalidConfigRoot);
        };

        let mut rules = HashMap::new();
        let mut context_defaults = HashMap::new();

        for (name, value) in values {
            if name == "context" {
                if let hocon_rs::Value::Object(context_values) = value {
                    for (context_name, context_value) in context_values {
                        context_defaults.insert(context_name, value_to_node(context_value));
                    }
                } else {
                    rules.insert(name, value_to_node(value));
                }
            } else {
                rules.insert(name, value_to_node(value));
            }
        }

        Ok(RuleSet {
            rules,
            context_defaults,
        })
    }

    pub fn render_rule(&self, rule_name: &str) -> Result<String, RenderError> {
        let mut state = RenderState::new(self);
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
}

pub fn render_config_rule(config: hocon_rs::Value, rule_name: &str) -> Result<String, RenderError> {
    let ruleset = RuleSet::from_config(config)?;
    ruleset.render_rule(rule_name)
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
    IfMissing,
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

/// Randomly renders one child node from a list of alternatives.
///
/// This is produced from config arrays. For example, `mood = [happy, sad]`
/// becomes a choice between two literal nodes. If the array is empty, rendering
/// returns `RenderError::EmptyChoice`.
pub struct ChoiceNode {
    nodes: Vec<Box<dyn Node>>,
}

impl ChoiceNode {
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

/// Renders a sequence of child nodes and concatenates their output.
///
/// This is produced from string templates after splitting literal text and
/// `{...}` expressions. For example, `"Hello {name}"` becomes a `VecNode`
/// containing a literal `"Hello "` and a `RuleCallNode` for `name`.
pub struct VecNode {
    nodes: Vec<Box<dyn Node>>,
}

impl VecNode {
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

fn value_to_node(value: hocon_rs::Value) -> Box<dyn Node> {
    match value {
        hocon_rs::Value::String(template) => template_to_node(&template),
        hocon_rs::Value::Array(values) => {
            let nodes = values.into_iter().map(value_to_node).collect();
            Box::new(ChoiceNode::new(nodes))
        }
        hocon_rs::Value::Object(_) => Box::new(UnsupportedValueNode::new("object".to_string())),
        _ => Box::new(value.to_string()),
    }
}

fn template_to_node(template: &str) -> Box<dyn Node> {
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
        nodes.push(expression_to_node(expression));
        cursor = full_match.end();
    }

    if cursor < template.len() {
        nodes.push(Box::new(template[cursor..].to_string()));
    }

    Box::new(VecNode::new(nodes))
}

fn expression_to_node(expression: &str) -> Box<dyn Node> {
    if let Some((name, source)) = expression.split_once(":=") {
        let node = Box::new(RuleCallNode::new(source.trim().to_string()));
        return Box::new(BindNode::new(
            name.trim().to_string(),
            node,
            BindMode::Overwrite,
        ));
    }

    if let Some((name, source)) = expression.split_once(':') {
        let node = Box::new(RuleCallNode::new(source.trim().to_string()));
        return Box::new(BindNode::new(
            name.trim().to_string(),
            node,
            BindMode::IfMissing,
        ));
    }

    Box::new(RuleCallNode::new(expression.to_string()))
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
    pub fn new(value_type: String) -> Self {
        UnsupportedValueNode { value_type }
    }
}

impl Node for UnsupportedValueNode {
    fn render(&self, _state: &mut RenderState) -> Result<String, RenderError> {
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

    #[test]
    fn renders_from_multiple_named_rules() {
        let rules = ruleset(
            r#"
            name = ["Mia"]
            animal = ["owl"]
            story = ["{hero} and {heroPet}"]
            origin = "{hero:name}{heroPet:animal}{story}"
            context = {
                hero = "{name}"
                heroPet = "{animal}"
            }
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "Mia and owl");
        assert_eq!(rules.render_rule("story").unwrap(), "Mia and owl");
        assert_eq!(rules.render_rule("name").unwrap(), "Mia");
        assert_eq!(rules.render_rule("animal").unwrap(), "owl");
    }

    #[test]
    fn binding_reuses_the_same_generated_value() {
        let rules = ruleset(
            r#"
            name = ["Mia"]
            origin = "{hero:name}{hero}/{hero}"
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
    }

    #[test]
    fn binding_does_not_overwrite_existing_value() {
        let rules = ruleset(
            r#"
            first = ["Mia"]
            second = ["Darcy"]
            origin = "{hero:first}{hero:second}{hero}"
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "Mia");
    }

    #[test]
    fn binding_does_not_overwrite_context_default_value() {
        let rules = ruleset(
            r#"
            name = ["Mia"]
            other = ["Darcy"]
            origin = "{hero}{hero:other}/{hero}"
            context = {
                hero = "{name}"
            }
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Mia");
    }

    #[test]
    fn overwrite_binding_replaces_existing_value() {
        let rules = ruleset(
            r#"
            first = ["Mia"]
            second = ["Darcy"]
            origin = "{hero:first}{hero:=second}{hero}"
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "Darcy");
    }

    #[test]
    fn overwrite_binding_replaces_context_default_value() {
        let rules = ruleset(
            r#"
            name = ["Mia"]
            other = ["Darcy"]
            origin = "{hero}{hero:=other}/{hero}"
            context = {
                hero = "{name}"
            }
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "Mia/Darcy");
    }

    #[test]
    fn calls_another_rule_without_eager_expansion() {
        let rules = ruleset(
            r#"
            adjective = ["bright"]
            story = ["A {adjective} path"]
            origin = "{story}"
            "#,
        );

        assert_eq!(rules.render_rule("origin").unwrap(), "A bright path");
    }

    #[test]
    fn unknown_rule_returns_error() {
        let rules = ruleset(
            r#"
            origin = "{missing}"
            "#,
        );

        assert_eq!(
            rules.render_rule("origin"),
            Err(RenderError::UnknownRule("missing".to_string()))
        );
    }

    #[test]
    fn circular_rule_reference_returns_error() {
        let rules = ruleset(
            r#"
            a = "{b}"
            b = "{a}"
            "#,
        );

        assert_eq!(
            rules.render_rule("a"),
            Err(RenderError::CircularRuleReference(vec![
                "a".to_string(),
                "b".to_string(),
                "a".to_string(),
            ]))
        );
    }

    #[test]
    fn empty_choice_returns_error() {
        let rules = ruleset(
            r#"
            origin = []
            "#,
        );

        assert_eq!(rules.render_rule("origin"), Err(RenderError::EmptyChoice));
    }

    #[test]
    fn rendering_object_rule_returns_error() {
        let rules = ruleset(
            r#"
            origin = { value = "nested" }
            "#,
        );

        assert_eq!(
            rules.render_rule("origin"),
            Err(RenderError::UnsupportedValue("object".to_string()))
        );
    }
}
