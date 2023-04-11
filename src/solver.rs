#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    layout_arr: ndarray::Array2<bool>,
}

impl Mask {
    pub fn new(shape: (usize, usize), mask: Vec<bool>) -> Result<Self, ndarray::ShapeError> {
        Ok(Mask {
            layout_arr: ndarray::Array2::from_shape_vec(shape, mask)?,
        })
    }

    fn rot90(self) -> Self {
        let mut mask = self
            .layout_arr
            .reversed_axes()
            .as_standard_layout()
            .into_owned();
        for row in mask.rows_mut() {
            row.into_slice().unwrap().reverse();
        }
        Mask { layout_arr: mask }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub position: (isize, isize),
    pub rotation: usize,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub loc: Location,
    pub part_index: usize,
    pub color: usize,
    pub compressed: bool,
}

#[derive(thiserror::Error, Debug)]
enum PlaceError {
    #[error("destination clobbered")]
    DestinationClobbered,

    #[error("source clipped")]
    SourceClipped,
}

#[derive(Clone, Debug)]
struct Grid {
    placements: Vec<Placement>,
    has_oob: bool,
    command_line_row: usize,
    layout_arr: ndarray::Array2<Option<usize>>,
}

impl Grid {
    fn new(size: (usize, usize), has_oob: bool, command_line_row: usize) -> Self {
        Self {
            placements: vec![],
            has_oob,
            command_line_row,
            layout_arr: ndarray::Array2::from_elem(size, None),
        }
    }

    fn placements(&self) -> &[Placement] {
        &self.placements
    }

    fn place(&mut self, mask: &Mask, placement: Placement) -> Result<(), PlaceError> {
        let placement_index = self.placements.len();

        let (h, w) = self.layout_arr.dim();

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
        for (y, row) in mask.layout_arr.rows().into_iter().enumerate() {
            for (x, &v) in row.into_iter().enumerate() {
                // Standard stuff...
                if x >= src_x && y >= src_y && x < w - dst_x && y < h - dst_y {
                    continue;
                }

                if v {
                    return Err(PlaceError::SourceClipped);
                }
            }
        }

        // Validate we're not clobbering over the destination.
        for (src_row, dst_row) in std::iter::zip(
            mask.layout_arr.slice(ndarray::s![src_y.., src_x..]).rows(),
            self.layout_arr
                .slice_mut(ndarray::s![dst_y.., dst_x..])
                .rows(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src && dst.is_some() {
                    return Err(PlaceError::DestinationClobbered);
                }
            }
        }
        for (src_row, dst_row) in std::iter::zip(
            mask.layout_arr.slice(ndarray::s![src_y.., src_x..]).rows(),
            self.layout_arr
                .slice_mut(ndarray::s![dst_y.., dst_x..])
                .rows_mut(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src {
                    *dst = Some(placement_index);
                }
            }
        }

        self.placements.push(placement);

        Ok(())
    }
}

/// A part is a NaviCust part.
#[derive(Debug, Clone)]
pub struct Part {
    pub is_solid: bool,
    pub colors: Vec<usize>,
    pub compressed_mask: Mask,
    pub uncompressed_mask: Mask,
}

pub struct Requirement {
    pub part_idx: usize,
    pub compressed: Option<bool>,
    pub on_command_line: Option<bool>,
    pub bugged: Option<bool>,
}

type Solution = Vec<Placement>;

/// Solve.
pub fn solve<'a>(
    parts: &'a [Part],
    requirements: &'a [Requirement],
    size: (usize, usize),
    has_oob: bool,
    command_line_row: usize,
) -> impl Iterator<Item = Solution> + 'a {
    let grid = Grid::new(size, has_oob, command_line_row);

    genawaiter::rc::gen!({}).into_iter()
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
    fn test_grid_place() {
        let mut grid = Grid::new((7, 7), true, 3);
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

        grid.place(
            &super_armor,
            Placement {
                loc: Location {
                    position: (0, 0),
                    rotation: 0,
                },
                part_index: 0,
                color: 0,
                compressed: false,
            },
        )
        .unwrap();

        assert_eq!(grid.layout_arr, expected_repr);
    }

    #[test]
    fn test_grid_place_different_sizes() {
        let mut grid = Grid::new((7, 7), true, 3);
        let super_armor = Mask::new(
            (3, 2),
            vec![
                true, false, //
                true, true, //
                true, false, //
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

        grid.place(
            &super_armor,
            Placement {
                loc: Location {
                    position: (0, 0),
                    rotation: 0,
                },
                part_index: 0,
                color: 0,
                compressed: false,
            },
        )
        .unwrap();

        assert_eq!(grid.layout_arr, expected_repr);
    }

    #[test]
    fn test_grid_place_rot() {
        let mut grid = Grid::new((7, 7), true, 3);
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

        grid.place(
            &super_armor,
            Placement {
                loc: Location {
                    position: (0, 0),
                    rotation: 1,
                },
                part_index: 0,
                color: 0,
                compressed: false,
            },
        )
        .unwrap();

        assert_eq!(grid.layout_arr, expected_repr);
    }

    #[test]
    fn test_grid_place_nonzero_pos() {
        let mut grid = Grid::new((7, 7), true, 3);
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

        grid.place(
            &super_armor,
            Placement {
                loc: Location {
                    position: (1, 0),
                    rotation: 0,
                },
                part_index: 0,
                color: 0,
                compressed: false,
            },
        )
        .unwrap();

        assert_eq!(grid.layout_arr, expected_repr);
    }

    #[test]
    fn test_grid_place_neg_pos() {
        let mut grid = Grid::new((7, 7), true, 3);
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

        grid.place(
            &super_armor,
            Placement {
                loc: Location {
                    position: (-1, 0),
                    rotation: 0,
                },
                part_index: 0,
                color: 0,
                compressed: false,
            },
        )
        .unwrap();

        assert_eq!(grid.layout_arr, expected_repr);
    }

    #[test]
    fn test_grid_place_source_clipped() {
        let mut grid = Grid::new((7, 7), true, 3);
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
            grid.place(
                &super_armor,
                Placement {
                    loc: Location {
                        position: (-1, -1),
                        rotation: 0,
                    },
                    part_index: 0,
                    color: 0,
                    compressed: false,
                },
            ),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_grid_place_source_clipped_other_side() {
        let mut grid = Grid::new((7, 7), true, 3);

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
            grid.place(
                &super_armor,
                Placement {
                    loc: Location {
                        position: (6, 0),
                        rotation: 0,
                    },
                    part_index: 0,
                    color: 0,
                    compressed: false,
                },
            ),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_grid_destination_clobbered() {
        let mut grid = Grid::new((7, 7), true, 3);
        grid.layout_arr[[0, 0]] = Some(2);

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
            grid.place(
                &super_armor,
                Placement {
                    loc: Location {
                        position: (0, 0),
                        rotation: 0,
                    },
                    part_index: 0,
                    color: 0,
                    compressed: false,
                },
            ),
            Err(PlaceError::DestinationClobbered)
        );
    }
}
