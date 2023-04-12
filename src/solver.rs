#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mask {
    cells: ndarray::Array2<bool>,
}

impl Mask {
    pub fn new(shape: (usize, usize), cells: Vec<bool>) -> Result<Self, ndarray::ShapeError> {
        Ok(Mask {
            cells: ndarray::Array2::from_shape_vec(shape, cells)?,
        })
    }

    fn rot90(&self) -> Self {
        let mut cells = self.cells.t().as_standard_layout().into_owned();
        for row in cells.rows_mut() {
            row.into_slice().unwrap().reverse();
        }
        Mask { cells }
    }

    fn trimmed(&self) -> Self {
        let (h, w) = self.cells.dim();

        let left = (0..w)
            .filter(|i| self.cells.column(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(0);

        let top = (0..h)
            .filter(|i| self.cells.row(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(0);

        let right = (0..w)
            .rev()
            .filter(|i| self.cells.column(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(w - 1)
            + 1;

        let bottom = (0..h)
            .rev()
            .filter(|i| self.cells.row(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(h - 1)
            + 1;

        Mask {
            cells: self
                .cells
                .slice(ndarray::s![top..bottom, left..right])
                .into_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub position: (isize, isize),
    pub rotation: usize,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub requirement_index: usize,
    pub loc: Location,
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
    has_oob: bool,
    command_line_row: usize,
    cells: ndarray::Array2<Cell>,
}

impl Grid {
    fn new(settings: &GridSettings) -> Self {
        let mut cells = ndarray::Array2::from_elem(settings.size, Cell::Empty);

        if settings.has_oob {
            let (w, h) = settings.size;
            cells[[0, 0]] = Cell::Forbidden;
            cells[[w - 1, 0]] = Cell::Forbidden;
            cells[[0, h - 1]] = Cell::Forbidden;
            cells[[w - 1, h - 1]] = Cell::Forbidden;
        }

        Self {
            has_oob: settings.has_oob,
            command_line_row: settings.command_line_row,
            cells,
        }
    }

    fn place(
        &mut self,
        mask: &Mask,
        (x, y): (isize, isize),
        placement_index: usize,
    ) -> Result<(), PlaceError> {
        let (h, w) = self.cells.dim();

        let (src_y, dst_y) = if y < 0 {
            (-y as usize, 0)
        } else {
            (0, y as usize)
        };

        let (src_x, dst_x) = if x < 0 {
            (-x as usize, 0)
        } else {
            (0, x as usize)
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

        // After this, we will start mutating.
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

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Part {
    pub is_solid: bool,
    pub colors: Vec<usize>,
    pub compressed_mask: Mask,
    pub uncompressed_mask: Mask,
}

#[derive(Debug, Clone)]
pub struct Requirement {
    pub part_index: usize,
    pub constraint: Constraint,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub compressed: Option<bool>,
    pub on_command_line: Option<bool>,
    pub bugged: Option<bool>,
}

type Solution = Vec<Placement>;

fn requirements_are_admissible<'a>(
    parts: &'a [Part],
    requirements: &'a [Requirement],
    grid_settings: &GridSettings,
) -> bool {
    let (w, h) = grid_settings.size;

    // Mandatory check: blocks required to be on the command line must be less than or equal to the number of columns.
    if requirements
        .iter()
        .filter(|req| req.constraint.on_command_line == Some(true))
        .count()
        > w
    {
        return false;
    }

    // Mandatory check: total number of squares must be less than the total allowed space.
    let max_empty_cells = w * h - if grid_settings.has_oob { 4 } else { 0 };
    if requirements
        .iter()
        .map(|req| {
            let part = &parts[req.part_index];
            if req.constraint.compressed == Some(false) {
                part.uncompressed_mask.cells.iter().filter(|x| **x).count()
            } else {
                part.compressed_mask.cells.iter().filter(|x| **x).count()
            }
        })
        .sum::<usize>()
        >= max_empty_cells
    {
        return false;
    }

    true
}

#[derive(Debug, Clone)]
pub struct GridSettings {
    pub size: (usize, usize),
    pub has_oob: bool,
    pub command_line_row: usize,
}

struct PlacementCandidate {
    loc: Location,
    compressed: bool,
}

fn placement_is_admissible<'a>(
    mask: &'a Mask,
    (x, y): (isize, isize),
    part_is_solid: bool,
    grid_settings: &GridSettings,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
) -> bool {
    let mut grid = Grid::new(grid_settings);
    let (h, w) = grid.cells.dim();

    if grid.place(mask, (x, y), 0).is_err() {
        return false;
    }

    // Optional admissibility: check if the block is appropriately in/out of bounds.
    if bugged == Some(false) && grid_settings.has_oob {
        if grid
            .cells
            .row(0)
            .iter()
            .any(|cell| matches!(cell, Cell::Placed(0)))
            || grid
                .cells
                .column(0)
                .iter()
                .any(|cell| matches!(cell, Cell::Placed(0)))
            || grid
                .cells
                .row(h - 1)
                .iter()
                .any(|cell| matches!(cell, Cell::Placed(0)))
            || grid
                .cells
                .column(w - 1)
                .iter()
                .any(|cell| matches!(cell, Cell::Placed(0)))
        {
            return false;
        }
    }

    // Optional admissibility: check if the block is appropriately on/off the command line.
    if on_command_line != None || bugged != None {
        let placed_on_command_line = grid
            .cells
            .row(grid.command_line_row)
            .iter()
            .any(|c| matches!(c, Cell::Placed(0)));

        if on_command_line
            .map(|on_command_line| on_command_line != placed_on_command_line)
            .unwrap_or(false)
        {
            return false;
        }

        if bugged
            .map(|bugged| !bugged && part_is_solid != placed_on_command_line)
            .unwrap_or(false)
        {
            return false;
        }
    }

    true
}

fn placement_positions_for_mask<'a>(
    mask: &'a Mask,
    part_is_solid: bool,
    grid_settings: &GridSettings,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
) -> Vec<(isize, isize)> {
    let mut positions = vec![];

    let (w, h) = mask.cells.dim();
    let w = w as isize;
    let h = h as isize;

    for y in (-h + 1)..h {
        for x in (-w + 1)..w {
            if !placement_is_admissible(
                mask,
                (x, y),
                part_is_solid,
                grid_settings,
                on_command_line,
                bugged,
            ) {
                continue;
            }

            positions.push((x, y));
        }
    }

    positions
}

fn placement_locations_for_mask<'a>(
    mask: &'a Mask,
    part_is_solid: bool,
    grid_settings: &GridSettings,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
) -> Vec<Location> {
    let mut locations =
        placement_positions_for_mask(mask, part_is_solid, grid_settings, on_command_line, bugged)
            .into_iter()
            .map(|p| Location {
                position: p,
                rotation: 0,
            })
            .collect::<Vec<_>>();

    // Figure out what mask rotations are necessary.
    let mut mask = std::borrow::Cow::Borrowed(mask);

    let mut known_masks = std::collections::HashSet::new();
    known_masks.insert(mask.trimmed());

    for i in 1..4 {
        mask = std::borrow::Cow::Owned(mask.rot90());
        if known_masks.contains(&mask.trimmed()) {
            break;
        }

        locations.extend(
            placement_positions_for_mask(
                &mask,
                part_is_solid,
                grid_settings,
                on_command_line,
                bugged,
            )
            .into_iter()
            .map(|p| Location {
                position: p,
                rotation: i,
            }),
        );
    }

    locations
}

fn placement_candidates<'a>(
    part: &'a Part,
    grid_settings: &GridSettings,
    constraint: &Constraint,
) -> Vec<PlacementCandidate> {
    match constraint.compressed {
        Some(true) => placement_locations_for_mask(
            &part.compressed_mask,
            part.is_solid,
            grid_settings,
            constraint.on_command_line,
            constraint.bugged,
        )
        .into_iter()
        .map(|loc| PlacementCandidate {
            loc,
            compressed: true,
        })
        .collect(),

        Some(false) => placement_locations_for_mask(
            &part.compressed_mask,
            part.is_solid,
            grid_settings,
            constraint.on_command_line,
            constraint.bugged,
        )
        .into_iter()
        .map(|loc| PlacementCandidate {
            loc,
            compressed: false,
        })
        .collect(),

        None if part.compressed_mask == part.uncompressed_mask => placement_locations_for_mask(
            &part.compressed_mask,
            part.is_solid,
            grid_settings,
            constraint.on_command_line,
            constraint.bugged,
        )
        .into_iter()
        .map(|loc| PlacementCandidate {
            loc,
            compressed: true,
        })
        .collect(),

        None => std::iter::Iterator::chain(
            placement_locations_for_mask(
                &part.compressed_mask,
                part.is_solid,
                grid_settings,
                constraint.on_command_line,
                constraint.bugged,
            )
            .into_iter()
            .map(|loc| PlacementCandidate {
                loc,
                compressed: true,
            }),
            placement_locations_for_mask(
                &part.uncompressed_mask,
                part.is_solid,
                grid_settings,
                constraint.on_command_line,
                constraint.bugged,
            )
            .into_iter()
            .map(|loc| PlacementCandidate {
                loc,
                compressed: false,
            }),
        )
        .collect(),
    }
}

pub fn solve<'a>(
    parts: &'a [Part],
    requirements: &'a [Requirement],
    settings: &'a GridSettings,
) -> impl Iterator<Item = Solution> + 'a {
    genawaiter::rc::gen!({
        if !requirements_are_admissible(parts, requirements, settings) {
            return;
        }

        let grid = Grid::new(settings);

        let mut part_placement_candidates = std::collections::HashMap::new();

        for (req_idx, req) in requirements.iter().enumerate() {
            part_placement_candidates
                .entry(req.part_index)
                .or_insert_with(|| {
                    placement_candidates(&parts[req.part_index], settings, &req.constraint)
                });
        }
    })
    .into_iter()
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
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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

        grid.place(&super_armor, (0, 0), 0).unwrap();

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_error_source_clipped_does_not_mutate() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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
            grid.place(&super_armor, (-1, 0), 0,),
            Err(PlaceError::SourceClipped)
        );

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_error_destination_clobbered_does_not_mutate() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: true,
            command_line_row: 3,
        });
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
            grid.place(&super_armor, (0, 0), 0),
            Err(PlaceError::DestinationClobbered)
        );

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_oob() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: true,
            command_line_row: 3,
        });
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

        grid.place(&super_armor, (1, 0), 0).unwrap();

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_forbidden() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: true,
            command_line_row: 3,
        });
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
            grid.place(&super_armor, (0, 0), 0,),
            Err(PlaceError::DestinationClobbered)
        );
    }

    #[test]
    fn test_grid_place_different_sizes() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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

        grid.place(&super_armor, (0, 0), 0).unwrap();

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_nonzero_pos() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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

        grid.place(&super_armor, (1, 0), 0).unwrap();

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_neg_pos() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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

        grid.place(&super_armor, (-1, 0), 0).unwrap();

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_source_clipped() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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
            grid.place(&super_armor, (-1, -1), 0,),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_grid_place_source_clipped_other_side() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });

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
            grid.place(&super_armor, (6, 0), 0,),
            Err(PlaceError::SourceClipped)
        );
    }

    #[test]
    fn test_grid_destination_clobbered() {
        let mut grid = Grid::new(&GridSettings {
            size: (7, 7),
            has_oob: false,
            command_line_row: 3,
        });
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
            grid.place(&super_armor, (0, 0), 0,),
            Err(PlaceError::DestinationClobbered)
        );
    }

    #[test]
    fn test_placement_positions_for_mask() {
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

        assert_eq!(
            placement_positions_for_mask(
                &super_armor,
                true,
                &GridSettings {
                    size: (7, 7),
                    has_oob: true,
                    command_line_row: 3,
                },
                None,
                None,
            ),
            vec![
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (5, 0),
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 1),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (5, 2),
                (0, 3),
                (1, 3),
                (2, 3),
                (3, 3),
                (4, 3),
                (5, 3),
                (1, 4),
                (2, 4),
                (3, 4),
                (4, 4),
                (5, 4)
            ]
        );
    }

    #[test]
    fn test_placement_positions_for_mask_on_command_line() {
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

        assert_eq!(
            placement_positions_for_mask(
                &super_armor,
                true,
                &GridSettings {
                    size: (7, 7),
                    has_oob: true,
                    command_line_row: 3,
                },
                Some(true),
                None,
            ),
            vec![
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 1),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (5, 2),
                (0, 3),
                (1, 3),
                (2, 3),
                (3, 3),
                (4, 3),
                (5, 3)
            ]
        );
    }

    #[test]
    fn test_placement_positions_for_mask_not_bugged() {
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

        assert_eq!(
            placement_positions_for_mask(
                &super_armor,
                true,
                &GridSettings {
                    size: (7, 7),
                    has_oob: true,
                    command_line_row: 3,
                },
                None,
                Some(false),
            ),
            vec![
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (1, 3),
                (2, 3),
                (3, 3),
                (4, 3)
            ]
        );
    }

    #[test]
    fn test_placement_locations_for_mask() {
        let super_armor = Mask::new(
            (3, 3),
            vec![
                true, false, false, //
                true, false, false, //
                true, false, false, //
            ],
        )
        .unwrap();

        assert_eq!(
            placement_locations_for_mask(
                &super_armor,
                true,
                &GridSettings {
                    size: (3, 3),
                    has_oob: false,
                    command_line_row: 1,
                },
                None,
                Some(false),
            ),
            vec![
                Location {
                    position: (0, 0),
                    rotation: 0
                },
                Location {
                    position: (1, 0),
                    rotation: 0
                },
                Location {
                    position: (2, 0),
                    rotation: 0
                },
                Location {
                    position: (0, 1),
                    rotation: 1
                },
            ]
        );
    }

    #[test]
    fn test_mask_trimmed() {
        let super_armor = Mask::new(
            (3, 3),
            vec![
                true, false, false, //
                true, false, false, //
                true, false, false, //
            ],
        )
        .unwrap();

        let expected_super_armor = Mask::new(
            (3, 1),
            vec![
                true, //
                true, //
                true, //
            ],
        )
        .unwrap();

        assert_eq!(super_armor.trimmed(), expected_super_armor);
    }
}
