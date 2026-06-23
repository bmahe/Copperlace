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
