use std::collections::HashMap;

use super::ruleset::RuleSet;

/// Initial variable bindings for one render operation.
///
/// Values in this map are available before top-level `context` defaults and
/// named rules. A render may still update them with overwrite bindings such as
/// `{alias:=rule}`.
pub type RenderContext = HashMap<String, String>;

/// Mutable state for one render operation.
///
/// `RuleSet::render_rule` creates a fresh state for each call. The state tracks
/// per-render bindings, the rule call stack used for cycle detection, and the
/// random number generator used by choice nodes.
pub struct RenderState<'a> {
    pub(crate) ruleset: &'a RuleSet,
    pub(crate) context: RenderContext,
    pub(crate) call_stack: Vec<String>,
    pub(crate) rng: rand::rngs::ThreadRng,
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
