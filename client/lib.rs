use graphql_client::reqwest_crate::Client;
use wasm_bindgen::prelude::*;

mod app;
pub mod console;
pub mod graphql;

lazy_static::lazy_static! {
    pub static ref CLIENT: Client = Client::new();
}

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    init_wasm_hooks();

    /*
    let vars = graphql::parts_query::Variables {};
    match post_graphql::<graphql::PartsQuery, _>(&CLIENT, "http://localhost:8000/query", vars).await {
        Ok(resp) => console::log!("{:?}", resp),
        Err(e) => console::log!("error: {}", e),
    }
    */

    yew::Renderer::<app::App>::new().render();
    Ok(())
}

fn init_wasm_hooks() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}
