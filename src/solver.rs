use genawaiter::yield_;

#[derive(Debug, Clone)]
pub struct Variable {
    /// Incrementing beyond this maximum will saturate at the maximum.
    pub max: usize,
}

/// A mask represents the shape of a NaviCust part.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    repr: ndarray::Array2<bool>,
}

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

impl Mask {
    pub fn new(shape: (usize, usize), mask: Vec<bool>) -> Result<Self, ndarray::ShapeError> {
        Ok(Mask {
            repr: ndarray::Array2::from_shape_vec(shape, mask)?,
        })
    }

    fn rot90(self) -> Self {
        let mut mask = self.repr.reversed_axes().as_standard_layout().into_owned();
        for row in mask.rows_mut() {
            row.into_slice().unwrap().reverse();
        }
        Mask { repr: mask }
    }
}

/// A part is a NaviCust part.
#[derive(Debug, Clone)]
pub struct Part {
    /// The NaviCust part must be placed on the command line for its unbugged effects to be active.
    pub must_be_on_command_line: bool,

    /// Effects.
    pub effects: Vec<Option<Effect>>,

    /// The shapes a part can be.
    pub shapes: Vec<Shape>,
}

/// A shape is a concrete shape a NaviCust part takes.
#[derive(Debug, Clone)]
pub struct Shape {
    /// A NaviCust part's color.
    pub color: usize,

    /// The mask of the part.
    pub mask: Mask,
}

/// An environment encapsulates all the starting parameters for the solver.
#[derive(Debug, Clone)]
pub struct Environment {
    /// List of variables.
    pub variables: Vec<Variable>,

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

/// A placement determines where to place a NaviCust part.
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    /// Which part to place.
    pub part_index: usize,

    /// Where to place the part.
    pub position: (isize, isize),

    /// How many 90 degree rotations are required.
    pub rotation: usize,
}

type Solution = Vec<Placement>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("mismatching constraints: expected {num_variables} got {num_constraints}")]
    MismatchingConstraints {
        num_constraints: usize,
        num_variables: usize,
    },
}

#[derive(thiserror::Error, Debug)]
enum PlaceError {
    #[error("mismatching shapes: expected {memory_map_shape:?} got {mask_shape:?}")]
    ShapesMismatched {
        memory_map_shape: (usize, usize),
        mask_shape: (usize, usize),
    },

    #[error("destination clobbered")]
    DestinationClobbered,

    #[error("source clipped")]
    SourceClipped,
}

#[derive(Clone, Debug)]
struct MemoryMap {
    repr: ndarray::Array2<Option<usize>>,
}

impl MemoryMap {
    fn new(size: (usize, usize)) -> Self {
        Self {
            repr: ndarray::Array2::from_elem(size, None),
        }
    }

    fn place(mut self, mask: &Mask, placement: Placement) -> Result<Self, PlaceError> {
        if mask.repr.shape() != self.repr.shape() {
            return Err(PlaceError::ShapesMismatched {
                memory_map_shape: self.repr.dim(),
                mask_shape: mask.repr.dim(),
            });
        }

        let (w, h) = self.repr.dim();

        let mut mask = std::borrow::Cow::Borrowed(mask);
        for _ in 0..placement.rotation {
            mask = std::borrow::Cow::Owned(mask.into_owned().rot90());
        }

        let (src_x, dst_x) = if placement.position.0 < 0 {
            (-placement.position.0 as usize, 0)
        } else {
            (0, placement.position.0 as usize)
        };

        let (src_y, dst_y) = if placement.position.1 < 0 {
            (-placement.position.1 as usize, 0)
        } else {
            (0, placement.position.1 as usize)
        };

        // Validate that our mask isn't being weirdly clipped.
        for (y, row) in mask.repr.rows().into_iter().enumerate() {
            for (x, &v) in row.into_iter().enumerate() {
                if x >= src_x && y >= src_y && x < w - dst_x && y < h - dst_y {
                    continue;
                }

                if v {
                    return Err(PlaceError::SourceClipped);
                }
            }
        }

        for (src_row, dst_row) in std::iter::zip(
            mask.repr.slice(ndarray::s![src_y.., src_x..]).rows(),
            self.repr
                .slice_mut(ndarray::s![dst_y.., dst_x..])
                .rows_mut(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src {
                    if dst.is_some() {
                        return Err(PlaceError::DestinationClobbered);
                    }
                    *dst = Some(placement.part_index);
                }
            }
        }

        Ok(self)
    }
}

/// Given a list of variables, parts, and constraints, find candidate sets of parts.
///
/// We don't respect the cap here, because it is subject to arrangement.
fn find_candidate_part_sets<'a>(
    parts: &'a [Part],
    variables: &'a [Variable],
    constraints: &'a [Constraint],
) -> Result<impl Iterator<Item = Vec<usize>> + 'a, Error> {
    if variables.len() != constraints.len() {
        return Err(Error::MismatchingConstraints {
            num_constraints: constraints.len(),
            num_variables: variables.len(),
        });
    }

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

        (0..variables.len())
            .map(|i| parts_by_variable_map.remove(&i).unwrap_or_else(|| vec![]))
            .collect::<Vec<_>>()
    };

    fn inner<'a>(
        parts: &'a [Part],
        parts_by_variable: std::rc::Rc<Vec<Vec<usize>>>,
        vc_pairs: Vec<(&'a Variable, Constraint)>,
    ) -> impl Iterator<Item = Vec<usize>> + 'a {
        genawaiter::rc::gen!({
            let variable_index = if let Some(i) = vc_pairs.iter().position(|(_, c)| c.target > 0) {
                i
            } else {
                yield_!(vec![0; parts.len()]);
                return;
            };

            for part_idx in parts_by_variable[variable_index].iter() {
                let part = &parts[*part_idx];

                let mut vc_pairs = vc_pairs.clone();
                for ((variable, constraint), effect) in vc_pairs.iter_mut().zip(part.effects.iter())
                {
                    if let Some(effect) = effect {
                        if effect.delta > constraint.target {
                            constraint.target = 0;
                        } else {
                            constraint.target -= effect.delta;
                        }
                    }
                }

                let part_sets =
                    inner(parts, parts_by_variable.clone(), vc_pairs).collect::<Vec<_>>();

                for parts in part_sets.iter() {
                    let mut parts = parts.clone();
                    parts[*part_idx] += 1;
                    yield_!(parts);
                }
            }
        })
        .into_iter()
    }

    Ok(inner(
        parts,
        std::rc::Rc::new(parts_by_variable),
        std::iter::zip(variables, constraints.iter().map(|c| c.clone())).collect(),
    ))
}

/// Solve.
pub fn solve<'a>(
    env: &'a Environment,
    constraints: &'a [Constraint],
    want_colorbug: Option<bool>,
) -> Result<impl Iterator<Item = Solution> + 'a, Error> {
    let candidates = find_candidate_part_sets(&env.parts, &env.variables, constraints)?;

    // Initialize a memory map.
    let memory_map = MemoryMap::new(env.size);

    Ok(genawaiter::rc::gen!({
        for candidate in candidates {
            //
        }
    })
    .into_iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_rot90() {
        let mask = Mask::new(
            (7, 7),
            vec![
                true, true, true, true, true, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
            ],
        )
        .unwrap();
        let mask = mask.rot90();
        assert_eq!(
            mask,
            Mask::new(
                (7, 7),
                vec![
                    true, true, true, true, true, true, true, //
                    true, true, true, true, true, true, true, //
                    true, true, true, true, true, true, true, //
                    true, true, true, true, true, true, true, //
                    false, false, false, false, false, false, true, //
                    false, false, false, false, false, false, false, //
                    false, false, false, false, false, false, false, //
                ],
            )
            .unwrap()
        )
    }

    #[test]
    fn test_memory_map_place() {
        let memory_map = MemoryMap::new((7, 7));
        let super_armor = Mask::new(
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
        .unwrap();

        #[rustfmt::skip]
        let expected_repr = ndarray::Array2::from_shape_vec((7, 7), vec![
            Some(0), None, None, None, None, None, None,
            Some(0), Some(0), None, None, None, None, None,
            Some(0), None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
        ]).unwrap();

        assert_eq!(
            memory_map
                .place(
                    &super_armor,
                    Placement {
                        part_index: 0,
                        position: (0, 0),
                        rotation: 0,
                    },
                )
                .unwrap()
                .repr,
            expected_repr
        );
    }

    #[test]
    fn test_memory_map_place_rot() {
        let memory_map = MemoryMap::new((7, 7));
        let super_armor = Mask::new(
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
        .unwrap();

        #[rustfmt::skip]
        let expected_repr = ndarray::Array2::from_shape_vec((7, 7), vec![
            None, None, None, None, Some(0), Some(0), Some(0),
            None, None, None, None, None, Some(0), None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
        ]).unwrap();

        assert_eq!(
            memory_map
                .place(
                    &super_armor,
                    Placement {
                        part_index: 0,
                        position: (0, 0),
                        rotation: 1,
                    },
                )
                .unwrap()
                .repr,
            expected_repr
        );
    }

    #[test]
    fn test_memory_map_place_nonzero_pos() {
        let memory_map = MemoryMap::new((7, 7));
        let super_armor = Mask::new(
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
        .unwrap();

        #[rustfmt::skip]
        let expected_repr = ndarray::Array2::from_shape_vec((7, 7), vec![
            None, Some(0), None, None, None, None, None,
            None, Some(0), Some(0), None, None, None, None,
            None, Some(0), None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
        ]).unwrap();

        assert_eq!(
            memory_map
                .place(
                    &super_armor,
                    Placement {
                        part_index: 0,
                        position: (1, 0),
                        rotation: 0,
                    },
                )
                .unwrap()
                .repr,
            expected_repr
        );
    }

    #[test]
    fn test_memory_map_place_neg_pos() {
        let memory_map = MemoryMap::new((7, 7));
        let super_armor = Mask::new(
            (7, 7),
            vec![
                false, true, false, false, false, false, false, //
                false, true, true, false, false, false, false, //
                false, true, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        )
        .unwrap();

        #[rustfmt::skip]
        let expected_repr = ndarray::Array2::from_shape_vec((7, 7), vec![
            Some(0), None, None, None, None, None, None,
            Some(0), Some(0), None, None, None, None, None,
            Some(0), None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
            None, None, None, None, None, None, None,
        ]).unwrap();

        assert_eq!(
            memory_map
                .place(
                    &super_armor,
                    Placement {
                        part_index: 0,
                        position: (-1, 0),
                        rotation: 0,
                    },
                )
                .unwrap()
                .repr,
            expected_repr
        );
    }

    #[test]
    fn test_memory_map_place_source_clipped() {
        let memory_map = MemoryMap::new((7, 7));
        let super_armor = Mask::new(
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
        .unwrap();

        assert_matches::assert_matches!(
            memory_map.place(
                &super_armor,
                Placement {
                    part_index: 0,
                    position: (-1, -1),
                    rotation: 0,
                },
            ),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_memory_map_place_source_clipped_other_side() {
        let memory_map = MemoryMap::new((7, 7));

        let super_armor = Mask::new(
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
        .unwrap();

        assert_matches::assert_matches!(
            memory_map.place(
                &super_armor,
                Placement {
                    part_index: 0,
                    position: (6, 0),
                    rotation: 0,
                },
            ),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_memory_map_destination_clobbered() {
        let mut memory_map = MemoryMap::new((7, 7));
        memory_map.repr[[0, 0]] = Some(2);

        let super_armor = Mask::new(
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
        .unwrap();

        assert_matches::assert_matches!(
            memory_map.place(
                &super_armor,
                Placement {
                    part_index: 0,
                    position: (0, 0),
                    rotation: 0,
                },
            ),
            Err(PlaceError::DestinationClobbered)
        );
    }

    #[test]
    fn test_find_candidate_part_sets() {
        assert_eq!(
            find_candidate_part_sets(
                &[
                    // Super Armor
                    Part {
                        must_be_on_command_line: true,
                        effects: vec![
                            Some(Effect {
                                bug_requirement: EffectBugRequirement::BuglessOnly,
                                delta: 1
                            }),
                            None
                        ],
                        shapes: vec![Shape {
                            color: 0,
                            mask: Mask::new(
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
                    Part {
                        must_be_on_command_line: false,
                        effects: vec![
                            None,
                            Some(Effect {
                                bug_requirement: EffectBugRequirement::Always,
                                delta: 100
                            })
                        ],
                        shapes: vec![Shape {
                            color: 1,
                            mask: Mask::new(
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
                    // Super Armor
                    Variable { max: 1 },
                    // HP
                    Variable { max: 100 },
                ],
                &[
                    Constraint { target: 1, cap: 1 },
                    Constraint {
                        target: 300,
                        cap: 300
                    }
                ],
            )
            .unwrap()
            .collect::<Vec<_>>(),
            vec![vec![1, 3]]
        );
    }
}
