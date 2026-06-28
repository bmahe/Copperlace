use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::render::{RenderContext, RenderError, RuleSet};

/// Error returned while loading, parsing, compiling, or rendering configuration.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// The configuration document could not be loaded or parsed.
    Parse(String),
    /// The config parsed successfully, but compilation or rendering failed.
    Render(RenderError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Parse(error) => write!(formatter, "failed to parse config: {error}"),
            ConfigError::Render(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<RenderError> for ConfigError {
    fn from(error: RenderError) -> Self {
        ConfigError::Render(error)
    }
}

/// Load-once renderer for repeated renders from one configuration.
///
/// `Copperlace` wraps a compiled [`RuleSet`]. Use it when rendering more than
/// one rule, or rendering the same rule multiple times, so the config is not
/// parsed and compiled for every render.
pub struct Copperlace {
    ruleset: RuleSet,
}

impl Copperlace {
    /// Compiles a configuration string into a reusable renderer.
    ///
    /// Returns [`ConfigError::Parse`] when the string is not valid configuration, and
    /// [`ConfigError::Render`] when the parsed config is not a valid Copperlace
    /// rule set.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(config: &str) -> Result<Self, ConfigError> {
        Ok(Self {
            ruleset: ruleset_from_str(config)?,
        })
    }

    /// Loads and compiles a configuration file into a reusable renderer.
    ///
    /// Returns [`ConfigError::Parse`] when the file cannot be loaded as configuration,
    /// and [`ConfigError::Render`] when the parsed config is not a valid
    /// Copperlace rule set.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        Ok(Self {
            ruleset: ruleset_from_file(path)?,
        })
    }

    /// Renders a named rule from the compiled config.
    ///
    /// Each call starts with a fresh render context. Bindings are consistent
    /// within one output but do not carry over to later renders.
    pub fn render(&self, rule_name: &str) -> Result<String, RenderError> {
        self.ruleset.render_rule(rule_name)
    }

    /// Renders a named rule from the compiled config with initial context.
    ///
    /// Initial context values are scoped to this render call. They resolve
    /// before config-defined `context` defaults and named rules.
    pub fn render_with_context(
        &self,
        rule_name: &str,
        context: RenderContext,
    ) -> Result<String, RenderError> {
        self.ruleset.render_rule_with_context(rule_name, context)
    }
}

impl FromStr for Copperlace {
    type Err = ConfigError;

    fn from_str(config: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            ruleset: ruleset_from_str(config)?,
        })
    }
}

/// Parses a configuration string and compiles it into a reusable [`RuleSet`].
pub fn ruleset_from_str(config: &str) -> Result<RuleSet, ConfigError> {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None)
        .map_err(|error| ConfigError::Parse(format!("{error:?}")))?;
    RuleSet::from_config(value).map_err(ConfigError::Render)
}

/// Loads a configuration file and compiles it into a reusable [`RuleSet`].
pub fn ruleset_from_file(path: impl AsRef<Path>) -> Result<RuleSet, ConfigError> {
    let path = path.as_ref().to_string_lossy();
    let value = hocon_rs::Config::load(path.as_ref(), None)
        .map_err(|error| ConfigError::Parse(format!("{error:?}")))?;
    RuleSet::from_config(value).map_err(ConfigError::Render)
}

/// Renders one rule from a configuration string.
///
/// This convenience helper parses and compiles the config, renders one rule,
/// and drops the compiled ruleset. Use [`Copperlace::from_str`] or
/// [`ruleset_from_str`] for repeated renders.
pub fn render_str(config: &str, rule_name: &str) -> Result<String, ConfigError> {
    render_str_with_context(config, rule_name, RenderContext::new())
}

/// Renders one rule from a configuration string with initial context.
pub fn render_str_with_context(
    config: &str,
    rule_name: &str,
    context: RenderContext,
) -> Result<String, ConfigError> {
    ruleset_from_str(config)?
        .render_rule_with_context(rule_name, context)
        .map_err(ConfigError::Render)
}

/// Renders one rule from a configuration file.
///
/// This convenience helper loads and compiles the file, renders one rule, and
/// drops the compiled ruleset. Use [`Copperlace::from_file`] or
/// [`ruleset_from_file`] for repeated renders.
pub fn render_file(path: impl AsRef<Path>, rule_name: &str) -> Result<String, ConfigError> {
    render_file_with_context(path, rule_name, RenderContext::new())
}

/// Renders one rule from a configuration file with initial context.
pub fn render_file_with_context(
    path: impl AsRef<Path>,
    rule_name: &str,
    context: RenderContext,
) -> Result<String, ConfigError> {
    ruleset_from_file(path)?
        .render_rule_with_context(rule_name, context)
        .map_err(ConfigError::Render)
}
