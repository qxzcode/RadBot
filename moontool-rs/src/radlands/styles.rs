use std::fmt::{self, Display};
use std::ops::Add;

use itertools::Itertools;
use lazy_static::lazy_static;
use tui::style::{Color, Modifier, Style};
use tui::text::Span;

lazy_static! {
    /// Style used for bold text.
    pub static ref BOLD: Style = Style::default().add_modifier(Modifier::BOLD);

    /// Style used for player state headings.
    pub static ref HEADING: Style = Style::default()
        .fg(Color::Gray)
        .add_modifier(Modifier::UNDERLINED);

    /// Style used for water-related text.
    pub static ref WATER: Style = Style::default().fg(Color::LightCyan);

    /// Style used for punk-related text.
    pub static ref PUNK: Style = Style::default().fg(Color::LightMagenta);

    /// Style used for a played person that is ready.
    pub static ref PERSON_READY: Style = Style::default().fg(Color::LightGreen);

    /// Style used for a played person that uninjured but not ready.
    pub static ref PERSON_NOT_READY: Style = Style::default().fg(Color::LightYellow);

    /// Style used for a played person that is injured.
    pub static ref PERSON_INJURED: Style = Style::default().fg(Color::LightRed);

    /// Style used for events.
    pub static ref EVENT: Style = Style::default().fg(Color::LightMagenta);

    /// Style used for (undamaged) camp names.
    pub static ref CAMP: Style = Style::default().fg(Color::LightBlue);

    /// Style used for damaged camp names.
    pub static ref CAMP_DAMAGED: Style = Style::default().fg(Color::LightRed);

    /// Style used for destroyed camps.
    pub static ref CAMP_DESTROYED: Style = Style::default().fg(Color::DarkGray);

    /// Style used to denote something missing or empty.
    pub static ref EMPTY: Style = Style::default().fg(Color::DarkGray);

    /// Style used for error text.
    pub static ref ERROR: Style = Style::default().fg(Color::LightRed);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledString {
    string: String,
    display_length: usize,
}

impl StyledString {
    /// Creates a new `StyledString` with the given content and style.
    pub fn new(string: &str, style: &str) -> Self {
        Self {
            string: format!("{}{}{}", style, string, RESET),
            display_length: string.chars().count(),
        }
    }

    /// Creates a new `StyledString` with the given content and plain styling.
    pub fn plain(string: &str) -> Self {
        Self::new(string, RESET)
    }

    /// Creates a new, empty `StyledString`.
    pub fn empty() -> Self {
        Self {
            string: String::new(),
            display_length: 0,
        }
    }

    /// Returns the length of the string when displayed.
    pub fn len(&self) -> usize {
        self.display_length
    }

    /// Returns whether the string is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn write_centered(&self, f: &mut fmt::Formatter, width: usize) -> fmt::Result {
        if self.len() > width {
            panic!("String is longer than centering width");
        }
        let initial_padding = (width - self.len()) / 2;
        for _ in 0..initial_padding {
            write!(f, " ")?;
        }
        self.fmt(f)?;
        for _ in 0..(width - self.len() - initial_padding) {
            write!(f, " ")?;
        }
        Ok(())
    }
}

impl fmt::Display for StyledString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl Add<&StyledString> for &StyledString {
    type Output = StyledString;

    fn add(self, other: &StyledString) -> StyledString {
        StyledString {
            string: format!("{}{}", self.string, other.string),
            display_length: self.display_length + other.display_length,
        }
    }
}

pub struct StyledTable<'a> {
    column_string_lists: Vec<Vec<StyledString>>,
    line_prefix: &'a str,
}

impl<'a> StyledTable<'a> {
    /// Creates a new `StyledTable` with the given contents.
    pub fn new(
        column_string_lists: impl IntoIterator<Item = Vec<StyledString>>,
        line_prefix: &'a str,
    ) -> Self {
        let column_string_lists = column_string_lists.into_iter().collect_vec();

        // validate the table structure
        assert!(!column_string_lists.is_empty());
        let mut column_lengths = column_string_lists.iter().map(|col| col.len());
        let first_col_len = column_lengths.next().unwrap();
        if column_lengths.any(|len| len != first_col_len) {
            panic!("All columns must have the same number of rows");
        }
        assert!(first_col_len > 0);

        Self {
            column_string_lists,
            line_prefix,
        }
    }

    /// Returns the number of rows in the table.
    pub fn row_count(&self) -> usize {
        self.column_string_lists[0].len()
    }

    /// Reduces the number of rows in the table by removing empty cells, if possible.
    pub fn reduce_rows(&mut self) -> &mut Self {
        // while all columns have at least one empty cell...
        while self
            .column_string_lists
            .iter()
            .all(|col| col.iter().any(|cell| cell.is_empty()))
        {
            // Remove the first empty cell in each column.
            for col in &mut self.column_string_lists {
                let empty_cell_index = col.iter().position(|cell| cell.is_empty()).unwrap();
                col.remove(empty_cell_index);
            }
        }

        // shift non-empty cells down to the bottom
        for col in &mut self.column_string_lists {
            col.sort_by(|a, b| b.is_empty().cmp(&a.is_empty()));
        }

        // return self
        self
    }
}

impl fmt::Display for StyledTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let column_widths = self
            .column_string_lists
            .iter()
            .map(|column_strings| column_strings.iter().map(|s| s.len()).max().unwrap() + 4)
            .collect_vec();
        for row_index in 0..self.row_count() {
            write!(f, "{}  ", self.line_prefix)?;
            for (col_index, col_width) in column_widths.iter().enumerate() {
                let column_string = &self.column_string_lists[col_index][row_index];
                column_string.write_centered(f, *col_width)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Trait for objects that have a name that's displayed with a style.
pub trait StyledName {
    /// Returns this object's name, styled for display.
    fn styled_name(&self) -> Span<'static>;
}

#[macro_export]
macro_rules! make_spans_iterable {
    ($(,)?) => {
        std::iter::empty()
    };
    (WATER_COST: $cost:expr $(, $($span:tt)*)?) => {
        [
            Span::raw(" (costs "),
            Span::styled(format!("{} water", $cost), *WATER),
            Span::raw(")"),
        ].into_iter().chain(make_spans_iterable!($($($span)*)?))
    };
    ($first_span:expr $(, $($span:tt)*)?) => {
        std::iter::once(Span::from($first_span)).chain(make_spans_iterable!($($($span)*)?))
    };
}

#[macro_export]
macro_rules! make_spans {
    ($($span:tt)*) => {{
        use $crate::make_spans_iterable;
        use ::tui::text::{Span, Spans};
        Spans::from(make_spans_iterable!($($span)*).collect_vec())
    }};
}
