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
}

/// Renders one rule from a HOCON config string.
#[wasm_bindgen(js_name = renderHoconString)]
pub fn render_hocon_string(config: &str, rule: &str) -> Result<String, JsError> {
    crate::render_hocon_str(config, rule).map_err(to_js_error)
}

fn to_js_error(error: impl ToString) -> JsError {
    JsError::new(&error.to_string())
}
