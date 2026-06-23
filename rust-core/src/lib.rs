//! Copperlace text renderer.
//!
//! Copperlace compiles HOCON config into named rules and renders one rule at a
//! time. Templates can reference other rules, make random choices, bind values
//! for the duration of one render, use lazy `context` defaults, and transform
//! rendered text with processors.
//!
//! For repeated renders from the same config, use [`Copperlace`] or [`RuleSet`]
//! so the config is parsed and compiled once. For one-off rendering, use
//! [`render_hocon_file`] or [`render_hocon_str`].

pub mod config;
pub mod ffi;
mod processors;
pub mod render;

pub use config::{
    ConfigError, Copperlace, render_hocon_file, render_hocon_str, ruleset_from_hocon_file,
    ruleset_from_hocon_str,
};
pub use render::{
    Processor, ProcessorRegistry, RenderError, RuleSet, processor, render_config_rule,
};
