use genawaiter::yield_;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mask {
    cells: Vec<bool>,
    height: usize,
    width: usize,
}

impl Mask {
    fn as_ndarray(&self) -> ndarray::ArrayView2<bool> {
        ndarray::ArrayView2::from_shape((self.height, self.width), &self.cells).unwrap()
    }

    fn rotate90(&self) -> Self {
        let mut ndarray = self.as_ndarray().t().as_standard_layout().into_owned();
        for row in ndarray.rows_mut() {
            row.into_slice().unwrap().reverse();
        }

        let (height, width) = ndarray.dim();
        Mask {
            width,
            height,
            cells: ndarray.into_raw_vec(),
        }
    }

    fn rotate<'a>(&'a self, num: usize) -> std::borrow::Cow<'a, Self> {
        let mut mask = std::borrow::Cow::Borrowed(self);
        for _ in 0..num {
            mask = std::borrow::Cow::Owned(mask.rotate90());
        }
        mask
    }

    fn trimmed(&self) -> Self {
        let ndarray = self.as_ndarray();

        let (h, w) = ndarray.dim();

        let left = (0..w)
            .filter(|i| ndarray.column(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(0);

        let top = (0..h)
            .filter(|i| ndarray.row(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(0);

        let right = (0..w)
            .rev()
            .filter(|i| ndarray.column(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(w - 1)
            + 1;

        let bottom = (0..h)
            .rev()
            .filter(|i| ndarray.row(*i).iter().any(|v| *v))
            .next()
            .unwrap_or(h - 1)
            + 1;

        let ndarray = ndarray.slice(ndarray::s![top..bottom, left..right]);

        let (height, width) = ndarray.dim();
        Mask {
            width,
            height,
            cells: ndarray.into_owned().into_raw_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: isize,
    pub y: isize,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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
    fn new(settings: GridSettings) -> Self {
        let mut cells = ndarray::Array2::from_elem((settings.height, settings.width), Cell::Empty);

        if settings.has_oob {
            cells[[0, 0]] = Cell::Forbidden;
            cells[[settings.height - 1, 0]] = Cell::Forbidden;
            cells[[0, settings.width - 1]] = Cell::Forbidden;
            cells[[settings.height - 1, settings.width - 1]] = Cell::Forbidden;
        }

        Self {
            has_oob: settings.has_oob,
            command_line_row: settings.command_line_row,
            cells,
        }
    }

    fn place(mut self, mask: &Mask, pos: Position, requirement_index: usize) -> Option<Grid> {
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

        let mask_ndarray = mask.as_ndarray();

        // Validate that our mask isn't being weirdly clipped.
        for (y, row) in mask_ndarray.rows().into_iter().enumerate() {
            for (x, &v) in row.into_iter().enumerate() {
                // Standard stuff...
                if x >= src_x && y >= src_y && x < w - dst_x && y < h - dst_y {
                    continue;
                }

                if v {
                    return None;
                }
            }
        }

        for (src_row, dst_row) in std::iter::zip(
            mask_ndarray.slice(ndarray::s![src_y.., src_x..]).rows(),
            self.cells
                .slice_mut(ndarray::s![dst_y.., dst_x..])
                .rows_mut(),
        ) {
            for (src, dst) in std::iter::zip(src_row, dst_row) {
                if *src {
                    if !matches!(dst, Cell::Empty) {
                        return None;
                    }
                    *dst = Cell::Placed(requirement_index);
                }
            }
        }

        Some(self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub is_solid: bool,
    pub color: usize,
    pub compressed_mask: Mask,
    pub uncompressed_mask: Mask,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub part_index: usize,
    pub constraint: Constraint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constraint {
    pub compressed: Option<bool>,
    pub on_command_line: Option<bool>,
    pub bugged: Option<bool>,
}

pub type Solution = Vec<Placement>;

fn requirements_are_admissible<'a>(
    parts: &'a [Part],
    requirements: &'a [Requirement],
    grid_settings: GridSettings,
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
        > grid_settings.width * grid_settings.height - if grid_settings.has_oob { 4 } else { 0 }
    {
        return false;
    }

    true
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridSettings {
    pub height: usize,
    pub width: usize,
    pub has_oob: bool,
    pub command_line_row: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub loc: Location,
    pub compressed: bool,
}

struct Candidate {
    placement: Placement,
    mask: Mask,
}

fn placement_is_admissible<'a>(
    grid: &'a Grid,
    part_is_solid: bool,
    requirement_index: usize,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
) -> bool {
    let (h, w) = grid.cells.dim();

    // Mandatory admissibility: ensure not everything is out of bounds.
    if grid.has_oob
        && grid
            .cells
            .slice(ndarray::s![1..h - 1, 1..w - 1])
            .iter()
            .all(|v| !matches!(v, Cell::Placed(req_idx) if requirement_index == *req_idx))
    {
        return false;
    }

    // Optional admissibility: check if the block is appropriately in/out of bounds.
    let out_of_bounds =
        if grid.has_oob {
            grid.cells
                .row(0)
                .iter()
                .any(|cell| matches!(cell, Cell::Placed(req_idx) if requirement_index == *req_idx))
                || grid.cells.column(0).iter().any(
                    |cell| matches!(cell, Cell::Placed(req_idx) if requirement_index == *req_idx),
                )
                || grid.cells.row(h - 1).iter().any(
                    |cell| matches!(cell, Cell::Placed(req_idx) if requirement_index == *req_idx),
                )
                || grid.cells.column(w - 1).iter().any(
                    |cell| matches!(cell, Cell::Placed(req_idx) if requirement_index == *req_idx),
                )
        } else {
            false
        };

    // Optional admissibility: check if the block is appropriately on/off the command line.
    let placed_on_command_line = grid
        .cells
        .row(grid.command_line_row)
        .iter()
        .any(|c| matches!(c, Cell::Placed(req_idx) if requirement_index == *req_idx));

    if on_command_line
        .map(|on_command_line| on_command_line != placed_on_command_line)
        .unwrap_or(false)
    {
        return false;
    }

    let placement_is_bugged = out_of_bounds || (part_is_solid == !placed_on_command_line);

    // It is not possible to know if a piece is definitively not bugged, as it must pass the coloring check later also.
    if bugged == Some(false) && placement_is_bugged {
        return false;
    }

    true
}

fn placement_positions_for_mask<'a>(
    mask: &'a Mask,
    part_is_solid: bool,
    grid_settings: GridSettings,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
) -> Vec<Position> {
    let mut positions = vec![];

    let w = grid_settings.width as isize;
    let h = grid_settings.height as isize;

    for y in (-h + 1)..h {
        for x in (-w + 1)..w {
            let pos = Position { x, y };
            let grid = if let Some(grid) = Grid::new(grid_settings).place(mask, pos, 0) {
                grid
            } else {
                continue;
            };

            if !placement_is_admissible(&grid, part_is_solid, 0, on_command_line, bugged) {
                continue;
            }

            positions.push(pos);
        }
    }

    positions
}

fn placement_locations_and_masks_for_mask<'a>(
    mask: &'a Mask,
    part_is_solid: bool,
    grid_settings: GridSettings,
    on_command_line: Option<bool>,
    bugged: Option<bool>,
    spinnable: bool,
) -> Vec<(Location, Mask)> {
    let mask = mask.clone();

    let mut locations =
        placement_positions_for_mask(&mask, part_is_solid, grid_settings, on_command_line, bugged)
            .into_iter()
            .map(|p| {
                (
                    Location {
                        position: p,
                        rotation: 0,
                    },
                    mask.clone(),
                )
            })
            .collect::<Vec<_>>();

    if spinnable {
        // Figure out what mask rotations are necessary.
        let mut mask = std::borrow::Cow::Borrowed(&mask);

        let mut known_masks = std::collections::HashSet::with_capacity(4);
        known_masks.insert(Mask::clone(&mask.trimmed()));

        for i in 1..4 {
            mask = std::borrow::Cow::Owned(mask.rotate90());
            let trimmed_mask = mask.trimmed();
            if known_masks.contains(&trimmed_mask) {
                break;
            }
            known_masks.insert(trimmed_mask);

            locations.extend(
                placement_positions_for_mask(
                    &mask,
                    part_is_solid,
                    grid_settings,
                    on_command_line,
                    bugged,
                )
                .into_iter()
                .map(|p| {
                    (
                        Location {
                            position: p,
                            rotation: i,
                        },
                        mask.clone().into_owned(),
                    )
                }),
            );
        }
    }

    locations
}

fn candidates_for_part<'a>(
    part: &'a Part,
    grid_settings: GridSettings,
    constraint: &Constraint,
    spinnable: bool,
) -> Vec<Candidate> {
    match constraint.compressed {
        Some(true) => placement_locations_and_masks_for_mask(
            &part.compressed_mask,
            part.is_solid,
            grid_settings,
            constraint.on_command_line,
            constraint.bugged,
            spinnable,
        )
        .into_iter()
        .map(|(loc, mask)| Candidate {
            placement: Placement {
                loc,
                compressed: true,
            },
            mask,
        })
        .collect(),

        Some(false) => placement_locations_and_masks_for_mask(
            &part.compressed_mask,
            part.is_solid,
            grid_settings,
            constraint.on_command_line,
            constraint.bugged,
            spinnable,
        )
        .into_iter()
        .map(|(loc, mask)| Candidate {
            placement: Placement {
                loc,
                compressed: false,
            },
            mask,
        })
        .collect(),

        None if part.compressed_mask == part.uncompressed_mask => {
            placement_locations_and_masks_for_mask(
                &part.compressed_mask,
                part.is_solid,
                grid_settings,
                constraint.on_command_line,
                constraint.bugged,
                spinnable,
            )
            .into_iter()
            .map(|(loc, mask)| Candidate {
                placement: Placement {
                    loc,
                    compressed: true,
                },
                mask,
            })
            .collect()
        }

        None => std::iter::Iterator::chain(
            placement_locations_and_masks_for_mask(
                &part.compressed_mask,
                part.is_solid,
                grid_settings,
                constraint.on_command_line,
                constraint.bugged,
                spinnable,
            )
            .into_iter()
            .map(|(loc, mask)| Candidate {
                placement: Placement {
                    loc,
                    compressed: true,
                },
                mask,
            }),
            placement_locations_and_masks_for_mask(
                &part.uncompressed_mask,
                part.is_solid,
                grid_settings,
                constraint.on_command_line,
                constraint.bugged,
                spinnable,
            )
            .into_iter()
            .map(|(loc, mask)| Candidate {
                placement: Placement {
                    loc,
                    compressed: false,
                },
                mask,
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
    #[derive(Clone, Copy, Debug, Default)]
    struct PlacementDetail {
        out_of_bounds: bool,
        on_command_line: bool,
        touching_same_color: bool,
    }

    let mut placement_details = vec![
        PlacementDetail {
            ..Default::default()
        };
        requirements.len()
    ];
    for (y, row) in grid.cells.rows().into_iter().enumerate() {
        for (x, &cell) in row.into_iter().enumerate() {
            let req_idx = match cell {
                Cell::Placed(req_idx) => req_idx,
                _ => {
                    continue;
                }
            };
            let req = &requirements[req_idx];
            let part = &parts[req.part_index];

            let placement_detail = &mut placement_details[req_idx];
            let (h, w) = grid.cells.dim();

            // Optional admissibility: check if a block has/doesn't have any out of bounds parts.
            if grid.has_oob && (x == 0 || x == w - 1 || y == h - 1 || x == w - 1) {
                placement_detail.out_of_bounds = true;
            }

            // Optional admissibility: check if a block is/isn't on the command line.
            if y == grid.command_line_row {
                placement_detail.on_command_line = true;
            }

            // Optional admissibility: check if same-colored blocks are appropriately touching/not touching.
            if [
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

                neighbor_req_idx != req_idx && neighbor_part.color == part.color
            }) {
                placement_detail.touching_same_color = true;
            }
        }
    }

    for (req, placement_detail) in requirements.iter().zip(placement_details) {
        let part = &parts[req.part_index];
        let placement_is_bugged = placement_detail.out_of_bounds
            || (part.is_solid == !placement_detail.on_command_line)
            || placement_detail.touching_same_color;

        if req
            .constraint
            .bugged
            .map(|bugged| bugged != placement_is_bugged)
            .unwrap_or(false)
        {
            return false;
        }
    }

    true
}

pub fn place_all(
    parts: &[&Part],
    requirements: &[&Requirement],
    placements: &[&Placement],
    grid_settings: GridSettings,
) -> Option<Vec<Option<usize>>> {
    let mut grid = Grid::new(grid_settings);
    for (req_idx, (req, placement)) in
        std::iter::zip(requirements.iter(), placements.iter()).enumerate()
    {
        let part = parts[req.part_index];
        let mask = &if placement.compressed {
            &part.compressed_mask
        } else {
            &part.uncompressed_mask
        }
        .rotate(placement.loc.rotation);
        grid = if let Some(grid) = grid.place(mask, placement.loc.position, req_idx) {
            grid
        } else {
            return None;
        };
    }

    Some(
        grid.cells
            .into_iter()
            .map(|x| match x {
                Cell::Empty | Cell::Forbidden => None,
                Cell::Placed(req_idx) => Some(req_idx),
            })
            .collect(),
    )
}

pub fn solve(
    parts: Vec<Part>,
    requirements: Vec<Requirement>,
    grid_settings: GridSettings,
    spinnable_colors: Vec<bool>,
) -> impl Iterator<Item = Solution> + 'static {
    fn solve_helper(
        parts: std::rc::Rc<Vec<Part>>,
        requirements: std::rc::Rc<Vec<Requirement>>,
        grid: Grid,
        candidates: std::rc::Rc<Vec<(usize, Vec<Candidate>)>>,
        candidate_idx: usize,
        visited: std::rc::Rc<std::cell::RefCell<std::collections::HashSet<Vec<Option<usize>>>>>,
    ) -> impl Iterator<Item = Vec<(usize, Placement)>> + 'static {
        genawaiter::rc::gen!({
            let (req_idx, cands) = if let Some(candidate) = candidates.get(candidate_idx) {
                candidate
            } else {
                yield_!(Vec::with_capacity(requirements.len()));
                return;
            };

            let requirement = &requirements[*req_idx];
            let part = &parts[requirement.part_index];

            for candidate in cands {
                let grid = grid.clone();
                let grid = if let Some(grid) =
                    grid.clone()
                        .place(&candidate.mask, candidate.placement.loc.position, *req_idx)
                {
                    grid
                } else {
                    continue;
                };

                if !placement_is_admissible(
                    &grid,
                    part.is_solid,
                    *req_idx,
                    requirement.constraint.on_command_line,
                    requirement.constraint.bugged,
                ) {
                    continue;
                }

                // Ensure that we haven't seen this arrangement of parts before.
                let grid_by_parts = grid
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
                    if visited.contains(&grid_by_parts) {
                        continue;
                    }
                    visited.insert(grid_by_parts);
                }

                let solutions = solve_helper(
                    parts.clone(),
                    requirements.clone(),
                    grid.clone(),
                    candidates.clone(),
                    candidate_idx + 1,
                    visited.clone(),
                )
                .collect::<Vec<_>>();
                for mut solution in solutions {
                    solution.push((*req_idx, candidate.placement.clone()));

                    // Out of candidates! Do the final check.
                    if candidate_idx == candidates.len() - 1
                        && !solution_is_admissible(&parts, &requirements, &grid)
                    {
                        continue;
                    }

                    yield_!(solution);
                }
            }
        })
        .into_iter()
    }

    genawaiter::rc::gen!({
        if grid_settings.command_line_row >= grid_settings.height {
            return;
        }

        let num_requirements = requirements.len();

        // Very cheap check to see if this is even solvable at all.
        if !requirements_are_admissible(&parts, &requirements, grid_settings) {
            return;
        }

        let candidates = {
            let start_time = instant::Instant::now();
            let mut candidates = requirements
                .iter()
                .enumerate()
                .map(|(i, req)| {
                    let part = &parts[req.part_index];
                    (
                        i,
                        candidates_for_part(
                            part,
                            grid_settings,
                            &req.constraint,
                            spinnable_colors
                                .get(part.color)
                                .map(|v| *v)
                                .unwrap_or(false),
                        ),
                    )
                })
                .collect::<Vec<_>>();

            // Heuristic: fit hard to fit blocks first, then easier ones.
            //
            // If two blocks are just as hard to fit, make sure to group ones of the same type together.
            candidates.sort_unstable_by_key(|(i, c)| (c.len(), *i));

            log::info!(
                "candidates took {:?}, ordering: {:?}",
                instant::Instant::now() - start_time,
                candidates
                    .iter()
                    .map(|(i, c)| (*i, c.len()))
                    .collect::<Vec<_>>()
            );
            candidates
        };

        for mut solution in solve_helper(
            std::rc::Rc::new(parts),
            std::rc::Rc::new(requirements),
            Grid::new(grid_settings),
            std::rc::Rc::new(candidates),
            0,
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashSet::new())),
        ) {
            solution.sort_by_key(|(i, _)| *i);
            assert!(solution.len() == num_requirements);
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
        let mask = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, true, true, true, true, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
                true, true, true, true, false, false, false, //
            ],
        };
        let mask = mask.rotate90();
        assert_eq!(
            mask,
            Mask {
                height: 7,
                width: 7,
                cells: vec![
                    true, true, true, true, true, true, true, //
                    true, true, true, true, true, true, true, //
                    true, true, true, true, true, true, true, //
                    true, true, true, true, true, true, true, //
                    false, false, false, false, false, false, true, //
                    false, false, false, false, false, false, false, //
                    false, false, false, false, false, false, false, //
                ],
            }
        )
    }

    #[test]
    fn test_grid_place() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

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

        assert_eq!(
            grid.place(&super_armor, Position { x: 0, y: 0 }, 0)
                .unwrap()
                .cells,
            expected_repr
        );
    }

    #[test]
    fn test_grid_place_error_source_clipped_does_not_mutate() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert!(matches!(
            grid.place(&super_armor, Position { x: -1, y: 0 }, 0,),
            None
        ));
    }

    #[test]
    fn test_grid_place_error_destination_clobbered_does_not_mutate() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: true,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert!(matches!(
            grid.place(&super_armor, Position { x: 0, y: 0 }, 0),
            None
        ));
    }

    #[test]
    fn test_grid_place_oob() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: true,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

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

        assert_eq!(
            grid.place(&super_armor, Position { x: 1, y: 0 }, 0)
                .unwrap()
                .cells,
            expected_repr
        );
    }

    #[test]
    fn test_grid_place_forbidden() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: true,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert!(matches!(
            grid.place(&super_armor, Position { x: 0, y: 0 }, 0,),
            None
        ));
    }

    #[test]
    fn test_grid_place_different_sizes() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 3,
            width: 2,
            cells: vec![
                true, false, //
                true, true, //
                true, false, //
            ],
        };

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

        assert_eq!(
            grid.place(&super_armor, Position { x: 0, y: 0 }, 0)
                .unwrap()
                .cells,
            expected_repr
        );
    }

    #[test]
    fn test_grid_place_nonzero_pos() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

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

        assert_eq!(
            grid.place(&super_armor, Position { x: 1, y: 0 }, 0)
                .unwrap()
                .cells,
            expected_repr
        );
    }

    #[test]
    fn test_grid_place_neg_pos() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                false, true, false, false, false, false, false, //
                false, true, true, false, false, false, false, //
                false, true, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

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

        assert_eq!(
            grid.place(&super_armor, Position { x: -1, y: 0 }, 0)
                .unwrap()
                .cells,
            expected_repr
        );
    }

    #[test]
    fn test_grid_place_source_clipped() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert!(matches!(
            grid.place(&super_armor, Position { x: -1, y: 1 }, 0,),
            None
        ));
    }

    #[test]
    fn test_grid_place_source_clipped_other_side() {
        let grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });

        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert!(matches!(
            grid.place(&super_armor, Position { x: 6, y: 0 }, 0,),
            None
        ));
    }

    #[test]
    fn test_grid_destination_clobbered() {
        let mut grid = Grid::new(GridSettings {
            height: 7,
            width: 7,
            has_oob: false,
            command_line_row: 3,
        });
        grid.cells[[0, 0]] = Cell::Placed(2);

        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert!(matches!(
            grid.place(&super_armor, Position { x: 0, y: 0 }, 0,),
            None
        ));
    }

    #[test]
    fn test_placement_positions_for_mask() {
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert_eq!(
            placement_positions_for_mask(
                &super_armor,
                true,
                GridSettings {
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
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert_eq!(
            placement_positions_for_mask(
                &super_armor,
                true,
                GridSettings {
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
        let super_armor = Mask {
            height: 7,
            width: 7,
            cells: vec![
                true, false, false, false, false, false, false, //
                true, true, false, false, false, false, false, //
                true, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
                false, false, false, false, false, false, false, //
            ],
        };

        assert_eq!(
            placement_positions_for_mask(
                &super_armor,
                true,
                GridSettings {
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
    fn test_mask_trimmed() {
        let super_armor = Mask {
            height: 3,
            width: 3,
            cells: vec![
                true, false, false, //
                true, false, false, //
                true, false, false, //
            ],
        };

        let expected_super_armor = Mask {
            height: 3,
            width: 1,
            cells: vec![
                true, //
                true, //
                true, //
            ],
        };

        assert_eq!(super_armor.trimmed(), expected_super_armor);
    }

    #[test]
    fn test_solve() {
        let super_armor = Mask {
            height: 3,
            width: 3,
            cells: vec![
                true, false, false, //
                true, true, false, //
                true, false, false, //
            ],
        };

        assert_eq!(
            solve(
                vec![Part {
                    is_solid: true,
                    color: 0,
                    compressed_mask: super_armor.clone(),
                    uncompressed_mask: super_armor.clone(),
                }],
                vec![Requirement {
                    part_index: 0,
                    constraint: Constraint {
                        compressed: Some(true),
                        on_command_line: Some(true),
                        bugged: Some(false),
                    },
                }],
                GridSettings {
                    height: 3,
                    width: 3,
                    has_oob: false,
                    command_line_row: 1,
                },
                vec![true],
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
