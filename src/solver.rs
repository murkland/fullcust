#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    cells: ndarray::Array2<bool>,
}

impl Mask {
    pub fn new(shape: (usize, usize), mask: Vec<bool>) -> Result<Self, ndarray::ShapeError> {
        Ok(Mask {
            cells: ndarray::Array2::from_shape_vec(shape, mask)?,
        })
    }

    fn rot90(self) -> Self {
        let mut mask = self.cells.reversed_axes().as_standard_layout().into_owned();
        for row in mask.rows_mut() {
            row.into_slice().unwrap().reverse();
        }
        Mask { cells: mask }
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Empty,
    Placed(usize),
    Forbidden,
}

#[derive(Clone, Debug)]
struct Grid {
    placements: Vec<Placement>,
    has_oob: bool,
    command_line_row: usize,
    cells: ndarray::Array2<Cell>,
}

impl Grid {
    fn new(size: (usize, usize), has_oob: bool, command_line_row: usize) -> Self {
        let mut cells = ndarray::Array2::from_elem(size, Cell::Empty);

        if has_oob {
            let (w, h) = size;
            cells[[0, 0]] = Cell::Forbidden;
            cells[[w - 1, 0]] = Cell::Forbidden;
            cells[[0, h - 1]] = Cell::Forbidden;
            cells[[w - 1, h - 1]] = Cell::Forbidden;
        }

        Self {
            placements: vec![],
            has_oob,
            command_line_row,
            cells,
        }
    }

    fn placements(&self) -> &[Placement] {
        &self.placements
    }

    fn place(&mut self, mask: &Mask, placement: Placement) -> Result<(), PlaceError> {
        let placement_index = self.placements.len();

        let (h, w) = self.cells.dim();

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
        for (y, row) in mask.cells.rows().into_iter().enumerate() {
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
            mask.cells.slice(ndarray::s![src_y.., src_x..]).rows(),
            self.cells.slice(ndarray::s![dst_y.., dst_x..]).rows(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src && !matches!(dst, Cell::Empty) {
                    return Err(PlaceError::DestinationClobbered);
                }
            }
        }
        for (src_row, dst_row) in std::iter::zip(
            mask.cells.slice(ndarray::s![src_y.., src_x..]).rows(),
            self.cells
                .slice_mut(ndarray::s![dst_y.., dst_x..])
                .rows_mut(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src {
                    *dst = Cell::Placed(placement_index);
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
    pub part_index: usize,
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
        let mut grid = Grid::new((7, 7), false, 3);
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
            Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Placed(0), Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_error_source_clipped_does_not_mutate() {
        let mut grid = Grid::new((7, 7), false, 3);
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
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
        ]).unwrap();

        assert_matches::assert_matches!(
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
            ),
            Err(PlaceError::SourceClipped)
        );

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_error_destination_clobbered_does_not_mutate() {
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
            Cell::Forbidden, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Forbidden,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Forbidden, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Forbidden,
        ]).unwrap();

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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_oob() {
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
            Cell::Forbidden, Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Forbidden,
            Cell::Empty, Cell::Placed(0), Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Forbidden, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Forbidden,
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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_forbidden() {
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

    #[test]
    fn test_grid_place_different_sizes() {
        let mut grid = Grid::new((7, 7), false, 3);
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
            Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Placed(0), Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_rot() {
        let mut grid = Grid::new((7, 7), false, 3);
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
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Placed(0), Cell::Placed(0), Cell::Placed(0),
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Placed(0), Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_nonzero_pos() {
        let mut grid = Grid::new((7, 7), false, 3);
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
            Cell::Empty, Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Placed(0), Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_neg_pos() {
        let mut grid = Grid::new((7, 7), false, 3);
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
            Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Placed(0), Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Placed(0), Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
            Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty,
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

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_source_clipped() {
        let mut grid = Grid::new((7, 7), false, 3);
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
        let mut grid = Grid::new((7, 7), false, 3);

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
        let mut grid = Grid::new((7, 7), false, 3);
        grid.cells[[0, 0]] = Cell::Placed(2);

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
