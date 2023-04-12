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
pub struct Parts(Vec<solver::Part>);

#[wasm_bindgen]
pub struct Requirements(Vec<solver::Requirement>);

#[wasm_bindgen]
pub struct GridSettings(solver::GridSettings);

#[wasm_bindgen]
pub struct SolutionIterator(Box<dyn Iterator<Item = solver::Solution>>);

#[wasm_bindgen]
pub struct Solution(solver::Solution);

#[wasm_bindgen]
impl SolutionIterator {
    pub fn next(&mut self) -> Option<Solution> {
        self.0.next().map(|v| Solution(v))
    }
}

#[wasm_bindgen]
pub fn solve(parts: Parts, requirements: Requirements, settings: GridSettings) -> SolutionIterator {
    SolutionIterator(Box::new(solver::solve(parts.0, requirements.0, settings.0)))
}

pub fn main() -> Result<(), anyhow::Error> {
    log::info!("hello!");
    Ok(())
}
