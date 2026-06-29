use std::collections::HashMap;
use std::sync::Arc;

/// String transformer used in template processor pipelines.
///
/// Processors receive the rendered output of a rule or binding expression and
/// return the transformed value. Returning `Err` stops rendering and surfaces a
/// [`crate::render::RenderError::ProcessorError`].
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
/// Custom processors registered with [`crate::render::RuleSet::from_config_with_processors`]
/// extend the builtin registry. If a custom processor uses the same name as a
/// builtin, the custom implementation takes precedence.
pub type ProcessorRegistry = HashMap<String, Arc<dyn Processor>>;

/// Wraps a processor implementation for insertion into a [`ProcessorRegistry`].
pub fn processor<F>(processor: F) -> Arc<dyn Processor>
where
    F: Processor + 'static,
{
    Arc::new(processor)
}
