mod compile;
mod error;
mod nodes;
mod processor;
mod ruleset;
mod state;
mod template;
mod value;

pub use error::RenderError;
pub use nodes::{
    BindMode, BindNode, ChoiceNode, ProcessorPipelineNode, RuleCallNode, TextGeneratorNode,
    UnsupportedValueNode, VariableNode, VecNode, WeightedChoiceNode,
};
pub use processor::{Processor, ProcessorRegistry, processor};
pub use ruleset::{
    RuleSet, render_config_rule, render_config_rule_structured,
    render_config_rule_structured_with_context, render_config_rule_with_context,
};
pub use state::{RenderContext, RenderState};
pub use value::{CopperlaceNumber, CopperlaceValue, StructuredNode};
