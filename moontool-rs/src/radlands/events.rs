use std::fmt;
use std::hash::{Hash, Hasher};

use by_address::ByAddress;
use itertools::Itertools;
use tui::text::Span;

use super::choices::*;
use super::styles::*;
use super::{GameResult, GameViewMut, IconEffect};

/// A type of event card.
pub struct EventType {
    /// The event's name.
    pub name: &'static str,

    /// How many of this event type are in the deck.
    pub num_in_deck: u32,

    /// The event's junk effect.
    pub junk_effect: IconEffect,

    /// The water cost to play this event.
    pub cost: u32,

    /// The number of turns this event resolves in. (Zero is immediate.)
    pub resolve_turns: u8,

    /// The handler function containing the logic to resolve this event.
    /// Takes a view from the perspective of this event's owner.
    pub on_resolve:
        for<'g, 'ctype> fn(GameViewMut<'g, 'ctype>) -> Result<ChoiceFuture<'g, 'ctype>, GameResult>,
}

// hash references by address
impl Hash for &EventType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ByAddress(*self).hash(state);
    }
}

// compare references by address
impl PartialEq for &EventType {
    fn eq(&self, other: &Self) -> bool {
        ByAddress(*self) == ByAddress(*other)
    }
}
impl Eq for &EventType {}

impl fmt::Debug for EventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "EventType[{}]", self.name)
    }
}

impl StyledName for EventType {
    /// Returns this event's name, styled for display.
    fn styled_name(&self) -> Span<'static> {
        Span::styled(self.name, *EVENT)
    }
}

pub fn get_event_types() -> Vec<EventType> {
    vec![
        EventType {
            name: "Strafe",
            num_in_deck: 2,
            junk_effect: IconEffect::Draw,
            cost: 2,
            resolve_turns: 0,
            // Injure all unprotected enemies
            on_resolve: |mut game_view| {
                game_view.injure_all_unprotected_enemies();
                Ok(ChoiceFuture::immediate(game_view.game_state))
            },
        },
        EventType {
            name: "Napalm",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 2,
            resolve_turns: 1,
            // Destroy all enemies in one column
            on_resolve: |game_view| {
                let cols_with_people = game_view
                    .other_state()
                    .enumerate_columns()
                    .filter(|(_, col)| col.people().next().is_some())
                    .map(|(col_idx, _)| col_idx)
                    .collect_vec();
                let future = if cols_with_people.is_empty() {
                    // no opponent columns have people, so resolving this event is a no-op
                    ChoiceFuture::immediate(game_view.game_state)
                } else {
                    DamageColumnChoice::future(game_view.player, true, true, cols_with_people)
                };
                Ok(future)
            },
        },
    ]
}
