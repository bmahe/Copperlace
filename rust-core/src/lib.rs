//! Copperlace text renderer.
//!
//! Copperlace compiles configuration into named rules and renders one rule at a
//! time. Templates can reference other rules, make random choices, bind values
//! for the duration of one render, use lazy `context` defaults, and transform
//! rendered text with processors.
//! Rust callers can also provide initial render context values when rendering a
//! rule.
//!
//! For repeated renders from the same config, use [`Copperlace`] or [`RuleSet`]
//! so the config is parsed and compiled once. For one-off rendering, use
//! [`render_file`] or [`render_str`].

pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod ffi;
mod processors;
pub mod render;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use config::{
    ConfigError, Copperlace, render_file, render_file_structured,
    render_file_structured_with_context, render_file_with_context, render_str,
    render_str_structured, render_str_structured_with_context, render_str_with_context,
    ruleset_from_file, ruleset_from_str,
};
pub use render::{
    CopperlaceNumber, CopperlaceValue, Processor, ProcessorRegistry, RenderContext, RenderError,
    RuleSet, StructuredNode, TextGeneratorNode, processor, render_config_rule,
    render_config_rule_structured, render_config_rule_structured_with_context,
    render_config_rule_with_context,
};
