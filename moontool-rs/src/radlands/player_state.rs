use crate::cards::Cards;

use super::*;

/// Represents the state of a player's board and hand.
pub struct PlayerState<'ctype> {
    /// The cards in the player's hand, not including Water Silo.
    pub hand: Cards<PersonOrEventType<'ctype>>,

    /// When it is not this player's turn, whether this player has Water Silo
    /// in their hand. (They are assumed to not have it in their hand when it
    /// *is* this player's turn.)
    pub has_water_silo: bool,

    /// The three columns of the player's board.
    pub columns: [CardColumn<'ctype>; 3],

    /// The three event slots of the player's board.
    pub events: [Option<&'ctype (dyn EventType + 'ctype)>; 3],
}

impl<'g, 'ctype: 'g> PlayerState<'ctype> {
    /// Creates a new `PlayerState` with the given camps, drawing an initial
    /// hand from the given deck.
    pub fn new(camps: &[&'ctype CampType], deck: &mut Vec<PersonOrEventType<'ctype>>) -> Self {
        // determine the number of starting cards from the set of camps
        assert_eq!(camps.len(), 3);
        let hand_size: usize = camps.iter().map(|c| c.num_initial_cards as usize).sum();

        // draw the top hand_size cards from the deck
        let deck_cut_index = deck.len() - hand_size;
        let hand = Cards::from_iter(deck.drain(deck_cut_index..));

        PlayerState {
            hand,
            has_water_silo: false,
            columns: [
                CardColumn::new(camps[0]),
                CardColumn::new(camps[1]),
                CardColumn::new(camps[2]),
            ],
            events: [None, None, None],
        }
    }

    /// Damages the camp in the given column.
    /// Returns true if this player has no camps remaining.
    #[must_use = "if this returns true, the game must immediately end with this player losing"]
    pub fn damage_camp(&mut self, column_index: ColumnIndex) -> bool {
        self.columns[column_index.as_usize()].camp.damage();
        self.columns.iter().all(|c| c.camp.is_destroyed())
    }

    /// Returns whether this player has an empty person slot.
    pub fn has_empty_person_slot(&self) -> bool {
        self.columns
            .iter()
            .flat_map(|col| &col.person_slots)
            .any(|slot| slot.is_none())
    }

    /// Returns an iterator over the people on this player's board.
    pub fn people(&self) -> impl Iterator<Item = &Person<'ctype>> {
        self.columns.iter().flat_map(|col| col.people())
    }

    /// Returns an iterator over the locations of this player's unprotected cards.
    pub fn unprotected_cards(&self) -> impl Iterator<Item = CardLocation> + '_ {
        self.columns
            .iter()
            .enumerate()
            .filter_map(|(col_index, col)| {
                col.frontmost_card_row()
                    .map(move |row_index| CardLocation::new(col_index.into(), row_index, todo!()))
            })
    }

    /// Returns an iterator over the locations of this player's unprotected people.
    pub fn unprotected_people(&self) -> impl Iterator<Item = CardLocation> + '_ {
        self.enumerate_columns().filter_map(|(col_index, col)| {
            col.frontmost_person_row()
                .map(move |row_index| CardLocation::new(col_index, row_index.into(), todo!()))
        })
    }

    /// Returns an iterator that enumerates the columns of this player's board with strongly-typed
    /// column indices.
    pub fn enumerate_columns(&self) -> impl Iterator<Item = (ColumnIndex, &CardColumn)> + '_ {
        self.columns
            .iter()
            .enumerate()
            .map(|(col_index, col)| (col_index.into(), col))
    }

    pub fn actions(&self, game: &'g GameState<'ctype>) -> Vec<Action<'ctype>> {
        let mut actions = Vec::new();

        // actions to play or junk a card
        let can_play_card = self.has_empty_person_slot();
        for card_type in self.hand.iter_unique() {
            if can_play_card && game.cur_player_water >= card_type.cost() {
                actions.push(Action::PlayCard(card_type));
            }
            if card_type.junk_effect().can_perform(game) {
                actions.push(Action::JunkCard(card_type));
            }
        }

        // action to pay 2 water to draw a card
        // (limited to 1 use per turn)
        if game.cur_player_water >= 2 && !game.has_paid_to_draw {
            actions.push(Action::DrawCard);
        }

        // actions to use an ability
        for person in self.columns[0].people() {
            match person {
                Person::Punk(_) => {
                    // punks don't have abilities
                    // TODO: unless they're given one by another card
                }
                Person::NonPunk(NonPunk {
                    person_type,
                    is_injured,
                }) => {
                    // TODO: check if they're ready...
                    actions.push(Action::UseAbility(/*TODO*/));
                }
            }
        }

        // action to end turn (and take Water Silo if possible)
        actions.push(Action::EndTurn);

        actions
    }

    pub fn fmt(&self, f: &mut fmt::Formatter, is_cur_player: bool) -> fmt::Result {
        let prefix = format!("\x1b[{};1m|{RESET} ", if is_cur_player { 93 } else { 90 });

        writeln!(f, "{prefix}{HEADING}Hand:{RESET}")?;
        for (card_type, count) in self.hand.iter() {
            write!(f, "{prefix}  {}", card_type.styled_name())?;
            if count > 1 {
                writeln!(f, " (x{count})")?;
            } else {
                writeln!(f)?;
            }
        }
        if self.has_water_silo {
            writeln!(f, "{prefix}  {WATER}Water Silo{RESET}")?;
        } else if self.hand.is_empty() {
            writeln!(f, "{prefix}  {EMPTY}<none>{RESET}")?;
        }

        writeln!(f, "{prefix}{HEADING}Columns:{RESET}")?;
        let table_columns = self.columns.iter().map(|col| {
            vec![
                col.person_slots[1].styled_name(),
                col.person_slots[0].styled_name(),
                col.camp.styled_name(),
            ]
        });
        write!(f, "{}", StyledTable::new(table_columns, &prefix))?;

        writeln!(f, "{prefix}{HEADING}Events:{RESET}")?;
        for (i, event) in self.events.iter().enumerate() {
            write!(f, "{prefix}  [{}]  ", i + 1)?;
            if let Some(event) = event {
                writeln!(f, "{}", event.name())?;
            } else {
                writeln!(f, "{EMPTY}<none>{RESET}")?;
            }
        }

        Ok(())
    }
}

pub struct CardColumn<'ctype> {
    /// The column's camp.
    pub camp: Camp<'ctype>,

    /// The people slots in the column.
    /// The first slot (index 0) is the one in the back.
    pub person_slots: [Option<Person<'ctype>>; 2],
}

impl<'ctype> CardColumn<'ctype> {
    /// Creates a new column with the given camp.
    pub fn new(camp_type: &'ctype CampType) -> Self {
        CardColumn {
            camp: Camp {
                camp_type,
                status: CampStatus::Undamaged,
            },
            person_slots: [None, None],
        }
    }

    /// Returns an iterator over the people in the column.
    pub fn people(&self) -> impl Iterator<Item = &Person<'ctype>> {
        self.person_slots
            .iter()
            .filter_map(|person| person.as_ref())
    }

    /// Returns the row index (0 or 1) of the frontmost person in the column, or None if there are
    /// no people in the column.
    pub fn frontmost_person_row(&self) -> Option<PersonRowIndex> {
        self.person_slots
            .iter()
            .rposition(|person| person.is_some())
            .map(|row| row.into())
    }

    /// Returns the row index (0 (camp), 1, or 2) of the frontmost card in the column, or None if
    /// there are no people in the column and the camp is destroyed.
    pub fn frontmost_card_row(&self) -> Option<CardRowIndex> {
        if let Some(front_person_row) = self.frontmost_person_row() {
            // there's a person in the column; return the row of the person
            Some(front_person_row.into())
        } else if let CampStatus::Destroyed = self.camp.status {
            // there are no people and the camp is destroyed, so there are no unprotected cards in
            // this column
            None
        } else {
            // the (non-destroyed) camp is the only thing in the column
            Some(CardRowIndex::camp())
        }
    }
}

/// A camp on the board.
pub struct Camp<'ctype> {
    /// The camp type.
    pub camp_type: &'ctype CampType,

    /// The damage status of the camp.
    pub status: CampStatus,
}

impl Camp<'_> {
    /// Damages the camp.
    /// Panics if the camp is already destroyed.
    pub fn damage(&mut self) {
        match self.status {
            CampStatus::Undamaged => self.status = CampStatus::Damaged,
            CampStatus::Damaged => self.status = CampStatus::Destroyed,
            CampStatus::Destroyed => panic!("Tried to damage a destroyed camp"),
        }
    }

    /// Returns whether the camp is destroyed.
    pub fn is_destroyed(&self) -> bool {
        match self.status {
            CampStatus::Destroyed => true,
            _ => false,
        }
    }
}

impl StyledName for Camp<'_> {
    /// Returns this camps's name, styled for display.
    fn styled_name(&self) -> StyledString {
        if let CampStatus::Destroyed = self.status {
            StyledString::new("<destroyed>", CAMP_DESTROYED)
        } else {
            StyledString::new(
                self.camp_type.name,
                match self.status {
                    CampStatus::Undamaged => CAMP,
                    CampStatus::Damaged => CAMP_DAMAGED,
                    CampStatus::Destroyed => unreachable!(),
                },
            )
        }
    }
}

/// Enum representing the damage status of a camp.
pub enum CampStatus {
    Undamaged,
    Damaged,
    Destroyed,
}

/// A person played on the board (a punk or face-up person).
pub enum Person<'ctype> {
    Punk(PersonOrEventType<'ctype>),
    NonPunk(NonPunk<'ctype>),
}

impl<'ctype> Person<'ctype> {
    /// Creates a fresh person from a person type.
    pub(super) fn new_non_punk(person_type: &'ctype PersonType) -> Self {
        Person::NonPunk(NonPunk {
            person_type,
            is_injured: false,
        })
    }
}

impl StyledName for Person<'_> {
    /// Returns the name of the person, styled for display.
    fn styled_name(&self) -> StyledString {
        match self {
            Person::Punk(_) => StyledString::new("Punk", PUNK),
            Person::NonPunk(NonPunk {
                person_type,
                is_injured,
            }) => StyledString::new(
                person_type.name,
                if *is_injured {
                    PERSON_INJURED
                } else {
                    PERSON_READY
                },
            ),
        }
    }
}

impl StyledName for Option<Person<'_>> {
    /// Returns the name of the person slot, styled for display.
    fn styled_name(&self) -> StyledString {
        match self {
            Some(person) => person.styled_name(),
            None => StyledString::new("<none>", EMPTY),
        }
    }
}

/// A non-punk (face-up) person played on the board.
pub struct NonPunk<'ctype> {
    pub person_type: &'ctype PersonType,
    pub is_injured: bool,
}
