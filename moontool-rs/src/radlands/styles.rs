use std::fmt::{self, Display};
use std::ops::Add;

use itertools::Itertools;

/// Resets all styling to default.
pub static RESET: &str = "\x1b[0m";

/// Resets the foreground color to default.
pub static RESET_FG: &str = "\x1b[39m";

/// Turns on bold text.
pub static BOLD: &str = "\x1b[1m";

/// Style used for player state headings.
pub static HEADING: &str = "\x1b[4;37m";

/// Style used for water-related text.
pub static WATER: &str = "\x1b[96m";

/// Style used for punk-related text.
pub static PUNK: &str = "\x1b[95m";

/// Style used for a played person that is ready.
pub static PERSON_READY: &str = "\x1b[92m";

/// Style used for a played person that uninjured but not ready.
pub static PERSON_NOT_READY: &str = "\x1b[93m";

/// Style used for a played person that is injured.
pub static PERSON_INJURED: &str = "\x1b[91m";

/// Style used for events.
pub static EVENT: &str = "\x1b[95m";

/// Style used for (undamaged) camp names.
pub static CAMP: &str = "\x1b[94m";

/// Style used for damaged camp names.
pub static CAMP_DAMAGED: &str = "\x1b[91m";

/// Style used for destroyed camps.
pub static CAMP_DESTROYED: &str = "\x1b[90m";

/// Style used to denote something missing or empty.
pub static EMPTY: &str = "\x1b[90m";

/// Style used for error text.
pub static ERROR: &str = "\x1b[91m";

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
    fn styled_name(&self) -> StyledString;
}
