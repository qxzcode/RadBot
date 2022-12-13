use tui::layout::{Constraint, Direction, Margin, Rect};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Layout<const N: usize> {
    direction: Direction,
    margin: Margin,
    constraints: [Constraint; N],
}

impl Default for Layout<0> {
    fn default() -> Self {
        Self {
            direction: Direction::Vertical,
            margin: Margin {
                horizontal: 0,
                vertical: 0,
            },
            constraints: [],
        }
    }
}

impl<const N: usize> Layout<N> {
    pub fn constraints<const N2: usize>(self, constraints: [Constraint; N2]) -> Layout<N2> {
        Layout {
            direction: self.direction,
            margin: self.margin,
            constraints,
        }
    }

    pub fn direction(self, direction: Direction) -> Self {
        Self { direction, ..self }
    }

    #[allow(unused)]
    pub fn margin(mut self, margin: u16) -> Self {
        self.margin = Margin {
            horizontal: margin,
            vertical: margin,
        };
        self
    }

    #[allow(unused)]
    pub fn horizontal_margin(mut self, horizontal: u16) -> Self {
        self.margin.horizontal = horizontal;
        self
    }

    #[allow(unused)]
    pub fn vertical_margin(mut self, vertical: u16) -> Self {
        self.margin.vertical = vertical;
        self
    }

    pub fn split(self, area: Rect) -> [Rect; N] {
        tui::layout::Layout::default()
            .direction(self.direction)
            .vertical_margin(self.margin.vertical)
            .horizontal_margin(self.margin.horizontal)
            .constraints(self.constraints)
            .split(area)
            .try_into()
            .unwrap()
    }
}
