mod solver;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Parts(Vec<solver::Part>);

#[wasm_bindgen]
pub struct Requirements(Vec<solver::Requirement>);

#[wasm_bindgen]
pub struct GridSettings(solver::GridSettings);

#[wasm_bindgen]
pub struct Solution(solver::Solution);

#[wasm_bindgen]
impl Solution {
    pub fn to_js(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.0).unwrap()
    }
}

#[wasm_bindgen]
pub struct SolutionIterator(Box<dyn Iterator<Item = solver::Solution>>);

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
