//! The polyhedral solver.
//!
//! This is the first phase of the solver, where we attempt to enumerate all points in a polyhedron subject to integer constraints.

use genawaiter::yield_;

#[derive(Debug, Clone)]
pub struct Effect {
    pub bugless: usize,
    pub bugged: usize,
}

/// A variable constraint is a requirement to be solved.
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Target value for the variable. This will always be honored.
    pub target: usize,

    /// Cap value for the variable. The solver will not attempt to find all solutions up to the limit, it will only reject solutions greater than the limit.
    pub limit: usize,
}

/// Given a list of variables, parts, and constraints, find candidate sets of parts.
///
/// We don't respect the limit here, because it is subject to arrangement.
pub fn solve<'a>(
    parts: &'a [&'a [Effect]],
    part_limit: usize,
    constraints: &'a [Constraint],
) -> impl Iterator<Item = Vec<usize>> + 'a {
    let parts_by_variable = {
        let mut parts_by_variable_map = std::collections::HashMap::new();
        for (i, part_effects) in parts.iter().enumerate() {
            for (variable_index, effect) in part_effects.iter().enumerate() {
                if effect.bugged == 0 && effect.bugless == 0 {
                    continue;
                }

                parts_by_variable_map
                    .entry(variable_index)
                    .or_insert_with(|| vec![])
                    .push(i);
            }
        }

        let mut parts_by_variable = (0..constraints.len())
            .map(|i| parts_by_variable_map.remove(&i).unwrap_or_else(|| vec![]))
            .collect::<Vec<_>>();

        for (i, part_indexes) in parts_by_variable.iter_mut().enumerate() {
            part_indexes.sort_unstable_by_key(|part_index| {
                let effect = &parts[*part_index][i];
                std::cmp::Reverse(std::cmp::min(effect.bugged, effect.bugless))
            });
        }

        parts_by_variable
    };

    /// An assignment is an assignment to a variable in the solver.
    #[derive(Debug, Clone)]
    struct Assignment {
        /// A guaranteed assignment is independent of placement: that is, the assignment applies bugged or bugless. This is the lower bound.
        guaranteed: usize,

        /// A worst case assignment is dependent of placement: it may or may not be applied. This is the upper bound.
        worst_case: usize,
    }

    fn inner<'a>(
        parts: &'a [&'a [Effect]],
        part_limit: usize,
        parts_by_variable: std::rc::Rc<Vec<Vec<usize>>>,
        assignments: Vec<(&'a Constraint, Assignment)>,
    ) -> impl Iterator<Item = Vec<usize>> + 'a {
        genawaiter::rc::gen!({
            let variable_index = if let Some(i) = assignments.iter().position(|(c, assignment)| {
                assignment.worst_case < c.target && assignment.guaranteed < c.limit
            }) {
                i
            } else {
                yield_!(vec![0; parts.len()]);
                return;
            };

            if part_limit == 0 {
                return;
            }

            'part_loop: for part_idx in parts_by_variable[variable_index].iter() {
                let part_effects = &parts[*part_idx];

                let mut assignments = assignments.clone();
                for ((c, assignment), effect) in assignments.iter_mut().zip(part_effects.iter()) {
                    assignment.guaranteed += std::cmp::min(effect.bugless, effect.bugged);
                    if assignment.guaranteed > c.limit {
                        continue 'part_loop;
                    }

                    assignment.worst_case += std::cmp::max(effect.bugless, effect.bugged);
                }

                let part_sets = inner(
                    parts,
                    part_limit - 1,
                    parts_by_variable.clone(),
                    assignments,
                )
                .collect::<Vec<_>>();

                for parts in part_sets.iter() {
                    let mut parts = parts.clone();
                    parts[*part_idx] += 1;
                    yield_!(parts);
                }
            }
        })
        .into_iter()
    }

    inner(
        parts,
        part_limit,
        std::rc::Rc::new(parts_by_variable),
        constraints
            .iter()
            .map(|c| {
                (
                    c,
                    Assignment {
                        guaranteed: 0,
                        worst_case: 0,
                    },
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve() {
        assert_eq!(
            solve(
                &[
                    // Super Armor
                    &[
                        Effect {
                            bugless: 1,
                            bugged: 0
                        },
                        Effect {
                            bugless: 0,
                            bugged: 0
                        }
                    ],
                    // HP +100
                    &[
                        Effect {
                            bugless: 0,
                            bugged: 0
                        },
                        Effect {
                            bugless: 100,
                            bugged: 100
                        }
                    ],
                ],
                4,
                &[
                    Constraint {
                        target: 1,
                        limit: 1
                    },
                    Constraint {
                        target: 300,
                        limit: 300
                    }
                ],
            )
            .collect::<Vec<_>>(),
            vec![vec![1, 3]]
        );
    }

    #[test]
    fn test_solve_inexact() {
        assert_eq!(
            solve(
                &[
                    // Super Armor
                    &[
                        Effect {
                            bugless: 1,
                            bugged: 0
                        },
                        Effect {
                            bugless: 0,
                            bugged: 0
                        }
                    ],
                    // HP +100
                    &[
                        Effect {
                            bugless: 0,
                            bugged: 0
                        },
                        Effect {
                            bugless: 100,
                            bugged: 100
                        }
                    ],
                ],
                10,
                &[
                    Constraint {
                        target: 1,
                        limit: 1
                    },
                    Constraint {
                        target: 350,
                        limit: 500
                    }
                ],
            )
            .collect::<Vec<_>>(),
            vec![vec![1, 4]]
        );
    }

    #[test]
    fn test_solve_limit() {
        assert_eq!(
            solve(
                &[
                    // HP +100
                    &[Effect {
                        bugless: 100,
                        bugged: 100
                    }],
                ],
                10,
                &[Constraint {
                    target: 50,
                    limit: 50
                }],
            )
            .collect::<Vec<_>>(),
            Vec::<Vec<_>>::new()
        );
    }

    #[test]
    fn test_solve_largest_first() {
        assert_eq!(
            solve(
                &[
                    // HP +10
                    &[Effect {
                        bugless: 10,
                        bugged: 10
                    }],
                    // HP +50
                    &[Effect {
                        bugless: 50,
                        bugged: 50
                    }],
                    // HP +100
                    &[Effect {
                        bugless: 100,
                        bugged: 100
                    }],
                ],
                2,
                &[Constraint {
                    target: 100,
                    limit: 100
                }],
            )
            .collect::<Vec<_>>(),
            vec![vec![0, 0, 1], vec![0, 2, 0]],
        );
    }

    #[test]
    fn test_solve_multiple_effects() {
        assert_eq!(
            solve(
                &[
                    // Body Pack
                    &[
                        Effect {
                            bugless: 1,
                            bugged: 0
                        },
                        Effect {
                            bugless: 1,
                            bugged: 0
                        },
                    ],
                    // Super Armor
                    &[
                        Effect {
                            bugless: 1,
                            bugged: 0
                        },
                        Effect {
                            bugless: 0,
                            bugged: 0
                        },
                    ],
                    // Air Shoes
                    &[
                        Effect {
                            bugless: 0,
                            bugged: 0
                        },
                        Effect {
                            bugless: 1,
                            bugged: 0
                        },
                    ],
                ],
                2,
                &[
                    Constraint {
                        target: 1,
                        limit: 1
                    },
                    Constraint {
                        target: 0,
                        limit: 1,
                    }
                ],
            )
            .collect::<Vec<_>>(),
            vec![vec![1, 0, 0], vec![0, 1, 0]],
        );
    }

    #[test]
    fn test_solve_multiple_effects_limit() {
        assert_eq!(
            solve(
                &[
                    // Body Pack
                    &[
                        Effect {
                            bugless: 1,
                            bugged: 1
                        },
                        Effect {
                            bugless: 1,
                            bugged: 1
                        },
                    ],
                    // Super Armor
                    &[
                        Effect {
                            bugless: 1,
                            bugged: 1
                        },
                        Effect {
                            bugless: 0,
                            bugged: 0
                        },
                    ],
                    // Air Shoes
                    &[
                        Effect {
                            bugless: 0,
                            bugged: 0
                        },
                        Effect {
                            bugless: 1,
                            bugged: 1
                        },
                    ],
                ],
                2,
                &[
                    Constraint {
                        target: 1,
                        limit: 1
                    },
                    Constraint {
                        target: 0,
                        limit: 0,
                    }
                ],
            )
            .collect::<Vec<_>>(),
            vec![vec![0, 1, 0]],
        );
    }
}
