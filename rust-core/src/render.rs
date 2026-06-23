use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use rand::seq::IndexedRandom;
use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    UnknownRule(String),
    UnknownProcessor(String),
    ProcessorError { processor: String, message: String },
    InvalidExpression(String),
    EmptyChoice,
    CircularRuleReference(Vec<String>),
    UnsupportedValue(String),
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

pub trait Processor: Send + Sync {
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

pub type ProcessorRegistry = HashMap<String, Arc<dyn Processor>>;

pub fn processor<F>(processor: F) -> Arc<dyn Processor>
where
    F: Processor + 'static,
{
    Arc::new(processor)
}

fn builtin_processors() -> ProcessorRegistry {
    let mut processors = ProcessorRegistry::new();
    processors.insert(
        "uppercase".to_string(),
        processor(|value: &str| Ok(value.to_uppercase())),
    );
    processors.insert(
        "lowercase".to_string(),
        processor(|value: &str| Ok(value.to_lowercase())),
    );
    processors.insert(
        "trim".to_string(),
        processor(|value: &str| Ok(value.trim().to_string())),
    );
    processors.insert("capitalize".to_string(), processor(capitalize));
    processors.insert("titlecase".to_string(), processor(titlecase));
    processors.insert("article".to_string(), processor(article));
    processors.insert("past_tense".to_string(), processor(past_tense));
    processors.insert("pluralize".to_string(), processor(pluralize));
    processors.insert("singularize".to_string(), processor(singularize));
    processors.insert("possessive".to_string(), processor(possessive));
    processors.insert(
        "present_participle".to_string(),
        processor(present_participle),
    );
    processors
}

fn capitalize(value: &str) -> Result<String, String> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Ok(String::new());
    };

    let mut output = String::new();
    output.extend(first.to_uppercase());
    output.push_str(&chars.as_str().to_lowercase());
    Ok(output)
}

fn titlecase(value: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut start_of_word = true;

    for character in value.chars() {
        if character.is_whitespace() {
            start_of_word = true;
            output.push(character);
        } else if start_of_word {
            output.extend(character.to_uppercase());
            start_of_word = false;
        } else {
            output.extend(character.to_lowercase());
        }
    }

    Ok(output)
}

fn article(value: &str) -> Result<String, String> {
    let article = if uses_an(value) { "an" } else { "a" };
    Ok(format!("{article} {value}"))
}

fn uses_an(value: &str) -> bool {
    let token = value.split_whitespace().next().unwrap_or("");
    if token.is_empty() {
        return false;
    }

    let token = token.trim_matches(|character: char| {
        !character.is_alphanumeric() && character != '\'' && character != '-'
    });
    if token.is_empty() {
        return false;
    }

    if starts_with_vowel_sound_number(token) {
        return true;
    }

    let lowercase = token.to_lowercase();
    if starts_with_silent_h(&lowercase) {
        return true;
    }
    if starts_with_hard_vowel_sound(&lowercase) {
        return false;
    }
    if is_initialism(token) {
        return starts_with_vowel_sound_initial(token);
    }

    matches!(lowercase.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'))
}

fn starts_with_silent_h(value: &str) -> bool {
    ["heir", "honest", "honor", "honour", "hour"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
}

fn starts_with_hard_vowel_sound(value: &str) -> bool {
    [
        "euro",
        "one",
        "ubiquit",
        "uk",
        "unanim",
        "unic",
        "uniform",
        "union",
        "unique",
        "unit",
        "university",
        "use",
        "user",
        "usual",
        "utensil",
        "utility",
        "utopia",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix))
}

fn starts_with_vowel_sound_number(value: &str) -> bool {
    value.starts_with('8') || value.starts_with("11") || value.starts_with("18")
}

fn is_initialism(value: &str) -> bool {
    let letters: Vec<char> = value
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect();
    !letters.is_empty() && letters.iter().all(|character| character.is_uppercase())
}

fn starts_with_vowel_sound_initial(value: &str) -> bool {
    matches!(
        value.chars().find(|character| character.is_alphabetic()),
        Some('A' | 'E' | 'F' | 'H' | 'I' | 'L' | 'M' | 'N' | 'O' | 'R' | 'S' | 'X')
    )
}

fn past_tense(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("input must contain one verb".to_string());
    }
    if trimmed.split_whitespace().count() != 1 {
        return Err("input must contain exactly one verb token".to_string());
    }

    let leading_len = value.len() - value.trim_start().len();
    let trailing_len = value.len() - value.trim_end().len();
    let leading = &value[..leading_len];
    let trailing = &value[value.len() - trailing_len..];
    let tense = apply_case_style(trimmed, &past_tense_lowercase(&trimmed.to_lowercase()));

    Ok(format!("{leading}{tense}{trailing}"))
}

fn pluralize(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "noun")?;
    let plural = apply_case_style(token, &pluralize_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{plural}{trailing}"))
}

fn pluralize_lowercase(value: &str) -> String {
    if let Some(irregular) = irregular_plural(value) {
        return irregular.to_string();
    }

    if let Some(stem) = value.strip_suffix("fe") {
        return format!("{stem}ves");
    }

    if let Some(stem) = value.strip_suffix('f') {
        return format!("{stem}ves");
    }

    if let Some(stem) = value.strip_suffix('y') {
        if stem
            .chars()
            .last()
            .is_some_and(|character| is_consonant(character))
        {
            return format!("{stem}ies");
        }
    }

    if value.ends_with('s')
        || value.ends_with('x')
        || value.ends_with('z')
        || value.ends_with("ch")
        || value.ends_with("sh")
    {
        return format!("{value}es");
    }

    format!("{value}s")
}

fn irregular_plural(value: &str) -> Option<&'static str> {
    match value {
        "person" => Some("people"),
        "child" => Some("children"),
        "mouse" => Some("mice"),
        "goose" => Some("geese"),
        "man" => Some("men"),
        "woman" => Some("women"),
        "tooth" => Some("teeth"),
        "foot" => Some("feet"),
        "ox" => Some("oxen"),
        _ => None,
    }
}

fn singularize(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "noun")?;
    let singular = apply_case_style(token, &singularize_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{singular}{trailing}"))
}

fn singularize_lowercase(value: &str) -> String {
    if let Some(irregular) = irregular_singular(value) {
        return irregular.to_string();
    }

    if let Some(stem) = value.strip_suffix("ies") {
        return format!("{stem}y");
    }

    if let Some(stem) = value.strip_suffix("ves") {
        return format!("{stem}f");
    }

    if value.ends_with("ches")
        || value.ends_with("shes")
        || value.ends_with("xes")
        || value.ends_with("ses")
        || value.ends_with("zes")
    {
        return value
            .strip_suffix("es")
            .expect("suffix was checked")
            .to_string();
    }

    if value.len() > 1 {
        if let Some(stem) = value.strip_suffix('s') {
            return stem.to_string();
        }
    }

    value.to_string()
}

fn irregular_singular(value: &str) -> Option<&'static str> {
    match value {
        "people" => Some("person"),
        "children" => Some("child"),
        "mice" => Some("mouse"),
        "geese" => Some("goose"),
        "men" => Some("man"),
        "women" => Some("woman"),
        "teeth" => Some("tooth"),
        "feet" => Some("foot"),
        "oxen" => Some("ox"),
        _ => None,
    }
}

fn possessive(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "name")?;
    let suffix = if token.ends_with('s') { "'" } else { "'s" };

    Ok(format!("{leading}{token}{suffix}{trailing}"))
}

fn present_participle(value: &str) -> Result<String, String> {
    let (leading, token, trailing) = single_token_parts(value, "verb")?;
    let participle = apply_case_style(token, &present_participle_lowercase(&token.to_lowercase()));

    Ok(format!("{leading}{participle}{trailing}"))
}

fn present_participle_lowercase(value: &str) -> String {
    if let Some(stem) = value.strip_suffix("ie") {
        return format!("{stem}ying");
    }

    if value.ends_with('e')
        && !value.ends_with("ee")
        && !value.ends_with("ye")
        && !value.ends_with("oe")
    {
        return format!(
            "{}ing",
            value.strip_suffix('e').expect("suffix was checked")
        );
    }

    if should_double_final_consonant(value) {
        let final_character = value.chars().last().expect("value is not empty");
        return format!("{value}{final_character}ing");
    }

    format!("{value}ing")
}

fn past_tense_lowercase(value: &str) -> String {
    if let Some(irregular) = irregular_past_tense(value) {
        return irregular.to_string();
    }

    if value.ends_with('e') {
        return format!("{value}d");
    }

    if let Some(stem) = value.strip_suffix('y') {
        if stem
            .chars()
            .last()
            .is_some_and(|character| is_consonant(character))
        {
            return format!("{stem}ied");
        }
    }

    if should_double_final_consonant(value) {
        let final_character = value.chars().last().expect("value is not empty");
        return format!("{value}{final_character}ed");
    }

    format!("{value}ed")
}

fn irregular_past_tense(value: &str) -> Option<&'static str> {
    match value {
        "am" | "be" | "is" => Some("was"),
        "are" => Some("were"),
        "go" => Some("went"),
        "do" => Some("did"),
        "have" => Some("had"),
        "make" => Some("made"),
        "take" => Some("took"),
        "come" => Some("came"),
        "run" => Some("ran"),
        "eat" => Some("ate"),
        "see" => Some("saw"),
        "say" => Some("said"),
        "get" => Some("got"),
        "give" => Some("gave"),
        "find" => Some("found"),
        "think" => Some("thought"),
        "buy" => Some("bought"),
        "catch" => Some("caught"),
        "teach" => Some("taught"),
        "bring" => Some("brought"),
        "write" => Some("wrote"),
        "read" => Some("read"),
        _ => None,
    }
}

fn single_token_parts<'a>(
    value: &'a str,
    part_of_speech: &str,
) -> Result<(&'a str, &'a str, &'a str), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("input must contain one {part_of_speech}"));
    }
    if trimmed.split_whitespace().count() != 1 {
        return Err(format!(
            "input must contain exactly one {part_of_speech} token"
        ));
    }

    let leading_len = value.len() - value.trim_start().len();
    let trailing_len = value.len() - value.trim_end().len();
    let leading = &value[..leading_len];
    let trailing = &value[value.len() - trailing_len..];

    Ok((leading, trimmed, trailing))
}

fn should_double_final_consonant(value: &str) -> bool {
    let characters: Vec<char> = value.chars().collect();
    if characters.len() < 3 {
        return false;
    }

    let last = characters[characters.len() - 1];
    let middle = characters[characters.len() - 2];
    let first = characters[characters.len() - 3];

    is_consonant(first)
        && is_vowel(middle)
        && is_consonant(last)
        && !matches!(last, 'w' | 'x' | 'y')
}

fn is_vowel(character: char) -> bool {
    matches!(character, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn is_consonant(character: char) -> bool {
    character.is_ascii_alphabetic() && !is_vowel(character.to_ascii_lowercase())
}

fn apply_case_style(original: &str, value: &str) -> String {
    if original
        .chars()
        .all(|character| !character.is_alphabetic() || character.is_uppercase())
    {
        return value.to_uppercase();
    }

    let mut characters = original
        .chars()
        .filter(|character| character.is_alphabetic());
    if let Some(first) = characters.next() {
        if first.is_uppercase() && characters.all(|character| character.is_lowercase()) {
            return capitalize(value).expect("capitalize is infallible");
        }
    }

    value.to_string()
}

pub struct RenderState<'a> {
    ruleset: &'a RuleSet,
    context: HashMap<String, String>,
    call_stack: Vec<String>,
    rng: rand::rngs::ThreadRng,
}

impl<'a> RenderState<'a> {
    pub fn new(ruleset: &'a RuleSet) -> Self {
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
    processors: ProcessorRegistry,
}

impl RuleSet {
    pub fn from_config(config: hocon_rs::Value) -> Result<Self, RenderError> {
        Self::from_config_with_processors(config, ProcessorRegistry::new())
    }

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
                        context_defaults
                            .insert(context_name, value_to_node(context_value, &processors)?);
                    }
                } else {
                    rules.insert(name, value_to_node(value, &processors)?);
                }
            } else {
                rules.insert(name, value_to_node(value, &processors)?);
            }
        }

        Ok(RuleSet {
            rules,
            context_defaults,
            processors,
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

/// Applies named processors to a rendered child value from left to right.
pub struct ProcessorPipelineNode {
    node: Box<dyn Node>,
    processors: Vec<String>,
}

impl ProcessorPipelineNode {
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

fn value_to_node(
    value: hocon_rs::Value,
    processors: &ProcessorRegistry,
) -> Result<Box<dyn Node>, RenderError> {
    Ok(match value {
        hocon_rs::Value::String(template) => template_to_node(&template, processors)?,
        hocon_rs::Value::Array(values) => {
            let nodes = values
                .into_iter()
                .map(|value| value_to_node(value, processors))
                .collect::<Result<Vec<_>, _>>()?;
            Box::new(ChoiceNode::new(nodes))
        }
        hocon_rs::Value::Object(_) => Box::new(UnsupportedValueNode::new("object".to_string())),
        _ => Box::new(value.to_string()),
    })
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
    pub fn new(value_type: String) -> Self {
        UnsupportedValueNode { value_type }
    }
}

impl Node for UnsupportedValueNode {
    fn render(&self, _state: &mut RenderState) -> Result<String, RenderError> {
        Err(RenderError::UnsupportedValue(self.value_type.clone()))
    }
}
