mod placement;
mod polyhedral;

type Solution = Vec<placement::Placement>;

/// Solve.
pub fn solve<'a>(
    parts: &'a [placement::Part],
    size: (usize, usize),
    has_oob: bool,
    command_line_row: usize,
    constraints: &'a [polyhedral::Constraint],
    want_colorbug: Option<bool>,
) -> impl Iterator<Item = Solution> + 'a {
    let candidate_parts = parts
        .iter()
        .map(|p| p.effects.as_slice())
        .collect::<Vec<_>>();

    // Initialize a tableau.
    let tableau = placement::Tableau::new(size, has_oob, command_line_row);

    genawaiter::rc::gen!({
        for candidate in polyhedral::solve(&candidate_parts, size.0 * size.1, constraints) {
            //
        }
    })
    .into_iter()
}
