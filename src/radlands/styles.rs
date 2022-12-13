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
