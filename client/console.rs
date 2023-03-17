use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn js_log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    pub fn js_error(s: &str);
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => (crate::console::js_log(&format_args!($($t)*).to_string()))
}

#[macro_export]
macro_rules! error {
    ($($t:tt)*) => (crate::console::js_error(&format_args!($($t)*).to_string()))
}

pub use error;
pub use log;
