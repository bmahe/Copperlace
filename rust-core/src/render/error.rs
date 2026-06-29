use std::fmt;

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
    /// A structured render was requested for a non-object path.
    UnsupportedStructuredTarget(String),
    /// A structured value could not be serialized to JSON.
    JsonSerialization(String),
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
            RenderError::UnsupportedStructuredTarget(rule) => {
                write!(
                    formatter,
                    "structured render target must be an object: {rule}"
                )
            }
            RenderError::JsonSerialization(message) => {
                write!(
                    formatter,
                    "failed to serialize structured value as JSON: {message}"
                )
            }
        }
    }
}

impl std::error::Error for RenderError {}
