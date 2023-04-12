mod solver;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main_js() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    wasm_log::init(wasm_log::Config::default());
}

#[wasm_bindgen]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveArgs {
    parts: Vec<solver::Part>,
    requirements: Vec<solver::Requirement>,
    grid_settings: solver::GridSettings,
}

#[wasm_bindgen]
impl SolveArgs {
    #[wasm_bindgen(js_name = fromJs)]
    pub fn from_js(v: JsValue) -> Result<SolveArgs, serde_wasm_bindgen::Error> {
        serde_wasm_bindgen::from_value(v)
    }
}

#[wasm_bindgen]
pub struct Solution(solver::Solution);

#[wasm_bindgen]
impl Solution {
    #[wasm_bindgen(js_name = toJs)]
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
pub fn solve(args: SolveArgs) -> SolutionIterator {
    SolutionIterator(Box::new(solver::solve(
        args.parts,
        args.requirements,
        args.grid_settings,
    )))
}

#[wasm_bindgen]
impl PlaceAllArgs {
    #[wasm_bindgen(js_name = fromJs)]
    pub fn from_js(v: JsValue) -> Result<PlaceAllArgs, serde_wasm_bindgen::Error> {
        serde_wasm_bindgen::from_value(v)
    }
}

#[wasm_bindgen]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceAllArgs {
    parts: Vec<solver::Part>,
    requirements: Vec<solver::Requirement>,
    placements: Vec<solver::Placement>,
    grid_settings: solver::GridSettings,
}

#[wasm_bindgen(js_name = placeAll)]
pub fn place_all(args: PlaceAllArgs) -> JsValue {
    serde_wasm_bindgen::to_value(
        &solver::place_all(
            args.parts.iter().map(|v| v).collect::<Vec<_>>().as_slice(),
            args.requirements
                .iter()
                .map(|v| v)
                .collect::<Vec<_>>()
                .as_slice(),
            args.placements
                .iter()
                .map(|v| v)
                .collect::<Vec<_>>()
                .as_slice(),
            args.grid_settings,
        )
        .map(|v| {
            {
                v.into_iter().map(|v| match v {
                    Some(v) => v as isize,
                    None => -1,
                })
            }
            .collect::<Vec<_>>()
        }),
    )
    .unwrap()
}

pub fn main() -> Result<(), anyhow::Error> {
    log::info!("hello!");
    Ok(())
}
