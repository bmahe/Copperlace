use std::fmt;
use std::path::Path;

use crate::render::{RenderError, RuleSet};

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Parse(String),
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

pub fn ruleset_from_hocon_str(config: &str) -> Result<RuleSet, ConfigError> {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None)
        .map_err(|error| ConfigError::Parse(format!("{error:?}")))?;
    RuleSet::from_config(value).map_err(ConfigError::Render)
}

pub fn ruleset_from_hocon_file(path: impl AsRef<Path>) -> Result<RuleSet, ConfigError> {
    let path = path.as_ref().to_string_lossy();
    let value = hocon_rs::Config::load(path.as_ref(), None)
        .map_err(|error| ConfigError::Parse(format!("{error:?}")))?;
    RuleSet::from_config(value).map_err(ConfigError::Render)
}

pub fn render_hocon_str(config: &str, rule_name: &str) -> Result<String, ConfigError> {
    ruleset_from_hocon_str(config)?
        .render_rule(rule_name)
        .map_err(ConfigError::Render)
}

pub fn render_hocon_file(path: impl AsRef<Path>, rule_name: &str) -> Result<String, ConfigError> {
    ruleset_from_hocon_file(path)?
        .render_rule(rule_name)
        .map_err(ConfigError::Render)
}
