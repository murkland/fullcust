mod candidates;
mod placement;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum EffectBugRequirement {
    BugOnly,
    BuglessOnly,
    Always,
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub delta: usize,
    pub bug_requirement: EffectBugRequirement,
}

/// A part is a NaviCust part.
#[derive(Debug, Clone)]
pub struct Part {
    /// The NaviCust part must be placed on the command line for its unbugged effects to be active.
    pub must_be_on_command_line: bool,

    /// Effects.
    pub effects: Vec<Option<Effect>>,

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

/// A variable constraint is a requirement to be solved.
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Target value for the variable. This will always be honored.
    pub target: usize,

    /// Cap value for the variable. The solver will not attempt to find all solutions up to the cap, it will only reject solutions greater than the cap.
    pub cap: usize,
}

type Solution = Vec<placement::Placement>;

/// Solve.
pub fn solve<'a>(
    env: &'a Environment,
    constraints: &'a [Constraint],
    want_colorbug: Option<bool>,
) -> impl Iterator<Item = Solution> + 'a {
    let candidates = candidates::gather(&env.parts, constraints);

    // Initialize a memory map.
    let memory_map = placement::MemoryMap::new(env.size);

    genawaiter::rc::gen!({
        for candidate in candidates {
            //
        }
    })
    .into_iter()
}
