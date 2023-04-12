use genawaiter::yield_;

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

    fn rotate90(&self) -> Self {
        let mut cells = self.cells.t().as_standard_layout().into_owned();
        for row in cells.rows_mut() {
            row.into_slice().unwrap().reverse();
        }
        Mask { cells }
    }

    fn rotate<'a>(&'a self, num: usize) -> std::borrow::Cow<'a, Self> {
        let mut mask = std::borrow::Cow::Borrowed(self);
        for _ in 0..num {
            mask = std::borrow::Cow::Owned(mask.rotate90());
        }
        mask
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
pub struct Position {
    pub x: isize,
    pub y: isize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub position: Position,
    pub rotation: usize,
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
        let mut cells = ndarray::Array2::from_elem((settings.height, settings.width), Cell::Empty);

        if settings.has_oob {
            cells[[0, 0]] = Cell::Forbidden;
            cells[[settings.width - 1, 0]] = Cell::Forbidden;
            cells[[0, settings.height - 1]] = Cell::Forbidden;
            cells[[settings.width - 1, settings.height - 1]] = Cell::Forbidden;
        }

        Self {
            has_oob: settings.has_oob,
            command_line_row: settings.command_line_row,
            cells,
        }
    }

    fn settings(&self) -> GridSettings {
        let (h, w) = self.cells.dim();
        GridSettings {
            width: w,
            height: h,
            has_oob: self.has_oob,
            command_line_row: self.command_line_row,
        }
    }

    fn place(&mut self, mask: &Mask, pos: Position, requirement_index: usize) -> bool {
        let (h, w) = self.cells.dim();

        let (src_y, dst_y) = if pos.y < 0 {
            (-pos.y as usize, 0)
        } else {
            (0, pos.y as usize)
        };

        let (src_x, dst_x) = if pos.x < 0 {
            (-pos.x as usize, 0)
        } else {
            (0, pos.x as usize)
        };

        // Validate that our mask isn't being weirdly clipped.
        for (y, row) in mask.cells.rows().into_iter().enumerate() {
            for (x, &v) in row.into_iter().enumerate() {
                // Standard stuff...
                if x >= src_x && y >= src_y && x < w - dst_x && y < h - dst_y {
                    continue;
                }

                if v {
                    return false;
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
                    return false;
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
                    *dst = Cell::Placed(requirement_index);
                }
            }
        }

        true
    }
}

#[derive(Debug, Clone)]
pub struct Part {
    pub is_solid: bool,
    pub color: usize,
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
    // Mandatory check: blocks required to be on the command line must be less than or equal to the number of columns.
    if requirements
        .iter()
        .filter(|req| req.constraint.on_command_line == Some(true))
        .count()
        > grid_settings.width
    {
        return false;
    }

    // Mandatory check: total number of squares must be less than the total allowed space.
    let max_empty_cells =
        grid_settings.width * grid_settings.height - if grid_settings.has_oob { 4 } else { 0 };
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
    pub height: usize,
    pub width: usize,
    pub has_oob: bool,
    pub command_line_row: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub loc: Location,
    pub compressed: bool,
}

fn placement_is_admissible<'a>(
    mask: &'a Mask,
    pos: Position,
    part_is_solid: bool,
    grid_settings: &GridSettings,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
) -> bool {
    let mut grid = Grid::new(grid_settings);
    let (h, w) = grid.cells.dim();

    if !grid.place(mask, pos, 0) {
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
) -> Vec<Position> {
    let mut positions = vec![];

    let (w, h) = mask.cells.dim();
    let w = w as isize;
    let h = h as isize;

    for y in (-h + 1)..h {
        for x in (-w + 1)..w {
            let pos = Position { x, y };
            if !placement_is_admissible(
                mask,
                pos,
                part_is_solid,
                grid_settings,
                on_command_line,
                bugged,
            ) {
                continue;
            }

            positions.push(pos);
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
        mask = std::borrow::Cow::Owned(mask.rotate90());
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

fn placements<'a>(
    part: &'a Part,
    grid_settings: &GridSettings,
    constraint: &Constraint,
) -> Vec<Placement> {
    match constraint.compressed {
        Some(true) => placement_locations_for_mask(
            &part.compressed_mask,
            part.is_solid,
            grid_settings,
            constraint.on_command_line,
            constraint.bugged,
        )
        .into_iter()
        .map(|loc| Placement {
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
        .map(|loc| Placement {
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
        .map(|loc| Placement {
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
            .map(|loc| Placement {
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
            .map(|loc| Placement {
                loc,
                compressed: false,
            }),
        )
        .collect(),
    }
}

fn solution_is_admissible<'a>(
    parts: &'a [Part],
    requirements: &'a [Requirement],
    grid: &'a Grid,
) -> bool {
    // Optional admissibility: check if same-colored blocks are appropriately touching/not touching.
    //
    // I don't think this can be done incrementally because it depends on the placement of all blocks. Consider:
    //
    // 1. Optionally touchSameColor block with color X is placed.
    // 2. touchSameColor block with color Y is placed, greedily next to X.
    // 3. touchSameColor block with color Z is placed, greedily next to X or Y.
    //
    // However, valid solutions also include those where X is not placed next to Y, e.g. only Y and Z are touching and X is not.
    for (y, row) in grid.cells.rows().into_iter().enumerate() {
        for (x, &cell) in row.into_iter().enumerate() {
            let req_idx = if let Cell::Placed(req_idx) = cell {
                req_idx
            } else {
                continue;
            };
            let requirement = &requirements[req_idx];
            let part = &parts[requirement.part_index];

            let touching_same_color = [
                x.checked_sub(1).and_then(|x| grid.cells.get([y, x])),
                x.checked_add(1).and_then(|x| grid.cells.get([y, x])),
                y.checked_sub(1).and_then(|y| grid.cells.get([y, x])),
                y.checked_add(1).and_then(|y| grid.cells.get([y, x])),
            ]
            .iter()
            .any(|neighbor| {
                let neighbor_req_idx = if let Some(Cell::Placed(req_idx)) = neighbor {
                    *req_idx
                } else {
                    return false;
                };

                let neighbor_requirement = &requirements[neighbor_req_idx];
                let neighbor_part = &parts[neighbor_requirement.part_index];

                neighbor_requirement.part_index != requirement.part_index
                    && neighbor_part.color == part.color
            });

            if requirement
                .constraint
                .bugged
                .map(|bugged| bugged != touching_same_color)
                .unwrap_or(false)
            {
                return false;
            }
        }
    }

    true
}

fn solve1<'a>(
    parts: &'a [Part],
    requirements: &'a [Requirement],
    grid: Grid,
    mut candidates: Vec<(usize, Vec<Placement>)>,
    visited: std::rc::Rc<std::cell::RefCell<std::collections::HashSet<Vec<Option<usize>>>>>,
) -> impl Iterator<Item = Vec<(usize, Placement)>> + 'a {
    genawaiter::rc::gen!({
        let (req_idx, placements) = if let Some(candidate) = candidates.pop() {
            candidate
        } else {
            yield_!(vec![]);
            return;
        };

        let requirement = &requirements[req_idx];
        let part = &parts[requirement.part_index];

        for placement in placements {
            let mask = &if placement.compressed {
                &part.compressed_mask
            } else {
                &part.uncompressed_mask
            }
            .rotate(placement.loc.rotation);

            let mut grid = grid.clone();
            if !grid.place(mask, placement.loc.position, req_idx) {
                continue;
            }

            if !placement_is_admissible(
                mask,
                placement.loc.position,
                part.is_solid,
                &grid.settings(),
                requirement.constraint.on_command_line,
                requirement.constraint.bugged,
            ) {
                continue;
            }

            let parts_string = grid
                .cells
                .iter()
                .map(|cell| match cell {
                    Cell::Placed(requirement_idx) => {
                        Some(requirements[*requirement_idx].part_index)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            {
                let mut visited = visited.borrow_mut();
                if visited.contains(&parts_string) {
                    continue;
                }
                visited.insert(parts_string);
            }

            let solutions = solve1(
                parts,
                requirements,
                grid.clone(),
                candidates.clone(),
                visited.clone(),
            )
            .collect::<Vec<_>>();
            for mut solution in solutions {
                solution.push((req_idx, placement.clone()));

                // Out of candidates! Do the final check.
                if candidates.is_empty() && !solution_is_admissible(parts, requirements, &grid) {
                    continue;
                }

                yield_!(solution);
            }
        }
    })
    .into_iter()
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

        let mut candidates = requirements
            .iter()
            .enumerate()
            .map(|(i, req)| {
                (
                    i,
                    placements(&parts[req.part_index], settings, &req.constraint),
                )
            })
            .collect::<Vec<_>>();

        // Heuristic: fit hard to fit blocks first, then easier ones.
        //
        // If two blocks are just as hard to fit, make sure to group ones of the same type together.
        candidates.sort_unstable_by_key(|(i, c)| (std::cmp::Reverse(c.len()), *i));

        for mut solution in solve1(
            parts,
            requirements,
            Grid::new(settings),
            candidates,
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new())),
        ) {
            solution.sort_by_key(|(i, _)| *i);
            assert!(solution.len() == requirements.len());
            yield_!(solution.into_iter().map(|(_, p)| p).collect());
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
        let mask = mask.rotate90();
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
            height: 7,
            width: 7,
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

        assert!(grid.place(&super_armor, Position { x: 0, y: 0 }, 0));
        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_error_source_clipped_does_not_mutate() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert_eq!(
            grid.place(&super_armor, Position { x: -1, y: 0 }, 0,),
            false
        );

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_error_destination_clobbered_does_not_mutate() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert_eq!(grid.place(&super_armor, Position { x: 0, y: 0 }, 0), false);

        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_oob() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert!(grid.place(&super_armor, Position { x: 1, y: 0 }, 0));
        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_forbidden() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert_eq!(grid.place(&super_armor, Position { x: 0, y: 0 }, 0,), false);
    }

    #[test]
    fn test_grid_place_different_sizes() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert!(grid.place(&super_armor, Position { x: 0, y: 0 }, 0));
        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_nonzero_pos() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert!(grid.place(&super_armor, Position { x: 1, y: 0 }, 0));
        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_neg_pos() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert!(grid.place(&super_armor, Position { x: -1, y: 0 }, 0));
        assert_eq!(grid.cells, expected_repr);
    }

    #[test]
    fn test_grid_place_source_clipped() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert_eq!(
            grid.place(&super_armor, Position { x: -1, y: 1 }, 0,),
            false
        );
    }

    #[test]
    fn test_grid_place_source_clipped_other_side() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert_eq!(grid.place(&super_armor, Position { x: 6, y: 0 }, 0,), false);
    }

    #[test]
    fn test_grid_destination_clobbered() {
        let mut grid = Grid::new(&GridSettings {
            height: 7,
            width: 7,
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

        assert_eq!(grid.place(&super_armor, Position { x: 0, y: 0 }, 0,), false);
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
                    height: 7,
                    width: 7,
                    has_oob: true,
                    command_line_row: 3,
                },
                None,
                None,
            ),
            vec![
                Position { x: 1, y: 0 },
                Position { x: 2, y: 0 },
                Position { x: 3, y: 0 },
                Position { x: 4, y: 0 },
                Position { x: 5, y: 0 },
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 4, y: 1 },
                Position { x: 5, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 4, y: 2 },
                Position { x: 5, y: 2 },
                Position { x: 0, y: 3 },
                Position { x: 1, y: 3 },
                Position { x: 2, y: 3 },
                Position { x: 3, y: 3 },
                Position { x: 4, y: 3 },
                Position { x: 5, y: 3 },
                Position { x: 1, y: 4 },
                Position { x: 2, y: 4 },
                Position { x: 3, y: 4 },
                Position { x: 4, y: 4 },
                Position { x: 5, y: 4 }
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
                    height: 7,
                    width: 7,
                    has_oob: true,
                    command_line_row: 3,
                },
                Some(true),
                None,
            ),
            vec![
                Position { x: 0, y: 1 },
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 4, y: 1 },
                Position { x: 5, y: 1 },
                Position { x: 0, y: 2 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 4, y: 2 },
                Position { x: 5, y: 2 },
                Position { x: 0, y: 3 },
                Position { x: 1, y: 3 },
                Position { x: 2, y: 3 },
                Position { x: 3, y: 3 },
                Position { x: 4, y: 3 },
                Position { x: 5, y: 3 }
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
                    height: 7,
                    width: 7,
                    has_oob: true,
                    command_line_row: 3,
                },
                None,
                Some(false),
            ),
            vec![
                Position { x: 1, y: 1 },
                Position { x: 2, y: 1 },
                Position { x: 3, y: 1 },
                Position { x: 4, y: 1 },
                Position { x: 1, y: 2 },
                Position { x: 2, y: 2 },
                Position { x: 3, y: 2 },
                Position { x: 4, y: 2 },
                Position { x: 1, y: 3 },
                Position { x: 2, y: 3 },
                Position { x: 3, y: 3 },
                Position { x: 4, y: 3 }
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
                    height: 3,
                    width: 3,
                    has_oob: false,
                    command_line_row: 1,
                },
                None,
                Some(false),
            ),
            vec![
                Location {
                    position: Position { x: 0, y: 0 },
                    rotation: 0
                },
                Location {
                    position: Position { x: 1, y: 0 },
                    rotation: 0
                },
                Location {
                    position: Position { x: 2, y: 0 },
                    rotation: 0
                },
                Location {
                    position: Position { x: 0, y: 1 },
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

    #[test]
    fn test_solve() {
        let super_armor = Mask::new(
            (3, 3),
            vec![
                true, false, false, //
                true, true, false, //
                true, false, false, //
            ],
        )
        .unwrap();

        assert_eq!(
            solve(
                &[Part {
                    is_solid: true,
                    color: 0,
                    compressed_mask: super_armor.clone(),
                    uncompressed_mask: super_armor.clone(),
                }][..],
                &[Requirement {
                    part_index: 0,
                    constraint: Constraint {
                        compressed: Some(true),
                        on_command_line: Some(true),
                        bugged: Some(false),
                    },
                }],
                &GridSettings {
                    height: 3,
                    width: 3,
                    has_oob: false,
                    command_line_row: 1,
                },
            )
            .collect::<Vec<_>>(),
            vec![
                vec![Placement {
                    loc: Location {
                        position: Position { x: 0, y: 0 },
                        rotation: 0
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: 1, y: 0 },
                        rotation: 0
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: 0, y: 0 },
                        rotation: 1
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: 0, y: 1 },
                        rotation: 1
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: -1, y: 0 },
                        rotation: 2
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: 0, y: 0 },
                        rotation: 2
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: 0, y: -1 },
                        rotation: 3
                    },
                    compressed: true
                }],
                vec![Placement {
                    loc: Location {
                        position: Position { x: 0, y: 0 },
                        rotation: 3
                    },
                    compressed: true
                }]
            ]
        );
    }
}
