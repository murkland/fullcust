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

/// A placement determines where to place a NaviCust part.
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    /// Which part to place.
    pub part_and_shape_index: (usize, usize),

    /// Where to place the part.
    pub position: (isize, isize),

    /// How many 90 degree rotations are required.
    pub rotation: usize,
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
pub struct MemoryMap {
    repr: ndarray::Array2<Option<(usize, usize)>>,
}

impl MemoryMap {
    pub fn new(size: (usize, usize)) -> Self {
        Self {
            repr: ndarray::Array2::from_elem(size, None),
        }
    }

    pub fn place(mut self, mask: &Mask, placement: Placement) -> Result<Self, PlaceError> {
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
                    *dst = Some(placement.part_and_shape_index);
                }
            }
        }

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
            Some((0, 0)), None, None, None, None, None, None,
            Some((0, 0)), Some((0, 0)), None, None, None, None, None,
            Some((0, 0)), None, None, None, None, None, None,
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
                        part_and_shape_index: (0, 0),
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
            None, None, None, None, Some((0, 0)), Some((0, 0)), Some((0, 0)),
            None, None, None, None, None, Some((0, 0)), None,
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
                        part_and_shape_index: (0, 0),
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
            None, Some((0, 0)), None, None, None, None, None,
            None, Some((0, 0)), Some((0, 0)), None, None, None, None,
            None, Some((0, 0)), None, None, None, None, None,
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
                        part_and_shape_index: (0, 0),
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
            Some((0, 0)), None, None, None, None, None, None,
            Some((0, 0)), Some((0, 0)), None, None, None, None, None,
            Some((0, 0)), None, None, None, None, None, None,
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
                        part_and_shape_index: (0, 0),
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
                    part_and_shape_index: (0, 0),
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
                    part_and_shape_index: (0, 0),
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
        memory_map.repr[[0, 0]] = Some((2, 0));

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
                    part_and_shape_index: (0, 0),
                    position: (0, 0),
                    rotation: 0,
                },
            ),
            Err(PlaceError::DestinationClobbered)
        );
    }
}
