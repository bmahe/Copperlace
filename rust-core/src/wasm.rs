use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsError;
use wasm_bindgen::prelude::*;

/// JavaScript-facing load-once renderer for Copperlace HOCON config.
#[wasm_bindgen]
pub struct Copperlace {
    inner: crate::RuleSet,
}

struct JsProcessor {
    function: Function,
}

unsafe impl Send for JsProcessor {}
unsafe impl Sync for JsProcessor {}

impl crate::Processor for JsProcessor {
    fn process(&self, value: &str) -> Result<String, String> {
        let output = self
            .function
            .call1(&JsValue::NULL, &JsValue::from_str(value))
            .map_err(js_value_to_string)?;
        output
            .as_string()
            .ok_or_else(|| "processor returned a non-string value".to_string())
    }
}

#[wasm_bindgen]
impl Copperlace {
    /// Compiles a HOCON config string into a renderer that can be reused.
    #[wasm_bindgen(constructor)]
    pub fn new(config: &str) -> Result<Copperlace, JsError> {
        crate::ruleset_from_hocon_str(config)
            .map(|inner| Copperlace { inner })
            .map_err(to_js_error)
    }

    /// Compiles a HOCON config string with custom processor functions.
    #[wasm_bindgen(js_name = withProcessors)]
    pub fn with_processors(config: &str, processors: JsValue) -> Result<Copperlace, JsError> {
        ruleset_from_hocon_string_with_processors(config, processors)
            .map(|inner| Copperlace { inner })
    }

    /// Renders a named rule from the loaded config.
    pub fn render(&self, rule: &str) -> Result<String, JsError> {
        self.inner.render_rule(rule).map_err(to_js_error)
    }

    /// Renders a named rule with initial context values.
    #[wasm_bindgen(js_name = renderWithContext)]
    pub fn render_with_context(&self, rule: &str, context: JsValue) -> Result<String, JsError> {
        self.inner
            .render_rule_with_context(rule, read_context(context)?)
            .map_err(to_js_error)
    }
}

/// Renders one rule from a HOCON config string.
#[wasm_bindgen(js_name = renderHoconString)]
pub fn render_hocon_string(config: &str, rule: &str) -> Result<String, JsError> {
    crate::render_hocon_str(config, rule).map_err(to_js_error)
}

/// Renders one rule from a HOCON config string with custom processor functions.
#[wasm_bindgen(js_name = renderHoconStringWithProcessors)]
pub fn render_hocon_string_with_processors(
    config: &str,
    rule: &str,
    processors: JsValue,
) -> Result<String, JsError> {
    ruleset_from_hocon_string_with_processors(config, processors)?
        .render_rule(rule)
        .map_err(to_js_error)
}

/// Renders one rule from a HOCON config string with initial context values.
#[wasm_bindgen(js_name = renderHoconStringWithContext)]
pub fn render_hocon_string_with_context(
    config: &str,
    rule: &str,
    context: JsValue,
) -> Result<String, JsError> {
    crate::render_hocon_str_with_context(config, rule, read_context(context)?).map_err(to_js_error)
}

/// Renders one rule from a HOCON config string with custom processors and initial context.
#[wasm_bindgen(js_name = renderHoconStringWithProcessorsAndContext)]
pub fn render_hocon_string_with_processors_and_context(
    config: &str,
    rule: &str,
    processors: JsValue,
    context: JsValue,
) -> Result<String, JsError> {
    ruleset_from_hocon_string_with_processors(config, processors)?
        .render_rule_with_context(rule, read_context(context)?)
        .map_err(to_js_error)
}

fn read_context(context: JsValue) -> Result<crate::RenderContext, JsError> {
    if context.is_null() || context.is_undefined() || !context.is_object() {
        return Err(JsError::new("context must be an object"));
    }

    let object = Object::from(context);
    let keys = Object::keys(&object);
    let mut render_context = crate::RenderContext::new();
    for index in 0..keys.length() {
        let key = keys.get(index);
        let key_string = key
            .as_string()
            .ok_or_else(|| JsError::new("context keys must be strings"))?;
        let value = Reflect::get(&object, &key)
            .map_err(|_| JsError::new("failed to read context value"))?;
        let value_string = value
            .as_string()
            .ok_or_else(|| JsError::new("context values must be strings"))?;
        render_context.insert(key_string, value_string);
    }

    Ok(render_context)
}

fn read_processors(processors: JsValue) -> Result<crate::ProcessorRegistry, JsError> {
    if processors.is_null() || processors.is_undefined() || !processors.is_object() {
        return Err(JsError::new("processors must be an object"));
    }

    let object = Object::from(processors);
    let keys = Object::keys(&object);
    let mut registry = crate::ProcessorRegistry::new();
    for index in 0..keys.length() {
        let key = keys.get(index);
        let key_string = key
            .as_string()
            .ok_or_else(|| JsError::new("processor names must be strings"))?;
        let value = Reflect::get(&object, &key)
            .map_err(|_| JsError::new("failed to read processor value"))?;
        let function = value
            .dyn_into::<Function>()
            .map_err(|_| JsError::new("processors must be functions"))?;
        registry.insert(key_string, std::sync::Arc::new(JsProcessor { function }));
    }

    Ok(registry)
}

fn ruleset_from_hocon_string_with_processors(
    config: &str,
    processors: JsValue,
) -> Result<crate::RuleSet, JsError> {
    let value = hocon_rs::Config::parse_str::<hocon_rs::Value>(config, None)
        .map_err(|error| JsError::new(&format!("failed to parse config: {error:?}")))?;
    crate::RuleSet::from_config_with_processors(value, read_processors(processors)?)
        .map_err(to_js_error)
}

fn to_js_error(error: impl ToString) -> JsError {
    JsError::new(&error.to_string())
}

fn js_value_to_string(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "processor callback failed".to_string())
}
