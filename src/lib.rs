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

#[wasm_bindgen]
pub struct Part(solver::Part);

#[wasm_bindgen]
pub struct Requirement(solver::Requirement);

#[wasm_bindgen]
pub struct GridSettings(solver::GridSettings);

pub fn solve(parts: Box<[Part]>, requirements: Box<[Requirement]>, settings: GridSettings) {
    solver::solve(
        &parts.into_iter().map(|v| &v.0).collect::<Vec<_>>(),
        &requirements.into_iter().map(|v| &v.0).collect::<Vec<_>>(),
        &settings.0,
    );
}

pub fn main() -> Result<(), anyhow::Error> {
    log::info!("hello!");
    Ok(())
}
