use std::fmt;

/// Resets all styling to default.
pub static RESET: &str = "\x1b[0m";

/// Resets the foreground color to default.
pub static RESET_FG: &str = "\x1b[39m";

/// Turns on bold text.
pub static BOLD: &str = "\x1b[1m";

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

/// Style used for camp names.
pub static CAMP: &str = "\x1b[94m";

/// Style used to denote something missing or empty.
pub static EMPTY: &str = "\x1b[90m";

/// Style used for error text.
pub static ERROR: &str = "\x1b[91m";

pub struct StyledString<'a> {
    pub string: &'a str,
    pub style: &'a str, // ANSI escape sequence
}

impl fmt::Display for StyledString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{RESET}", self.style, self.string)
    }
}

impl StyledString<'_> {
    pub fn write_centered(&self, f: &mut fmt::Formatter, width: usize) -> fmt::Result {
        if self.string.len() > width {
            panic!("String is longer than centering width");
        }
        let initial_padding = (width - self.string.len()) / 2;
        for _ in 0..initial_padding {
            write!(f, " ")?;
        }
        write!(f, "{}{}{RESET}", self.style, self.string)?;
        for _ in 0..(width - self.string.len() - initial_padding) {
            write!(f, " ")?;
        }
        Ok(())
    }
}
