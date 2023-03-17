use wasm_bindgen::prelude::*;

mod app;
pub mod console;
pub mod graphql;

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    init_wasm_hooks();
    yew::Renderer::<app::App>::new().render();
    Ok(())
}

fn init_wasm_hooks() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}
