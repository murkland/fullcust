/// A mask represents the shape of a NaviCust part.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    repr: ndarray::Array2<bool>,
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

/// A shape is a concrete shape a NaviCust part takes.
#[derive(Debug, Clone)]
pub struct Shape {
    /// A NaviCust part's color.
    pub color: usize,

    /// The mask of the part.
    pub mask: Mask,
}

/// A location determines where to place a NaviCust part.
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// Where to place the part.
    pub position: (isize, isize),

    /// How many 90 degree rotations are required.
    pub rotation: usize,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub loc: Location,
    pub part_index: usize,
    pub part_shape_index: usize,
    pub color: usize,
}

#[derive(thiserror::Error, Debug)]
enum PlaceError {
    #[error("mismatching shapes: expected {tableau_shape:?} got {mask_shape:?}")]
    ShapesMismatched {
        tableau_shape: (usize, usize),
        mask_shape: (usize, usize),
    },

    #[error("destination clobbered")]
    DestinationClobbered,

    #[error("source clipped")]
    SourceClipped,
}

#[derive(Clone, Debug)]
pub struct Tableau {
    placements: Vec<Placement>,
    arr: ndarray::Array2<Option<usize>>,
}

impl Tableau {
    pub fn new(size: (usize, usize)) -> Self {
        Self {
            placements: vec![],
            arr: ndarray::Array2::from_elem(size, None),
        }
    }

    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    pub fn place(mut self, mask: &Mask, placement: Placement) -> Result<Self, PlaceError> {
        if mask.repr.shape() != self.arr.shape() {
            return Err(PlaceError::ShapesMismatched {
                tableau_shape: self.arr.dim(),
                mask_shape: mask.repr.dim(),
            });
        }

        let placement_index = self.placements.len();

        let (w, h) = self.arr.dim();

        let mut mask = std::borrow::Cow::Borrowed(mask);
        for _ in 0..placement.loc.rotation {
            mask = std::borrow::Cow::Owned(mask.into_owned().rot90());
        }

        let (src_x, dst_x) = if placement.loc.position.0 < 0 {
            (-placement.loc.position.0 as usize, 0)
        } else {
            (0, placement.loc.position.0 as usize)
        };

        let (src_y, dst_y) = if placement.loc.position.1 < 0 {
            (-placement.loc.position.1 as usize, 0)
        } else {
            (0, placement.loc.position.1 as usize)
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
            self.arr.slice_mut(ndarray::s![dst_y.., dst_x..]).rows_mut(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src {
                    if dst.is_some() {
                        return Err(PlaceError::DestinationClobbered);
                    }
                    *dst = Some(placement_index);
                }
            }
        }

        self.placements.push(placement);

        Ok(self)
    }
}

/// A part is a NaviCust part.
#[derive(Debug, Clone)]
pub struct Part {
    /// The NaviCust part must be placed on the command line for its unbugged effects to be active.
    pub must_be_on_command_line: bool,

    /// Effects.
    pub effects: Vec<super::polyhedral::Effect>,

    /// The shapes a part can be.
    pub shapes: Vec<Shape>,
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
    fn test_tableau_place() {
        let tableau = Tableau::new((7, 7));
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
            tableau
                .place(
                    &super_armor,
                    Placement {
                        loc: Location {
                            position: (0, 0),
                            rotation: 0,
                        },
                        part_index: 0,
                        part_shape_index: 0,
                        color: 0
                    },
                )
                .unwrap()
                .arr,
            expected_repr
        );
    }

    #[test]
    fn test_tableau_place_rot() {
        let tableau = Tableau::new((7, 7));
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
            tableau
                .place(
                    &super_armor,
                    Placement {
                        loc: Location {
                            position: (0, 0),
                            rotation: 1,
                        },
                        part_index: 0,
                        part_shape_index: 0,
                        color: 0
                    },
                )
                .unwrap()
                .arr,
            expected_repr
        );
    }

    #[test]
    fn test_tableau_place_nonzero_pos() {
        let tableau = Tableau::new((7, 7));
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
            tableau
                .place(
                    &super_armor,
                    Placement {
                        loc: Location {
                            position: (1, 0),
                            rotation: 0,
                        },
                        part_index: 0,
                        part_shape_index: 0,
                        color: 0
                    },
                )
                .unwrap()
                .arr,
            expected_repr
        );
    }

    #[test]
    fn test_tableau_place_neg_pos() {
        let tableau = Tableau::new((7, 7));
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
            tableau
                .place(
                    &super_armor,
                    Placement {
                        loc: Location {
                            position: (-1, 0),
                            rotation: 0,
                        },
                        part_index: 0,
                        part_shape_index: 0,
                        color: 0
                    },
                )
                .unwrap()
                .arr,
            expected_repr
        );
    }

    #[test]
    fn test_tableau_place_source_clipped() {
        let tableau = Tableau::new((7, 7));
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
            tableau.place(
                &super_armor,
                Placement {
                    loc: Location {
                        position: (-1, -1),
                        rotation: 0,
                    },
                    part_index: 0,
                    part_shape_index: 0,
                    color: 0
                },
            ),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_tableau_place_source_clipped_other_side() {
        let tableau = Tableau::new((7, 7));

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
            tableau.place(
                &super_armor,
                Placement {
                    loc: Location {
                        position: (6, 0),
                        rotation: 0,
                    },
                    part_index: 0,
                    part_shape_index: 0,
                    color: 0
                },
            ),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_tableau_destination_clobbered() {
        let mut tableau = Tableau::new((7, 7));
        tableau.arr[[0, 0]] = Some(2);

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
            tableau.place(
                &super_armor,
                Placement {
                    loc: Location {
                        position: (0, 0),
                        rotation: 0,
                    },
                    part_index: 0,
                    part_shape_index: 0,
                    color: 0
                },
            ),
            Err(PlaceError::DestinationClobbered)
        );
    }
}
