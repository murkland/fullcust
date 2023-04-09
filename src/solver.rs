mod placement;
mod polyhedral;

/// An environment encapsulates all the starting parameters for the solver.
#[derive(Debug, Clone)]
pub struct Environment {
    /// List of eligible parts.
    pub parts: Vec<placement::Part>,

    /// Size of the NaviCust environment.
    pub size: (usize, usize),

    /// Whether or not the NaviCust's tableau has BN6-style out of bounds areas.
    pub has_oob: bool,

    /// Which row the command line is on.
    pub command_line_row: usize,
}

type Solution = Vec<placement::Placement>;

/// Solve.
pub fn solve<'a>(
    env: &'a Environment,
    constraints: &'a [polyhedral::Constraint],
    want_colorbug: Option<bool>,
) -> impl Iterator<Item = Solution> + 'a {
    let candidate_parts = env
        .parts
        .iter()
        .map(|p| p.effects.as_slice())
        .collect::<Vec<_>>();

    // Initialize a tableau.
    let tableau = placement::Tableau::new(env.size);

    genawaiter::rc::gen!({
        for candidate in polyhedral::solve(&candidate_parts, env.size.0 * env.size.1, constraints) {
            //
        }
    })
    .into_iter()
}
