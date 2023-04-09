mod solver;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    wasm_log::init(wasm_log::Config::default());

    main().map_err(|e| JsError::new(&format!("{:?}", e)))?;
    Ok(())
}

pub fn main() -> Result<(), anyhow::Error> {
    log::info!("hello!");
    Ok(())
}
