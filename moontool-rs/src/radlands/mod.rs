pub mod camps;
pub mod controllers;
pub mod locations;
pub mod people;
pub mod player_state;
pub mod styles;

use by_address::ByAddress;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;

use self::camps::CampType;
use self::locations::*;
use self::people::PersonType;
use self::player_state::*;
use self::styles::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GameResult {
    P1Wins,
    P2Wins,
    Tie,
}

pub struct GameState<'ctype> {
    player1: PlayerState<'ctype>,
    player2: PlayerState<'ctype>,

    deck: Vec<PersonOrEventType<'ctype>>,
    discard: Vec<PersonOrEventType<'ctype>>,

    /// The identity of the player whose turn it currently is.
    cur_player: Player,

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
        let mut deck = Vec::new();
        for person_type in person_types {
            for _ in 0..person_type.num_in_deck {
                deck.push(PersonOrEventType::Person(person_type));
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
            cur_player: thread_rng().gen(), // randomly pick which player goes first
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
        let cur_controller = match self.cur_player {
            Player::Player1 => p1_controller,
            Player::Player2 => p2_controller,
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
        self.cur_player = self.cur_player.other();

        Ok(())
    }

    /// Has the current player damage an unprotected opponent card.
    pub fn damage_enemy(
        &mut self,
        cur_controller: &dyn PlayerController,
    ) -> Result<(), GameResult> {
        // get all possible targets
        let target_player = self.cur_player.other();
        let target_locs = self
            .player(target_player)
            .unprotected_card_locs()
            .map(|loc| loc.for_player(target_player))
            .collect_vec();

        // ask the player which one to damage
        let target_loc = cur_controller.choose_card_to_damage(self, &target_locs);

        // damage the card
        self.damage_card_at(target_loc)
    }

    /// Has the current player injure an unprotected opponent person.
    /// Assumes that the opponent has at least one person.
    pub fn injure_enemy(&mut self, cur_controller: &dyn PlayerController) {
        // get all possible targets
        let target_player = self.cur_player.other();
        let target_locs = self
            .player(target_player)
            .unprotected_person_locs()
            .map(|loc| loc.for_player(target_player))
            .collect_vec();

        // ask the player which one to injure
        let target_loc = cur_controller.choose_card_to_damage(self, &target_locs);

        // injure the person
        self.damage_card_at(target_loc)
            .expect("injure_enemy should not end the game");
    }

    /// Damages the card at the given location.
    /// Panics if there is no card there.
    fn damage_card_at(&mut self, loc: CardLocation) -> Result<(), GameResult> {
        let player_state = match loc.player() {
            Player::Player1 => &mut self.player1,
            Player::Player2 => &mut self.player2,
        };

        match loc.row().to_person_index() {
            Ok(person_row_index) => {
                // damage the person
                let slot = &mut player_state.columns[loc.column().as_usize()].person_slots
                    [person_row_index.as_usize()];
                let person = slot.as_mut().expect("Tried to damage an empty person slot");
                match person {
                    Person::Punk(card_type) => {
                        // return the card to the top of the deck
                        self.deck.push(*card_type);

                        // destroy the punk
                        *slot = None;
                    }
                    Person::NonPunk(NonPunk {
                        person_type,
                        is_injured,
                    }) => {
                        if *is_injured {
                            // the person was already injured, so now it's dead;
                            // discard the card and empty the slot
                            self.discard.push(PersonOrEventType::Person(*person_type));
                            *slot = None;
                        } else {
                            // injure the person
                            *is_injured = true;
                        }
                    }
                }
            }
            Err(()) => {
                // damage the camp in the given column and check for win condition
                let no_camps_left = player_state.damage_camp_at(loc.column());
                if no_camps_left {
                    return Err(match loc.player() {
                        Player::Player1 => GameResult::P2Wins,
                        Player::Player2 => GameResult::P1Wins,
                    });
                }
            }
        }

        Ok(())
    }

    /// Has the current player restore one of their own damaged cards.
    /// Assumes that the player has at least one restorable card.
    pub fn restore_card(&mut self, cur_controller: &dyn PlayerController) {
        // get all possible targets
        let target_locs = self.cur_player().restorable_card_locs().collect_vec();

        // ask the player which one to restore
        let target_loc = cur_controller.choose_card_to_restore(self, &target_locs);

        // restore the card
        self.cur_player_mut().restore_card_at(target_loc);
    }

    /// Draws a card from the deck.
    pub fn draw_card(&'g mut self) -> Result<PersonOrEventType<'ctype>, GameResult> {
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

    /// Plays or advances the current player's Raiders event.
    pub fn raid(&'g mut self) {
        // search for the Raiders event in the event queue
        for i in 0..self.cur_player().events.len() {
            if let Some(event) = self.cur_player().events[i] {
                if let Some(raiders) = event.as_raiders() {
                    // found the raiders event
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
                    return;
                }
            }
        }

        // if we get here, the raiders event was not found in the event queue;
        // add it to the queue
        self.play_event(&RaidersEvent);
    }

    /// Plays an event into the current player's event queue.
    /// Panics if there is not a free slot for the event.
    fn play_event(&'g mut self, event: &'ctype dyn EventType) {
        let slot_index = (event.resolve_turns() - 1) as usize;
        let free_slot = self.cur_player_mut().events[slot_index..]
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("Tried to play an event, but there was no free slot");
        *free_slot = Some(event);
    }

    /// Has the current player add a punk to their board.
    /// Does nothing if the player's board is full.
    pub fn gain_punk(&mut self, cur_controller: &dyn PlayerController) -> Result<(), GameResult> {
        if self.cur_player().has_empty_person_slot() {
            let punk = Person::Punk(self.draw_card()?);
            self.play_person(cur_controller, punk);
        }
        Ok(())
    }

    /// Asks the current player's controller to choose a location, then plays the given person
    /// onto that location.
    fn play_person(&'g mut self, cur_controller: &dyn PlayerController, person: Person<'ctype>) {
        // determine possible locations to place the card
        let mut play_locs = Vec::new();
        for (col_index, col) in self.cur_player().enumerate_columns() {
            match col.people().count() {
                0 => {
                    // no people in this column, so only one possible play location
                    play_locs.push(PlayLocation::new(col_index, 0.into()));
                }
                1 => {
                    // one person in this column, so two possible play locations
                    play_locs.push(PlayLocation::new(col_index, 0.into()));
                    play_locs.push(PlayLocation::new(col_index, 1.into()));
                }
                _ => {
                    // two people in this column, so no possible play locations
                }
            }
        }

        // ask the player controller which location to play the card into
        let play_loc = cur_controller.choose_play_location(self, &person, &play_locs);

        // place the card onto the board
        let col_index = play_loc.column().as_usize();
        let row_index = play_loc.row().as_usize();
        let col = &mut self.cur_player_mut().columns[col_index];
        if let Some(old_person) = col.person_slots[row_index].replace(person) {
            // if there was a person in the slot, move it to the other slot
            let other_row_index = 1 - row_index;
            let replaced_slot = col.person_slots[other_row_index].replace(old_person);
            assert!(replaced_slot.is_none());
        }

        // TODO: activate any on-play effect of the person
    }

    pub fn cur_player_mut(&'g mut self) -> &'g mut PlayerState<'ctype> {
        match self.cur_player {
            Player::Player1 => &mut self.player1,
            Player::Player2 => &mut self.player2,
        }
    }

    pub fn cur_player(&'g self) -> &'g PlayerState<'ctype> {
        self.player(self.cur_player)
    }

    pub fn other_player(&'g self) -> &'g PlayerState<'ctype> {
        self.player(self.cur_player.other())
    }

    pub fn player(&'g self, which: Player) -> &'g PlayerState<'ctype> {
        match which {
            Player::Player1 => &self.player1,
            Player::Player2 => &self.player2,
        }
    }
}

impl fmt::Display for GameState<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let write_player_header = |f: &mut fmt::Formatter, player: Player| {
            let n = player.number();
            if player == self.cur_player {
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
        write_player_header(f, Player::Player1)?;
        self.player1.fmt(f, self.cur_player == Player::Player1)?;
        writeln!(f)?;
        write_player_header(f, Player::Player2)?;
        self.player2.fmt(f, self.cur_player == Player::Player2)?;
        writeln!(
            f,
            "\n{} cards in deck, {} in discard",
            self.deck.len(),
            self.discard.len()
        )
    }
}

/// An action that can be performed by a player during their turn.
pub enum Action<'ctype> {
    /// Play a person or event card from the hand onto the board.
    PlayCard(PersonOrEventType<'ctype>),

    /// Draw a card (costs 2 water).
    DrawCard,

    /// Junk a card from the hand to use its junk effect.
    JunkCard(PersonOrEventType<'ctype>),

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

                match card {
                    PersonOrEventType::Person(person_type) => {
                        // play the person onto the board
                        game_state.play_person(cur_controller, Person::new_non_punk(person_type));
                    }
                    PersonOrEventType::Event(event_type) => {
                        // add the event to the event queue
                        todo!("add event to event queue");
                    }
                }

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
                card.junk_effect().perform(game_state, cur_controller)?;

                Ok(false)
            },
            Action::UseAbility(/*TODO*/) => {
                todo!("perform Action::UseAbility");
                Ok(false)
            },
            Action::EndTurn => {
                // take Water Silo if possible, then end the turn
                game_state.cur_player_mut().has_water_silo = game_state.cur_player_water >= 1;
                Ok(true)
            },
        }
    }
}

impl fmt::Display for Action<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::PlayCard(card) => write!(f, "Play {} (costs {WATER}{} water{RESET})", card.styled_name(), card.cost()),
            Action::DrawCard => write!(f, "Draw a card (costs {WATER}2 water{RESET})"),
            Action::JunkCard(card) => write!(f, "Junk {} (effect: {:?})", card.styled_name(), card.junk_effect()),
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
        person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation;

    fn choose_card_to_damage<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        target_locs: &[CardLocation],
    ) -> CardLocation;

    fn choose_card_to_restore<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation;
}

/// Enum for playable card types (people or events).
#[derive(Clone, Copy)]
pub enum PersonOrEventType<'ctype> {
    Person(&'ctype PersonType),
    Event(&'ctype dyn EventType),
}

impl PersonOrEventType<'_> {
    /// Returns the card's name.
    pub fn name(&self) -> &'static str {
        match self {
            PersonOrEventType::Person(person_type) => person_type.name,
            PersonOrEventType::Event(event_type) => event_type.name(),
        }
    }

    /// Returns the card's junk effect.
    pub fn junk_effect(&self) -> IconEffect {
        match self {
            PersonOrEventType::Person(person_type) => person_type.junk_effect,
            PersonOrEventType::Event(event_type) => event_type.junk_effect(),
        }
    }

    /// Returns the water cost to play this card.
    pub fn cost(&self) -> u32 {
        match self {
            PersonOrEventType::Person(person_type) => person_type.cost,
            PersonOrEventType::Event(event_type) => event_type.cost(),
        }
    }
}

impl StyledName for PersonOrEventType<'_> {
    /// Returns this card's name, styled for display.
    fn styled_name(&self) -> StyledString {
        match self {
            PersonOrEventType::Person(person_type) => person_type.styled_name(),
            PersonOrEventType::Event(event_type) => event_type.styled_name(),
        }
    }
}

// hash by address
impl Hash for PersonOrEventType<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            PersonOrEventType::Person(person_type) => ByAddress(person_type).hash(state),
            PersonOrEventType::Event(event_type) => ByAddress(event_type).hash(state),
        }
    }
}

// compare by address
impl PartialEq for PersonOrEventType<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (PersonOrEventType::Person(person), PersonOrEventType::Person(other_person)) => {
                ByAddress(person) == ByAddress(other_person)
            }
            (PersonOrEventType::Event(event), PersonOrEventType::Event(other_event)) => {
                ByAddress(event) == ByAddress(other_event)
            }
            _ => false,
        }
    }
}
impl Eq for PersonOrEventType<'_> {}

/// Trait for a type of event card.
pub trait EventType {
    /// Returns the event's name.
    fn name(&self) -> &'static str;

    /// Returns how many of this event type are in the deck.
    fn num_in_deck(&self) -> u32;

    /// Returns the event's junk effect.
    fn junk_effect(&self) -> IconEffect;

    /// Returns the water cost to play this event.
    fn cost(&self) -> u32;

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

impl<T: EventType + ?Sized> StyledName for T {
    /// Returns this event's name, styled for display.
    fn styled_name(&self) -> StyledString {
        StyledString::new(self.name(), EVENT)
    }
}

pub struct RaidersEvent;

impl EventType for RaidersEvent {
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

    fn resolve_turns(&self) -> u8 {
        2
    }

    fn resolve<'g, 'ctype: 'g>(&self, game_state: &'g mut GameState<'ctype>) {
        todo!("resolve Raiders event");
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
    /// Returns whether this icon effect can be performed given the current game state.
    pub fn can_perform(&self, game_state: &GameState) -> bool {
        match self {
            IconEffect::Damage => true, // if there's nothing to damage, the game is over!
            IconEffect::Injure => game_state.other_player().people().next().is_some(),
            IconEffect::Restore => game_state.cur_player().has_restorable_card(),
            IconEffect::Draw => true, // it's always possible to draw a card
            IconEffect::Water => true, // it's always possible to gain water
            IconEffect::GainPunk => game_state.cur_player().has_empty_person_slot(),
            IconEffect::Raid => game_state.cur_player().can_raid(),
        }
    }

    /// Performs the effect for the current player.
    pub fn perform(
        &self,
        game_state: &mut GameState<'_>,
        cur_controller: &dyn PlayerController,
    ) -> Result<(), GameResult> {
        match *self {
            IconEffect::Damage => {
                game_state.damage_enemy(cur_controller)?;
            }
            IconEffect::Injure => {
                game_state.injure_enemy(cur_controller);
            }
            IconEffect::Restore => {
                game_state.restore_card(cur_controller);
            }
            IconEffect::Draw => {
                game_state.draw_card_into_hand()?;
            }
            IconEffect::Water => {
                game_state.gain_water();
            }
            IconEffect::GainPunk => {
                game_state.gain_punk(cur_controller)?;
            }
            IconEffect::Raid => {
                game_state.raid();
            }
        }
        Ok(())
    }
}
