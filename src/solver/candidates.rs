use genawaiter::yield_;

#[derive(Debug, Clone)]
struct Assignment {
    /// A guaranteed assignment is independent of placement: that is, the assignment applies bugged or bugless. This is the lower bound.
    guaranteed: usize,

    /// A worst case assignment is dependent of placement: it may or may not be applied. This is the upper bound.
    worst_case: usize,
}

/// Given a list of variables, parts, and constraints, find candidate sets of parts.
///
/// We don't respect the cap here, because it is subject to arrangement.
pub fn gather<'a>(
    parts: &'a [super::Part],
    constraints: &'a [super::Constraint],
) -> impl Iterator<Item = Vec<usize>> + 'a {
    let parts_by_variable = {
        let mut parts_by_variable_map = std::collections::HashMap::new();
        for (i, part) in parts.iter().enumerate() {
            for (variable_index, effect) in part.effects.iter().enumerate() {
                if effect.is_none() {
                    continue;
                }

                parts_by_variable_map
                    .entry(variable_index)
                    .or_insert_with(|| vec![])
                    .push(i);
            }
        }

        (0..constraints.len())
            .map(|i| parts_by_variable_map.remove(&i).unwrap_or_else(|| vec![]))
            .collect::<Vec<_>>()
    };

    fn inner<'a>(
        parts: &'a [super::Part],
        parts_by_variable: std::rc::Rc<Vec<Vec<usize>>>,
        assignments: Vec<(&'a super::Constraint, Assignment)>,
    ) -> impl Iterator<Item = Vec<usize>> + 'a {
        genawaiter::rc::gen!({
            let variable_index = if let Some(i) = assignments.iter().position(|(c, assignment)| {
                c.target > assignment.worst_case && c.cap > assignment.guaranteed
            }) {
                i
            } else {
                yield_!(vec![0; parts.len()]);
                return;
            };

            for part_idx in parts_by_variable[variable_index].iter() {
                let part = &parts[*part_idx];

                let mut assignments = assignments.clone();
                for ((c, assignment), effect) in assignments.iter_mut().zip(part.effects.iter()) {
                    if let Some(effect) = effect {
                        assignment.worst_case += effect.delta;
                        if effect.bug_requirement == super::EffectBugRequirement::Always {
                            assignment.guaranteed += effect.delta;
                        }
                    }
                }

                let part_sets =
                    inner(parts, parts_by_variable.clone(), assignments).collect::<Vec<_>>();

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
    fn test_find_candidate_part_sets() {
        assert_eq!(
            gather(
                &[
                    // Super Armor
                    super::super::Part {
                        must_be_on_command_line: true,
                        effects: vec![
                            Some(super::super::Effect {
                                bug_requirement: super::super::EffectBugRequirement::BuglessOnly,
                                delta: 1
                            }),
                            None
                        ],
                        shapes: vec![super::super::placement::Shape {
                            color: 0,
                            mask: super::super::placement::Mask::new(
                                (7, 7),
                                vec![
                                    true, false, false, false, false, false, false, //
                                    true, true, false, false, false, false, false, //
                                    true, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                ],
                            )
                            .unwrap(),
                        }],
                    },
                    // HP +100
                    super::super::Part {
                        must_be_on_command_line: false,
                        effects: vec![
                            None,
                            Some(super::super::Effect {
                                bug_requirement: super::super::EffectBugRequirement::Always,
                                delta: 100
                            })
                        ],
                        shapes: vec![super::super::placement::Shape {
                            color: 1,
                            mask: super::super::placement::Mask::new(
                                (7, 7),
                                vec![
                                    true, true, false, false, false, false, false, //
                                    true, true, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                ],
                            )
                            .unwrap(),
                        }],
                    },
                ],
                &[
                    super::super::Constraint { target: 1, cap: 1 },
                    super::super::Constraint {
                        target: 300,
                        cap: 300
                    }
                ],
            )
            .collect::<Vec<_>>(),
            vec![vec![1, 3]]
        );
    }

    #[test]
    fn test_find_candidate_part_sets_inexact() {
        assert_eq!(
            gather(
                &[
                    // Super Armor
                    super::super::Part {
                        must_be_on_command_line: true,
                        effects: vec![
                            Some(super::super::Effect {
                                bug_requirement: super::super::EffectBugRequirement::BuglessOnly,
                                delta: 1
                            }),
                            None
                        ],
                        shapes: vec![super::super::placement::Shape {
                            color: 0,
                            mask: super::super::placement::Mask::new(
                                (7, 7),
                                vec![
                                    true, false, false, false, false, false, false, //
                                    true, true, false, false, false, false, false, //
                                    true, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                ],
                            )
                            .unwrap(),
                        }],
                    },
                    // HP +100
                    super::super::Part {
                        must_be_on_command_line: false,
                        effects: vec![
                            None,
                            Some(super::super::Effect {
                                bug_requirement: super::super::EffectBugRequirement::Always,
                                delta: 100
                            })
                        ],
                        shapes: vec![super::super::placement::Shape {
                            color: 1,
                            mask: super::super::placement::Mask::new(
                                (7, 7),
                                vec![
                                    true, true, false, false, false, false, false, //
                                    true, true, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                    false, false, false, false, false, false, false, //
                                ],
                            )
                            .unwrap(),
                        }],
                    },
                ],
                &[
                    super::super::Constraint { target: 1, cap: 1 },
                    super::super::Constraint {
                        target: 350,
                        cap: 350
                    }
                ],
            )
            .collect::<Vec<_>>(),
            vec![vec![1, 4]]
        );
    }
}