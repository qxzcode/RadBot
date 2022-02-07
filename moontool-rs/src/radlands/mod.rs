pub mod camps;
pub mod people;
pub mod styles;

use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::fmt;
use std::mem;

use crate::cards::Cards;
use styles::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GameResult {
    P1Wins,
    P2Wins,
    Tie,
}

pub struct GameState<'ctype> {
    player1: PlayerState<'ctype>,
    player2: PlayerState<'ctype>,

    deck: Vec<&'ctype (dyn PersonOrEventType + 'ctype)>,
    discard: Vec<&'ctype (dyn PersonOrEventType + 'ctype)>,

    /// Whether it is currently player 1's turn.
    is_player1_turn: bool,

    /// The amount of water that the current player has available for use.
    cur_player_water: u32,

    /// Whether the current player has used the generic "pay 2 water to draw a card"
    /// ability this turn.
    has_paid_to_draw: bool,

    /// Whether the the deck has been reshuffled from the discard pile in this game.
    has_reshuffled_deck: bool,
}

impl<'g, 'ctype: 'g> GameState<'ctype> {
    pub fn new(camp_types: &'ctype [CampType], person_types: &'ctype [PersonType]) -> Self {
        // populate the deck and shuffle it
        let mut deck = Vec::<&dyn PersonOrEventType>::new();
        for person_type in person_types {
            for _ in 0..person_type.num_in_deck() {
                deck.push(person_type);
            }
        }
        deck.shuffle(&mut thread_rng());

        // pick 3 camps for each player at random
        let chosen_camps = camp_types
            .choose_multiple(&mut thread_rng(), 6)
            .collect_vec();
        let p1_camps = &chosen_camps[..3];
        let p2_camps = &chosen_camps[3..];

        GameState {
            player1: PlayerState::new(p1_camps, &mut deck),
            player2: PlayerState::new(p2_camps, &mut deck),
            deck,
            discard: Vec::new(),
            is_player1_turn: thread_rng().gen(), // randomly pick which player goes first
            cur_player_water: 0,
            has_paid_to_draw: false,
            has_reshuffled_deck: false,
        }
    }

    pub fn do_turn(
        &'g mut self,
        p1_controller: &dyn PlayerController,
        p2_controller: &dyn PlayerController,
        is_first_turn: bool,
    ) -> Result<(), GameResult> {
        let cur_controller = if self.is_player1_turn {
            p1_controller
        } else {
            p2_controller
        };

        // resolve/advance events
        if let Some(event) = self.cur_player_mut().events[0].take() {
            event.resolve(self);
        }
        self.cur_player_mut().events.rotate_left(1);

        // replenish water
        self.cur_player_water = if is_first_turn { 1 } else { 3 };
        if self.cur_player().has_water_silo {
            self.cur_player_water += 1;
            self.cur_player_mut().has_water_silo = false;
        }

        // reset other turn state
        self.has_paid_to_draw = false;

        // draw a card
        self.draw_card_into_hand()?;

        // perform actions
        loop {
            // get all the possible actions
            let actions = self.cur_player().actions(self);

            // ask the player what to do
            let action = cur_controller.choose_action(self, &actions);

            // perform the action
            if action.perform(self, cur_controller)? {
                break;
            }

            // check for win condition
            //...
        }

        // finally, switch whose turn it is
        self.is_player1_turn = !self.is_player1_turn;

        Ok(())
    }

    /// Draws a card from the deck.
    pub fn draw_card(&'g mut self) -> Result<&'ctype dyn PersonOrEventType, GameResult> {
        if self.deck.is_empty() {
            if self.discard.is_empty() {
                // Theoretically, this could legitimately happen if one or more players
                // hoard a huge amount of cards in their hand. The following behavior
                // is a bit of a hack to stop the game, since it couldn't meaningfully
                // continue in such a case.
                eprint!("{ERROR}Tried to draw, but both deck and discard are empty! ");
                eprintln!("Ending game with a tie.{RESET}");
                return Err(GameResult::Tie);
            }

            // check for tie condition
            if self.has_reshuffled_deck {
                return Err(GameResult::Tie);
            } else {
                // reshuffle the discard pile into the deck
                mem::swap(&mut self.deck, &mut self.discard);
                self.deck.shuffle(&mut thread_rng());
                self.has_reshuffled_deck = true;
            }
        }
        Ok(self.deck.pop().unwrap())
    }

    /// Draws a card from the deck and puts it in the current player's hand.
    pub fn draw_card_into_hand(&'g mut self) -> Result<(), GameResult> {
        let card = self.draw_card()?;
        self.cur_player_mut().hand.add_one(card);
        Ok(())
    }

    /// Subtracts the given amount of water from the current player's pool.
    /// Panics if the player does not have enough water.
    pub fn spend_water(&mut self, amount: u32) {
        if self.cur_player_water < amount {
            panic!(
                "Tried to spend {amount} water, but only {} available",
                self.cur_player_water
            );
        }
        self.cur_player_water -= amount;
    }

    /// Adds 1 water to the current player's pool.
    pub fn gain_water(&mut self) {
        self.cur_player_water += 1;
    }

    /// Has the current player add a punk to their board, if possible.
    pub fn gain_punk(&mut self) {
        todo!();
    }

    /// Plays or advances the current player's Raiders event.
    pub fn raid(&'g mut self) {
        for i in 0..self.cur_player().events.len() {
            if let Some(event) = self.cur_player().events[i] {
                if let Some(raiders) = event.as_raiders() {
                    // found the raiders event in the event queue
                    if i == 0 {
                        // it's the first event, so resolve it
                        raiders.resolve(self);
                    } else {
                        // it's not the first event, so advance it if possible
                        let events = &mut self.cur_player_mut().events;
                        if events[i - 1].is_none() {
                            events[i - 1] = events[i].take();
                        }
                    }
                }
            }
        }
    }

    pub fn cur_player_mut(&'g mut self) -> &'g mut PlayerState<'ctype> {
        if self.is_player1_turn {
            &mut self.player1
        } else {
            &mut self.player2
        }
    }

    pub fn cur_player(&'g self) -> &'g PlayerState<'ctype> {
        if self.is_player1_turn {
            &self.player1
        } else {
            &self.player2
        }
    }
}

impl fmt::Display for GameState<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let write_player_header = |f: &mut fmt::Formatter, is_player_1: bool| {
            let n = if is_player_1 { 1 } else { 2 };
            if is_player_1 == self.is_player1_turn {
                // current player
                writeln!(
                    f,
                    "{BOLD}Player {n} ({WATER}{} water{RESET_FG}){RESET}",
                    self.cur_player_water
                )
            } else {
                // other player
                writeln!(f, "Player {n}")
            }
        };
        write_player_header(f, true)?;
        self.player1.fmt(f, self.is_player1_turn)?;
        writeln!(f)?;
        write_player_header(f, false)?;
        self.player2.fmt(f, !self.is_player1_turn)?;
        writeln!(
            f,
            "\n{} cards in deck, {} in discard",
            self.deck.len(),
            self.discard.len()
        )
    }
}

/// A location at which to play a person onto the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayLocation {
    /// The column to play the person into (0, 1, or 2).
    column: u8,

    /// The row to play the person into (0 or 1).
    row: u8,
}

impl PlayLocation {
    /// Creates a new PlayLocation.
    pub fn new(column: u8, row: u8) -> Self {
        assert!(column < 3);
        assert!(row < 2);
        Self { column, row }
    }

    /// Returns the column (0, 1, or 2).
    pub fn column(&self) -> u8 {
        self.column
    }

    /// Returns the row (0 or 1).
    pub fn row(&self) -> u8 {
        self.row
    }
}

/// An action that can be performed by a player during their turn.
pub enum Action<'ctype> {
    /// Play a person or event card from the hand onto the board.
    PlayCard(&'ctype (dyn PersonOrEventType + 'ctype)),

    /// Draw a card (costs 2 water).
    DrawCard,

    /// Junk a card from the hand to use its junk effect.
    JunkCard(&'ctype (dyn PersonOrEventType + 'ctype)),

    /// Use an ability of a ready person or camp.
    UseAbility(/*TODO*/),

    /// End the current player's turn, taking Water Silo if possible.
    EndTurn,
}

impl<'g, 'ctype: 'g> Action<'ctype> {
    /// Performs the action on the given game state.
    /// Returns whether the player's turn should end after this action.
    fn perform(
        &self,
        game_state: &'g mut GameState<'ctype>,
        cur_controller: &dyn PlayerController,
    ) -> Result<bool, GameResult> {
        match *self {
            Action::PlayCard(card) => {
                // pay the card's cost and remove it from the player's hand
                game_state.spend_water(card.cost());
                game_state.cur_player_mut().hand.remove_one(card);

                if let Some(person_type) = card.as_person() {
                    // play the person onto the board
                    Action::play_person(game_state, cur_controller, person_type);
                } else {
                    todo!();
                }

                // let person = Person::new_non_punk(card);
                // game_state.cur_player().columns[0].people.push(person);
                Ok(false)
            },
            Action::DrawCard => {
                game_state.spend_water(2);
                game_state.draw_card_into_hand()?;
                game_state.has_paid_to_draw = true;
                Ok(false)
            },
            Action::JunkCard(card) => {
                // move the card to the discard pile
                game_state.cur_player_mut().hand.remove_one(card);
                game_state.discard.push(card);

                // perform the card's junk effect
                card.junk_effect().perform(game_state)?;

                Ok(false)
            },
            Action::UseAbility(/*TODO*/) => {
                todo!();
                Ok(false)
            },
            Action::EndTurn => {
                // take Water Silo if possible, then end the turn
                game_state.cur_player_mut().has_water_silo = game_state.cur_player_water >= 1;
                Ok(true)
            },
        }
    }

    fn play_person(
        game_state: &'g mut GameState<'ctype>,
        cur_controller: &dyn PlayerController,
        person: &'ctype PersonType,
    ) {
        // determine possible locations to place the card
        let mut play_locs = Vec::new();
        for (col_index, col) in game_state.cur_player().columns.iter().enumerate() {
            match col.people().count() {
                0 => {
                    // no people in this column, so only one possible play location
                    play_locs.push(PlayLocation::new(col_index as u8, 0));
                }
                1 => {
                    // one person in this column, so two possible play locations
                    play_locs.push(PlayLocation::new(col_index as u8, 0));
                    play_locs.push(PlayLocation::new(col_index as u8, 1));
                }
                _ => {
                    // two people in this column, so no possible play locations
                }
            }
        }

        // ask the player controller which location to play the card into
        let play_loc = cur_controller.choose_play_location(game_state, person, &play_locs);

        // place the card onto the board
        let col_index = play_loc.column() as usize;
        let row_index = play_loc.row() as usize;
        let col = &mut game_state.cur_player_mut().columns[col_index];
        if let Some(old_person) = col.person_slots[row_index].replace(Person::new_non_punk(person))
        {
            // if there was a person in the slot, move it to the other slot
            let other_row_index = 1 - row_index;
            let replaced_slot = col.person_slots[other_row_index].replace(old_person);
            assert!(replaced_slot.is_none());
        }

        // TODO: activate any on-play effect of the person
    }
}

impl fmt::Display for Action<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::PlayCard(card) => write!(f, "Play {} (costs {WATER}{} water{RESET})", card.get_styled_name(), card.cost()),
            Action::DrawCard => write!(f, "Draw a card (costs {WATER}2 water{RESET})"),
            Action::JunkCard(card) => write!(f, "Junk {}", card.get_styled_name()),
            Action::UseAbility(/*TODO*/) => write!(f, "Use ability: [TODO]"),
            Action::EndTurn => write!(f, "End turn, taking {WATER}Water Silo{RESET} if possible"),
        }
    }
}

pub trait PlayerController {
    fn choose_action<'a, 'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype>;

    fn choose_play_location<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        person: &'ctype PersonType,
        locations: &[PlayLocation],
    ) -> PlayLocation;
}

/// Represents the state of a player's board and hand.
pub struct PlayerState<'ctype> {
    /// The cards in the player's hand, not including Water Silo.
    pub hand: Cards<'ctype, dyn PersonOrEventType + 'ctype>,

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
    pub fn new(
        camps: &[&'ctype CampType],
        deck: &mut Vec<&'ctype (dyn PersonOrEventType + 'ctype)>,
    ) -> Self {
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

    /// Returns whether this player has an empty person slot.
    pub fn has_empty_person_slot(&self) -> bool {
        self.columns
            .iter()
            .any(|col| col.person_slots.iter().any(|slot| slot.is_none()))
    }

    pub fn actions(&self, game: &'g GameState<'ctype>) -> Vec<Action<'ctype>> {
        let mut actions = Vec::new();

        // actions to play or junk a card
        let can_play_card = self.has_empty_person_slot();
        for card_type in self.hand.iter_unique() {
            if can_play_card && game.cur_player_water >= card_type.cost() {
                actions.push(Action::PlayCard(card_type));
            }
            actions.push(Action::JunkCard(card_type));
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

    fn fmt(&self, f: &mut fmt::Formatter, is_cur_player: bool) -> fmt::Result {
        let prefix = format!("\x1b[{};1m|{RESET} ", if is_cur_player { 93 } else { 90 });

        writeln!(f, "{prefix}{HEADING}Hand:{RESET}")?;
        for (card_type, count) in self.hand.iter() {
            write!(f, "{prefix}  {}", card_type.get_styled_name())?;
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
                col.person_slots[1].get_styled_name(),
                col.person_slots[0].get_styled_name(),
                col.camp.get_styled_name(),
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
}

/// A camp on the board.
pub struct Camp<'ctype> {
    /// The camp type.
    pub camp_type: &'ctype CampType,

    /// The damage status of the camp.
    pub status: CampStatus,
}

impl StyledName for Camp<'_> {
    /// Returns this camps's name, styled for display.
    fn get_styled_name(&self) -> StyledString {
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
    Punk(&'ctype PersonType),
    NonPunk(NonPunk<'ctype>),
}

impl<'ctype> Person<'ctype> {
    /// Creates a fresh person from a person type.
    fn new_non_punk(person_type: &'ctype PersonType) -> Self {
        Person::NonPunk(NonPunk {
            person_type,
            is_injured: false,
        })
    }
}

impl StyledName for Person<'_> {
    /// Returns the name of the person, styled for display.
    fn get_styled_name(&self) -> StyledString {
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
    fn get_styled_name(&self) -> StyledString {
        match self {
            Some(person) => person.get_styled_name(),
            None => StyledString::new("<none>", EMPTY),
        }
    }
}

/// A non-punk (face-up) person played on the board.
pub struct NonPunk<'ctype> {
    person_type: &'ctype PersonType,
    is_injured: bool,
}

/// A type of camp card.
pub struct CampType {
    /// The camp's name.
    pub name: &'static str,

    /// The number of cards this camp grants at the start of the game.
    pub num_initial_cards: u32,
}

/// Supertrait for playable cards (people or events).
pub trait PersonOrEventType: StyledName {
    /// Returns the card's name.
    fn name(&self) -> &'static str;

    /// Returns how many of this person type are in the deck.
    fn num_in_deck(&self) -> u32;

    /// Returns the card's junk effect.
    fn junk_effect(&self) -> IconEffect;

    /// Returns the water cost to play this card.
    fn cost(&self) -> u32;

    fn as_person(&self) -> Option<&PersonType> {
        None
    }

    /// Returns whether this card is a person (not an event).
    fn is_person(&self) -> bool {
        self.as_person().is_some()
    }
}

impl<T: PersonOrEventType> StyledName for T {
    /// Returns this card's name, styled for display.
    fn get_styled_name(&self) -> StyledString {
        StyledString::new(
            self.name(),
            if self.is_person() {
                PERSON_READY
            } else {
                EVENT
            },
        )
    }
}

/// A type of person card.
pub struct PersonType {
    /// The person's name.
    pub name: &'static str,

    /// How many of this person type are in the deck.
    pub num_in_deck: u32,

    /// The person's junk effect.
    pub junk_effect: IconEffect,

    /// The water cost to play this person.
    pub cost: u32,
    // TODO: abilities
}

impl PersonOrEventType for PersonType {
    fn name(&self) -> &'static str {
        self.name
    }

    fn num_in_deck(&self) -> u32 {
        self.num_in_deck
    }

    fn junk_effect(&self) -> IconEffect {
        self.junk_effect
    }

    fn cost(&self) -> u32 {
        self.cost
    }

    fn as_person(&self) -> Option<&PersonType> {
        Some(self)
    }
}

/// Trait for a type of event card.
pub trait EventType: PersonOrEventType {
    /// Returns the number of turns before the event resolves after being played.
    fn resolve_turns(&self) -> u8;

    /// Resolves the event.
    /// TODO: Which player's event is this? With Omen Clock, it might not be
    /// the current player.
    fn resolve<'g, 'ctype: 'g>(&self, game_state: &'g mut GameState<'ctype>);

    /// Returns this event if it is the Raiders event, otherwise None.
    fn as_raiders(&self) -> Option<&RaidersEvent> {
        None
    }
}

pub struct RaidersEvent;

impl PersonOrEventType for RaidersEvent {
    fn name(&self) -> &'static str {
        "Raiders"
    }

    fn num_in_deck(&self) -> u32 {
        1
    }

    fn junk_effect(&self) -> IconEffect {
        panic!("Raiders can never be junked");
    }

    fn cost(&self) -> u32 {
        panic!("Raiders does not have a water cost");
    }
}

impl EventType for RaidersEvent {
    fn resolve_turns(&self) -> u8 {
        2
    }

    fn resolve<'g, 'ctype: 'g>(&self, game_state: &'g mut GameState<'ctype>) {
        todo!();
    }

    fn as_raiders(&self) -> Option<&RaidersEvent> {
        Some(self)
    }
}

/// Enum representing basic icon effects for abilities and junk effects.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum IconEffect {
    Damage,
    Injure,
    Restore,
    Draw,
    Water,
    GainPunk,
    Raid,
}

impl IconEffect {
    /// Performs the effect for the current player.
    pub fn perform<'g, 'ctype: 'g>(
        &self,
        game_state: &'g mut GameState<'ctype>,
    ) -> Result<(), GameResult> {
        match *self {
            IconEffect::Damage => {
                todo!();
            }
            IconEffect::Injure => {
                todo!();
            }
            IconEffect::Restore => {
                todo!();
            }
            IconEffect::Draw => {
                game_state.draw_card_into_hand()?;
            }
            IconEffect::Water => {
                game_state.gain_water();
            }
            IconEffect::GainPunk => {
                game_state.gain_punk();
            }
            IconEffect::Raid => {
                game_state.raid();
            }
        }
        Ok(())
    }
}
