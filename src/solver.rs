mod candidates;
mod placement;

#[derive(Debug, Clone)]
pub struct Effect {
    pub bugless: usize,
    pub bugged: usize,
}

/// A part is a NaviCust part.
#[derive(Debug, Clone)]
pub struct Part {
    /// The NaviCust part must be placed on the command line for its unbugged effects to be active.
    pub must_be_on_command_line: bool,

    /// Effects.
    pub effects: Vec<Effect>,

    /// The shapes a part can be.
    pub shapes: Vec<placement::Shape>,
}

/// An environment encapsulates all the starting parameters for the solver.
#[derive(Debug, Clone)]
pub struct Environment {
    /// List of eligible parts.
    pub parts: Vec<Part>,

    /// Size of the NaviCust environment.
    pub size: (usize, usize),

    /// Whether or not the NaviCust's memory map has BN6-style out of bounds areas.
    pub has_oob: bool,

    /// Which row the command line is on.
    pub command_line_row: usize,
}

type Solution = Vec<placement::Placement>;

/// Solve.
pub fn solve<'a>(
    env: &'a Environment,
    constraints: &'a [candidates::Constraint],
    want_colorbug: Option<bool>,
) -> impl Iterator<Item = Solution> + 'a {
    let candidate_parts = env
        .parts
        .iter()
        .map(|p| p.effects.as_slice())
        .collect::<Vec<_>>();

    // Initialize a memory map.
    let memory_map = placement::MemoryMap::new(env.size);

    genawaiter::rc::gen!({
        for candidate in candidates::gather(&candidate_parts, env.size.0 * env.size.1, constraints)
        {
            //
        }
    })
    .into_iter()
}
