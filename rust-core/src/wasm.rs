use js_sys::{Object, Reflect};
use wasm_bindgen::JsError;
use wasm_bindgen::prelude::*;

/// JavaScript-facing load-once renderer for Copperlace HOCON config.
#[wasm_bindgen]
pub struct Copperlace {
    inner: crate::Copperlace,
}

#[wasm_bindgen]
impl Copperlace {
    /// Compiles a HOCON config string into a renderer that can be reused.
    #[wasm_bindgen(constructor)]
    pub fn new(config: &str) -> Result<Copperlace, JsError> {
        crate::Copperlace::from_hocon_str(config)
            .map(|inner| Copperlace { inner })
            .map_err(to_js_error)
    }

    /// Renders a named rule from the loaded config.
    pub fn render(&self, rule: &str) -> Result<String, JsError> {
        self.inner.render(rule).map_err(to_js_error)
    }

    /// Renders a named rule with initial context values.
    #[wasm_bindgen(js_name = renderWithContext)]
    pub fn render_with_context(&self, rule: &str, context: JsValue) -> Result<String, JsError> {
        self.inner
            .render_with_context(rule, read_context(context)?)
            .map_err(to_js_error)
    }
}

/// Renders one rule from a HOCON config string.
#[wasm_bindgen(js_name = renderHoconString)]
pub fn render_hocon_string(config: &str, rule: &str) -> Result<String, JsError> {
    crate::render_hocon_str(config, rule).map_err(to_js_error)
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

fn to_js_error(error: impl ToString) -> JsError {
    JsError::new(&error.to_string())
}
