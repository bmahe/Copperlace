use std::collections::{HashMap, HashSet};

use super::ruleset::RuleSet;

/// Initial variable bindings for one render operation.
///
/// Values in this map are available before top-level `context` defaults and
/// named rules. A render may still update them with overwrite bindings such as
/// `{alias:=rule}`.
pub type RenderContext = HashMap<String, String>;

/// Render-time options that affect rule expansion behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderOptions {
    /// Maximum recursive re-entries allowed for one rule name.
    ///
    /// A value of `0` preserves the default behavior: re-entering a rule that is
    /// already on the call stack returns `CircularRuleReference`. Values greater
    /// than zero allow that many recursive re-entries before recursive calls
    /// return an empty string.
    pub max_recursion_depth: usize,
}

/// Mutable state for one render operation.
///
/// `RuleSet::render_rule` creates a fresh state for each call. The state tracks
/// per-render bindings, the rule call stack used for cycle detection, and the
/// random number generator used by choice nodes.
pub struct RenderState<'a> {
    pub(crate) ruleset: &'a RuleSet,
    pub(crate) context: RenderContext,
    pub(crate) options: RenderOptions,
    pub(crate) call_stack: Vec<String>,
    pub(crate) unique_choices: HashMap<String, HashSet<usize>>,
    pub(crate) rng: rand::rngs::ThreadRng,
}

impl<'a> RenderState<'a> {
    /// Creates an empty render state for a ruleset.
    pub fn new(ruleset: &'a RuleSet) -> Self {
        Self::with_context(ruleset, RenderContext::new())
    }

    /// Creates a render state with initial variable bindings.
    pub fn with_context(ruleset: &'a RuleSet, context: RenderContext) -> Self {
        Self::with_context_and_options(ruleset, context, RenderOptions::default())
    }

    /// Creates a render state with initial variable bindings and render options.
    pub fn with_context_and_options(
        ruleset: &'a RuleSet,
        context: RenderContext,
        options: RenderOptions,
    ) -> Self {
        RenderState {
            ruleset,
            context,
            options,
            call_stack: Vec::new(),
            unique_choices: HashMap::new(),
            rng: rand::rngs::ThreadRng::default(),
        }
    }

    pub(crate) fn used_unique_choice_indices(&self, rule_name: &str) -> Option<&HashSet<usize>> {
        self.unique_choices.get(rule_name)
    }

    pub(crate) fn mark_unique_choice_index(&mut self, rule_name: &str, index: usize) {
        self.unique_choices
            .entry(rule_name.to_string())
            .or_default()
            .insert(index);
    }
}
