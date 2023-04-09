use genawaiter::yield_;

/// An attribute kind determines what arithmetic is performed on an attribute.
#[derive(Debug, Clone, Copy)]
pub enum AttributeKind {
    /// Boolean attributes will saturate to |true| on addition.
    Boolean,

    /// Integer attributes will not saturate.
    Integer,
}

/// An attribute is an effect that can be applied, e.g. HP+, enable Super Armor, etc.
#[derive(Debug, Clone)]
pub struct Attribute {
    /// The kind of the attribute.
    pub kind: AttributeKind,
}

/// An effect is an alteration of an attribute. Each NaviCust part may impart effects (e.g. HP+, enable Super Armor, etc.)
#[derive(Debug, Clone)]
pub struct Effect {
    /// The attribute to alter.
    pub attribute_index: usize,

    /// The amount to alter the attribute by. If the attribute is boolean, this should be 1.
    pub delta: usize,
}

/// A layout represents the shape of a NaviCust part.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    mask: ndarray::Array2<bool>,
}

impl Layout {
    pub fn new(shape: (usize, usize), mask: Vec<bool>) -> Result<Self, ndarray::ShapeError> {
        Ok(Layout {
            mask: ndarray::Array2::from_shape_vec(shape, mask)?,
        })
    }

    pub fn rot90(self) -> Self {
        let mut mask = self.mask.reversed_axes().as_standard_layout().into_owned();
        for row in mask.rows_mut() {
            row.into_slice().unwrap().reverse();
        }
        Layout { mask }
    }
}

/// A part is a NaviCust part.
#[derive(Debug, Clone)]
pub struct Part {
    /// An adjacency group is a NaviCust part's color.
    pub adjacency_group: usize,

    /// The NaviCust part must be placed on the command line for its unbugged effects to be active.
    pub must_be_on_command_line: bool,

    /// Effects that occur when the NaviCust part is unbugged.
    pub unbugged_effects: Vec<Effect>,

    /// Effects that occur when the NaviCust part is bugged. If the NaviCust part is not required to be on the command line, unbugged effects should be repeated here.
    pub bugged_effects: Vec<Effect>,

    /// The layout of the part.
    pub layout: Layout,
}

/// An environment encapsulates all the starting parameters for the solver.
#[derive(Debug, Clone)]
pub struct Environment {
    /// List of attributes.
    pub attributes: Vec<Attribute>,

    /// List of eligible parts.
    pub part: Vec<Part>,

    /// Size of the NaviCust environment.
    pub size: (usize, usize),

    /// Whether or not the NaviCust's memory map has BN6-style out of bounds areas.
    pub has_oob: bool,

    /// Which row the command line is on.
    pub command_line_row: usize,
}

/// A attribute constraint is a requirement to be solved.
#[derive(Debug, Clone)]
pub struct AttributeConstraint {
    /// Minimum value for the attribute.
    pub min: Option<usize>,

    /// Maximum value for the attribute.
    pub max: Option<usize>,
}

/// A placement determines where to place a NaviCust part.
#[derive(Debug, Clone)]
pub struct Placement {
    /// Which part to place.
    pub part_index: usize,

    /// Where to place the part.
    pub position: (usize, usize),

    /// How many 90 degree rotations are required.
    pub rotation: usize,
}

type Solution = Vec<Placement>;

/// Solve.
pub fn solve(
    env: Environment,
    attribute_constraints: &[AttributeConstraint],
    want_colorbug: Option<bool>,
) -> impl Iterator<Item = Solution> {
    genawaiter::rc::gen!({
        //
    })
    .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_rot90() {
        let layout = Layout::new(
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
        let layout = layout.rot90();
        assert_eq!(
            layout,
            Layout::new(
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
}
