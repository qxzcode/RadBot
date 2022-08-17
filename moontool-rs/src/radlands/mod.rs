pub mod abilities;
pub mod camps;
pub mod choices;
pub mod controllers;
pub mod locations;
pub mod observed_state;
pub mod people;
pub mod player_state;
pub mod styles;

use by_address::ByAddress;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use tui::text::{Span, Spans};

use crate::cards::Cards;
use crate::make_spans;

use self::abilities::Ability;
use self::camps::CampType;
use self::choices::{Choice, ChoiceFuture, DamageChoice, PlayChoice, RestoreChoice};
use self::controllers::PlayerController;
use self::locations::*;
use self::people::{PersonType, SpecialType};
use self::player_state::*;
use self::styles::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GameResult {
    P1Wins,
    P2Wins,
    Tie,
}

#[derive(Clone)]
pub struct GameState<'ctype> {
    player1: PlayerState<'ctype>,
    player2: PlayerState<'ctype>,

    deck: Vec<PersonOrEventType<'ctype>>,
    discard: Vec<PersonOrEventType<'ctype>>,

    /// The identity of the player whose turn it currently is.
    pub cur_player: Player,

    /// The amount of water that the current player has available for use.
    pub cur_player_water: u32,

    /// Whether the current player has used the generic "pay 2 water to draw a card"
    /// ability this turn.
    has_paid_to_draw: bool,

    /// Whether the current player has played an event this turn.
    has_played_event: bool,

    /// Whether the the deck has been reshuffled from the discard pile in this game.
    has_reshuffled_deck: bool,
}

impl<'g, 'ctype: 'g> GameState<'ctype> {
    /// Creates a game state and initial Choice for a random new game.
    pub fn new(
        camp_types: &'ctype [CampType],
        person_types: &'ctype [PersonType],
    ) -> (Self, Choice<'ctype>) {
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

        let mut game_state = GameState {
            player1: PlayerState::new(p1_camps, &mut deck),
            player2: PlayerState::new(p2_camps, &mut deck),
            deck,
            discard: Vec::new(),
            cur_player: thread_rng().gen(), // randomly pick which player goes first
            cur_player_water: 1,            // the first player gets 1 water for the first turn
            has_paid_to_draw: false,
            has_played_event: false,
            has_reshuffled_deck: false,
        };

        // have the current player draw a card for the start of their turn
        game_state
            .view_for_cur_mut()
            .draw_card_into_hand()
            .expect("The first draw of the game should always succeed");

        // return the game state and initial Choice of actions
        let choice = Choice::new_actions(&mut game_state);
        (game_state, choice)
    }

    pub fn player(&'g self, which: Player) -> &'g PlayerState<'ctype> {
        match which {
            Player::Player1 => &self.player1,
            Player::Player2 => &self.player2,
        }
    }

    pub fn player_mut(&'g mut self, which: Player) -> &'g mut PlayerState<'ctype> {
        match which {
            Player::Player1 => &mut self.player1,
            Player::Player2 => &mut self.player2,
        }
    }

    /// Returns a view of this game state from the perspective of the given player.
    pub fn view_for(&'g self, which: Player) -> GameView<'g, 'ctype> {
        GameView {
            game_state: self,
            player: which,
        }
    }

    /// Returns a view of this game state from the perspective of the given player.
    pub fn view_for_mut(&'g mut self, which: Player) -> GameViewMut<'g, 'ctype> {
        GameViewMut {
            game_state: self,
            player: which,
        }
    }

    /// Returns a view of this game state from the perspective of the current player.
    pub fn view_for_cur(&'g self) -> GameView<'g, 'ctype> {
        self.view_for(self.cur_player)
    }

    /// Returns a view of this game state from the perspective of the current player.
    pub fn view_for_cur_mut(&'g mut self) -> GameViewMut<'g, 'ctype> {
        self.view_for_mut(self.cur_player)
    }

    /// Resolves the current player's first event (if any), then advances any other events.
    /// Returns a future that may represent choices from the event resolution.
    fn advance_cur_events(&'g mut self) -> ChoiceFuture<'g, 'ctype> {
        let mut view = self.view_for_cur_mut();

        // take the first event (if any)
        let first_event = view.my_state_mut().events[0].take();

        // advance any other events
        view.my_state_mut().events.rotate_left(1);

        // resolve the first event (if any)
        if let Some(event) = first_event {
            // discard it if it's not Raiders
            if event.as_raiders().is_none() {
                view.game_state
                    .discard
                    .push(PersonOrEventType::Event(event));
            }

            // resolve the event
            event.resolve(&mut self.view_for_cur_mut())
        } else {
            ChoiceFuture::immediate(self)
        }
    }

    /// Ends the current player's turn and starts the next player's turn.
    /// Returns the next Choice.
    pub fn end_turn(&'g mut self) -> Result<Choice<'ctype>, GameResult> {
        // set all camps and uninjured people to be ready, and reset use counts
        for col in &mut self.player_mut(self.cur_player).columns {
            col.camp.end_turn_reset();
            for person in col.people_mut() {
                person.end_turn_reset();
            }
        }

        // switch whose turn it is
        self.cur_player = self.cur_player.other();

        // resolve/advance events
        self.advance_cur_events().then(move |game_state, _| {
            let mut view = game_state.view_for_cur_mut();

            // replenish water
            view.game_state.cur_player_water = 3;
            if view.my_state().has_water_silo {
                view.game_state.cur_player_water += 1;
                view.my_state_mut().has_water_silo = false;
            }

            // reset other turn state
            view.game_state.has_paid_to_draw = false;
            view.game_state.has_played_event = false;

            // draw a card
            view.draw_card_into_hand()?;

            // return the next choice of actions
            Ok(Choice::new_actions(game_state))
        })
    }

    /// Damages or destroys the card at the given location.
    /// If `destroy` is true, the card is always destroyed; otherwise, it is damaged.
    /// If `shift` is true and the card is destroyed, any person in front of it is shifted back.
    ///
    /// If multiple cards need to be damaged/destroyed at the same time, `damage_cards_at` must be
    /// used instead.
    ///
    /// Panics if there is no card there.
    fn damage_card_at(
        &mut self,
        loc: CardLocation,
        destroy: bool,
        shift: bool,
    ) -> Result<(), GameResult> {
        let player_state = match loc.player() {
            Player::Player1 => &mut self.player1,
            Player::Player2 => &mut self.player2,
        };

        match loc.row().to_person_index() {
            Ok(person_row_index) => {
                // damage the person
                let column = player_state.column_mut(loc.column());
                let slot = &mut column.person_slots[person_row_index.as_usize()];
                let person = slot
                    .as_mut()
                    .expect("Tried to damage or destroy an empty person slot");
                let was_destroyed = match person {
                    Person::Punk { .. } => {
                        // destroy the punk
                        *slot = None;
                        true
                    }
                    Person::NonPunk {
                        person_type,
                        status,
                        ..
                    } => {
                        if destroy || *status == NonPunkStatus::Injured {
                            // the person was killed/destroyed;
                            // discard the card and empty the slot
                            self.discard.push(PersonOrEventType::Person(*person_type));
                            *slot = None;
                            true
                        } else {
                            // injure the person
                            *status = NonPunkStatus::Injured;
                            false
                        }
                    }
                };

                // if we're supposed to shift, and if the target person was destroyed and behind
                // another person, shift the other person back
                if shift && was_destroyed && person_row_index == 0.into() {
                    column.person_slots[0] = column.person_slots[1].take();
                }
            }
            Err(()) => {
                // damage/destroy the camp in the given column and check for win condition
                let no_camps_left = player_state.damage_camp_at(loc.column(), destroy);
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

    /// Damages or destroys zero or more cards at the given set of locations.
    /// If `destroy` is true, the cards are always destroyed; otherwise, they are damaged.
    ///
    /// This function should always be used instead of calling `damage_card_at` multiple times,
    /// because it correctly handles cases where one card being destroyed causes another card to
    /// be shifted back.
    ///
    /// Assumes that all locations are unique.
    /// Panics if any location has no card there.
    fn damage_cards_at(
        &mut self,
        locations: impl IntoIterator<Item = CardLocation>,
        destroy: bool,
    ) -> Result<(), GameResult> {
        // damage/destroy all the cards without shifting any cards
        for loc in locations {
            self.damage_card_at(loc, destroy, false)?;
        }

        // shift any cards back as necessary
        for player_state in [&mut self.player1, &mut self.player2] {
            for column in &mut player_state.columns {
                if column.person_slots[0].is_none() {
                    column.person_slots[0] = column.person_slots[1].take();
                }
            }
        }

        Ok(())
    }

    /// Draws a card from the deck.
    pub fn draw_card(&'g mut self) -> Result<PersonOrEventType<'ctype>, GameResult> {
        if self.deck.is_empty() {
            if self.discard.is_empty() {
                // Both the deck and discard are empty.
                // Theoretically, this could legitimately happen if one or more players
                // hoard a huge amount of cards in their hand. The following behavior
                // is a bit of a hack to stop the game, since it couldn't meaningfully
                // continue in such a case.
                eprint!("\x1b[91m");
                eprint!("Tried to draw, but both deck and discard are empty! ");
                eprint!("Ending game with a tie.");
                eprintln!("\x1b[0m");
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

    /// Plays or advances a player's Raiders event.
    pub fn raid(&'g mut self, player: Player) -> ChoiceFuture<'g, 'ctype> {
        // search for the Raiders event in the event queue
        let my_state = self.player_mut(player);
        for i in 0..my_state.events.len() {
            if let Some(event) = my_state.events[i] {
                if let Some(raiders) = event.as_raiders() {
                    // found the raiders event
                    if i == 0 {
                        // it's the first event, so remove and resolve it
                        my_state.events[0] = None;
                        return raiders.resolve(&mut self.view_for_mut(player));
                    } else {
                        // it's not the first event, so advance it if possible
                        let events = &mut my_state.events;
                        if events[i - 1].is_none() {
                            events[i - 1] = events[i].take();
                        }
                        return ChoiceFuture::immediate(self); // no choice to make
                    }
                }
            }
        }

        // if we get here, the raiders event was not found in the event queue;
        // add it to the queue
        self.view_for_mut(player).play_event(&RaidersEvent)
    }
}

/// A view of a game from one player's perspective.
#[derive(Clone, Copy)]
pub struct GameView<'g, 'ctype: 'g> {
    /// The game state.
    game_state: &'g GameState<'ctype>,

    /// The identity of the player whose view this is for.
    player: Player,
}

/// A view of a game from one player's perspective.
pub struct GameViewMut<'g, 'ctype: 'g> {
    /// The game state.
    game_state: &'g mut GameState<'ctype>,

    /// The identity of the player whose view this is for.
    player: Player,
}

impl<'v, 'g: 'v, 'ctype: 'g> From<GameViewMut<'g, 'ctype>> for GameView<'g, 'ctype> {
    fn from(game_view_mut: GameViewMut<'g, 'ctype>) -> Self {
        Self {
            game_state: game_view_mut.game_state,
            player: game_view_mut.player,
        }
    }
}

/// Helper macro to implement functions common to both GameView and GameViewMut.
macro_rules! impl_game_view_common {
    ($ViewType:ident) => {
        impl<'v, 'g: 'v, 'ctype: 'g> $ViewType<'g, 'ctype> {
            pub fn my_state(&self) -> &PlayerState<'ctype> {
                self.game_state.player(self.player)
            }

            pub fn other_state(&self) -> &PlayerState<'ctype> {
                self.game_state.player(self.player.other())
            }

            /// Has this player damage an unprotected opponent card.
            /// Returns the location of the card that was damaged.
            pub fn damage_enemy(&self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                // get all possible targets
                let target_locs = self
                    .other_state()
                    .unprotected_card_locs()
                    .map(|loc| loc.for_player(self.player.other()))
                    .collect_vec();

                // ask the player to damage one of them
                self.choose_and_damage_card(target_locs)
            }

            /// Has this player damage any opponent card.
            pub fn damage_any_enemy(&'v self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                // get all possible targets
                let target_locs = self
                    .other_state()
                    .card_locs()
                    .map(|loc| loc.for_player(self.player.other()))
                    .collect_vec();

                // ask the player to damage one of them
                self.choose_and_damage_card(target_locs)
            }

            /// Has this player damage an unprotected opponent camp.
            pub fn damage_unprotected_camp(&self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                // get all possible targets
                let target_locs = self
                    .other_state()
                    .unprotected_card_locs()
                    .filter(|loc| loc.row().is_camp())
                    .map(|loc| loc.for_player(self.player.other()))
                    .collect_vec();

                // ask the player to damage one of them
                self.choose_and_damage_card(target_locs)
            }

            /// Has this player injure an unprotected opponent person.
            /// Assumes that the opponent has at least one person.
            pub fn injure_enemy(&self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                self.choose_and_damage_card(self.unprotected_enemies_vec())
            }

            /// Has this player destroy an unprotected opponent person.
            /// Assumes that the opponent has at least one person.
            pub fn destroy_enemy(&self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                self.choose_and_destroy_card(self.unprotected_enemies_vec())
            }

            /// Returns a Vec of the locations of all unprotected opponent people.
            fn unprotected_enemies_vec(&self) -> Vec<CardLocation> {
                self.other_state()
                    .unprotected_person_locs()
                    .map(|loc| loc.for_player(self.player.other()))
                    .collect()
            }

            /// Has this player choose and then damage a card from a given list of locations.
            /// Returns the location of the card that was damaged.
            pub fn choose_and_damage_card(
                &'v self,
                locs: Vec<CardLocation>,
            ) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                DamageChoice::future(self.player, false, locs)
            }

            /// Has this player destroy one of their own people.
            /// Assumes that the player has at least one person.
            pub fn destroy_own_person(&'v self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                // get all possible targets
                let target_locs = self
                    .my_state()
                    .person_locs()
                    .map(|loc| loc.for_player(self.player))
                    .collect_vec();

                // ask the player to destroy one of them
                self.choose_and_destroy_card(target_locs)
            }

            /// Has this player destroy an opponent camp.
            pub fn destroy_enemy_camp(&self) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                // get all possible targets (non-destroyed camps)
                let target_locs = self
                    .other_state()
                    .enumerate_camps()
                    .filter(|(_, camp)| !camp.is_destroyed())
                    .map(|(loc, _)| loc.for_player(self.player.other()))
                    .collect_vec();

                // ask the player to destroy one of them
                self.choose_and_destroy_card(target_locs)
            }

            /// Has this player choose and then destroy a card from a given list of locations.
            pub fn choose_and_destroy_card(
                &'v self,
                locs: Vec<CardLocation>,
            ) -> ChoiceFuture<'g, 'ctype, CardLocation> {
                DamageChoice::future(self.player, true, locs)
            }

            /// Returns whether this player can use the raid effect to play or advance
            /// their Raiders event.
            pub fn can_raid(&self) -> bool {
                // search for the Raiders event in the event queue
                let events = self.my_state().events;
                for i in 0..events.len() {
                    if matches!(events[i], Some(event) if event.as_raiders().is_some()) {
                        // found the raiders event
                        if i == 0 {
                            // it's the first event, so the raid effect would resolve it
                            return true;
                        } else {
                            // it's not the first event; the raid effect can only advance it if
                            // there is not an event directly in front of it
                            return events[i - 1].is_none();
                        }
                    }
                }

                // if we get here, the raiders event was not found in the event queue;
                // the raid effect can only be used if there is a free event slot for it
                self.can_play_event(RaidersEvent.resolve_turns())
            }

            /// Returns whether this player can play an event that resolves in the given number of turns.
            pub fn can_play_event(&self, resolve_turns: u8) -> bool {
                let resolve_turns = self.effective_resolve_turns(resolve_turns);
                if resolve_turns == 0 {
                    // immediately-resolving events are always allowed
                    true
                } else {
                    // other events can only be played if there is a free event slot on or after
                    // their initial slot
                    let initial_slot = resolve_turns - 1;
                    self.my_state()
                        .events[initial_slot as usize..]
                        .iter().any(|slot| slot.is_none())
                }
            }

            /// Given the "normal" resolve timer for an event, returns the *actual* resolve timer
            /// for the event if played now, taking into account other card effects.
            pub fn effective_resolve_turns(&self, resolve_turns: u8) -> u8 {
                if !self.game_state.has_played_event
                    && self.my_state().has_special_person(SpecialType::ZetoKhan)
                {
                    0  // Zeto Khan's trait: the first event played this turn resolves in 0
                } else {
                    resolve_turns
                }
            }
        }
    };
}

impl_game_view_common!(GameView);
impl_game_view_common!(GameViewMut);

impl<'v, 'g: 'v, 'ctype: 'g> GameViewMut<'g, 'ctype> {
    pub fn as_non_mut(&'v self) -> GameView<'v, 'ctype> {
        GameView {
            game_state: self.game_state,
            player: self.player,
        }
    }

    pub fn my_state_mut(&mut self) -> &mut PlayerState<'ctype> {
        self.game_state.player_mut(self.player)
    }

    pub fn other_view_mut(&'v mut self) -> GameView<'v, 'ctype> {
        GameView {
            game_state: self.game_state,
            player: self.player.other(),
        }
    }

    pub fn immediate_future(self) -> ChoiceFuture<'g, 'ctype> {
        ChoiceFuture::immediate(self.game_state)
    }

    /// Injures all unprotected opponent people.
    pub fn injure_all_unprotected_enemies(&mut self) {
        self.game_state
            .damage_cards_at(self.unprotected_enemies_vec(), false)
            .expect("injure_all_unprotected_enemies should not end the game");
    }

    /// Destroys all injured opponent people.
    pub fn destroy_all_injured_enemies(&mut self) {
        let injured_enemy_locs = self
            .other_state()
            .enumerate_people()
            .filter(|(_, person)| person.is_injured())
            .map(|(loc, _)| loc.for_player(self.player.other()))
            .collect_vec();
        self.game_state
            .damage_cards_at(injured_enemy_locs, true)
            .expect("destroy_all_injured_enemies should not end the game");
    }

    /// Has this player restore one of their own damaged cards,
    /// or does nothing if the player does not have at least one restorable card.
    pub fn restore_card(self) -> ChoiceFuture<'g, 'ctype> {
        // get all possible targets
        let target_locs = self.my_state().restorable_card_locs().collect_vec();
        if target_locs.is_empty() {
            return self.immediate_future();
        }

        // ask the player which one to restore
        RestoreChoice::future(self.player, target_locs)
    }

    /// Draws a card from the deck and puts it in this player's hand.
    /// Returns the type of the drawn card.
    pub fn draw_card_into_hand(&'v mut self) -> Result<PersonOrEventType<'ctype>, GameResult> {
        let card = self.game_state.draw_card()?;
        self.my_state_mut().hand.add_one(card);
        Ok(card)
    }

    /// Draws `n` cards from the deck and puts them in this player's hand.
    /// Returns the types of the drawn cards.
    pub fn draw_cards_into_hand(
        &'v mut self,
        n: usize,
    ) -> Result<Cards<PersonOrEventType<'ctype>>, GameResult> {
        (0..n).map(|_| self.draw_card_into_hand()).collect()
    }

    /// Plays an event into this player's event queue (or resolves it immediately
    /// if it's a 0-turn event).
    /// Panics if there is not a free slot for the event.
    fn play_event(mut self, event: &'ctype dyn EventType) -> ChoiceFuture<'g, 'ctype> {
        let resolve_turns = self.effective_resolve_turns(event.resolve_turns());
        self.game_state.has_played_event = true;
        if resolve_turns == 0 {
            event.resolve(&mut self)
        } else {
            let slot_index = (resolve_turns - 1) as usize;
            let free_slot = self.my_state_mut().events[slot_index..]
                .iter_mut()
                .find(|slot| slot.is_none())
                .expect("Tried to play an event, but there was no free slot");
            *free_slot = Some(event);

            self.immediate_future()
        }
    }

    /// Has this player add a punk to their board.
    /// Does nothing if the player's board is full.
    pub fn gain_punk(self) -> ChoiceFuture<'g, 'ctype> {
        if self.my_state().has_empty_person_slot() {
            let punk = Person::new_punk(&self.as_non_mut());
            self.play_person(punk, None)
        } else {
            self.immediate_future()
        }
    }

    /// Asks this player's controller to choose a location, then plays the given person
    /// onto that location.
    /// If `camp_destroyed` is `Some`, then the possible play locations are restricted to
    /// columns where `column.camp.is_destroyed() == camp_destroyed`.
    /// Assumes that there is at least one valid play location.
    fn play_person(
        &'v self,
        person: Person<'ctype>,
        camp_destroyed: Option<bool>,
    ) -> ChoiceFuture<'g, 'ctype> {
        // determine possible locations to place the card
        let mut play_locs = Vec::new();
        for (col_index, col) in self.my_state().enumerate_columns() {
            if matches!(camp_destroyed, Some(destroyed) if col.camp.is_destroyed() != destroyed) {
                // this column doesn't match the `camp_destroyed` requirement; skip it
                continue;
            }

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

        // ask the player which location to play the card into
        PlayChoice::future(self.player, person, play_locs)
    }
}

/// An action that can be performed by a player during their turn.
#[derive(Clone)]
pub enum Action<'ctype> {
    /// Play a person card from the hand onto the board.
    /// If the card is "Holdout", then this action only allows playing into a column
    /// whose camp is not destroyed.
    PlayPerson(&'ctype PersonType),

    /// Play a "Holdout" person into a column with a destroyed camp, for free.
    PlayHoldout(&'ctype PersonType),

    /// Play an event card from the hand onto the event queue.
    PlayEvent(&'ctype dyn EventType),

    /// Draw a card (costs 2 water).
    DrawCard,

    /// Junk a card from the hand to use its junk effect.
    JunkCard(PersonOrEventType<'ctype>),

    /// Use an ability of a ready person.
    UsePersonAbility(&'ctype dyn Ability, PlayLocation),

    /// Use an ability of a ready camp.
    UseCampAbility(&'ctype dyn Ability, ColumnIndex),

    /// End the current player's turn, taking Water Silo if possible.
    EndTurn,
}

impl<'v, 'g: 'v, 'ctype: 'g> Action<'ctype> {
    /// Performs the action on the given game view.
    /// Returns whether the player's turn should end after this action.
    fn perform(
        &self,
        mut game_view: GameViewMut<'g, 'ctype>,
    ) -> Result<Choice<'ctype>, GameResult> {
        match *self {
            Action::PlayPerson(person_type) => {
                // pay the person's cost and remove it from the player's hand
                game_view.game_state.spend_water(person_type.cost);
                game_view
                    .my_state_mut()
                    .hand
                    .remove_one(PersonOrEventType::Person(person_type));

                // play the person onto the board
                let destroyed_restriction = if person_type.special_type == SpecialType::Holdout {
                    // Only allow a `PlayPerson` action to play Holdout into columns with
                    // non-destroyed camps. Playing it for free into a column with a
                    // destroyed camp is handled by the `PlayHoldout` action variant.
                    Some(false)
                } else {
                    // No such restriction for other people.
                    None
                };
                let person = Person::new_non_punk(person_type, &game_view.as_non_mut());
                game_view
                    .play_person(person, destroyed_restriction)
                    .then(|game_state, _| Ok(Choice::new_actions(game_state)))
            }
            Action::PlayHoldout(person_type) => {
                // remove the person from the player's hand
                game_view
                    .my_state_mut()
                    .hand
                    .remove_one(PersonOrEventType::Person(person_type));

                // play the person into a column with a destroyed camp
                let person = Person::new_non_punk(person_type, &game_view.as_non_mut());
                game_view
                    .play_person(person, Some(true))
                    .then(|game_state, _| Ok(Choice::new_actions(game_state)))
            }
            Action::PlayEvent(event_type) => {
                // pay the event's cost and remove it from the player's hand
                game_view.game_state.spend_water(event_type.cost());
                game_view
                    .my_state_mut()
                    .hand
                    .remove_one(PersonOrEventType::Event(event_type));

                // play the event
                game_view
                    .play_event(event_type)
                    .then(|game_state, _| Ok(Choice::new_actions(game_state)))
            }
            Action::DrawCard => {
                game_view.game_state.spend_water(2);
                game_view.draw_card_into_hand()?;
                game_view.game_state.has_paid_to_draw = true;
                Ok(Choice::new_actions(game_view.game_state))
            }
            Action::JunkCard(card) => {
                // move the card to the discard pile
                game_view.my_state_mut().hand.remove_one(card);
                game_view.game_state.discard.push(card);

                // perform the card's junk effect
                card.junk_effect()
                    .perform(game_view)?
                    .then(|game_state, _| Ok(Choice::new_actions(game_state)))
            }
            Action::UsePersonAbility(ability, location) => {
                // pay the ability's cost
                game_view
                    .game_state
                    .spend_water(ability.cost(&game_view.as_non_mut()));

                // mark the person as no longer ready (unless Vera Vosh's trait is active and it's
                // the first time using this person this turn)
                let is_vera_vosh_trait_active = game_view
                    .my_state()
                    .has_special_person(SpecialType::VeraVosh);
                let person = game_view
                    .my_state_mut()
                    .person_mut_slot(location)
                    .expect("Tried to use a person ability, but there was no person in the slot");
                person.increment_times_used();
                if !(is_vera_vosh_trait_active && person.times_used() == 1) {
                    person.set_not_ready();
                }

                // perform the ability
                let card_loc = location.for_player(game_view.player);
                ability
                    .perform(game_view, card_loc)?
                    .then(|game_state, _| Ok(Choice::new_actions(game_state)))
            }
            Action::UseCampAbility(ability, column_index) => {
                // pay the ability's cost
                game_view
                    .game_state
                    .spend_water(ability.cost(&game_view.as_non_mut()));

                // mark the camp as no longer ready (unless Vera Vosh's trait is active and it's
                // the first time using this camp this turn)
                let is_vera_vosh_trait_active = game_view
                    .my_state()
                    .has_special_person(SpecialType::VeraVosh);
                let camp = &mut game_view.my_state_mut().column_mut(column_index).camp;
                camp.increment_times_used();
                if !(is_vera_vosh_trait_active && camp.times_used() == 1) {
                    camp.set_not_ready();
                }

                // perform the ability
                let card_loc =
                    CardLocation::new(column_index, CardRowIndex::camp(), game_view.player);
                ability
                    .perform(game_view, card_loc)?
                    .then(|game_state, _| Ok(Choice::new_actions(game_state)))
            }
            Action::EndTurn => {
                // take Water Silo if possible, then end the turn
                game_view.my_state_mut().has_water_silo =
                    game_view.game_state.cur_player_water >= 1;
                game_view.game_state.end_turn()
            }
        }
    }

    /// Formats the action for display.
    pub fn format(&self, game_view: &'v GameView<'g, 'ctype>) -> Spans<'static> {
        match *self {
            Action::PlayPerson(card) => make_spans!(
                "Play ",
                card.styled_name(),
                if card.special_type == SpecialType::Holdout {
                    " in column without destroyed camp"
                } else {
                    ""
                },
                WATER_COST: card.cost,
                if card.on_enter_play.is_some() { " <has on-enter-play effect>" } else { "" },
                if card.enters_play_ready { " <enters play ready>" } else { "" },
            ),
            Action::PlayHoldout(card) => make_spans!(
                "Play ",
                card.styled_name(),
                " in column with destroyed camp",
                WATER_COST: 0,
            ),
            Action::PlayEvent(card) => make_spans!(
                "Play ",
                card.styled_name(),
                WATER_COST: card.cost(),
            ),
            Action::DrawCard => make_spans!(
                "Draw a card",
                WATER_COST: 2,
            ),
            Action::JunkCard(card) => make_spans!(
                "Junk ",
                card.styled_name(),
                format!(" (effect: {:?})", card.junk_effect()),
            ),
            Action::UsePersonAbility(ability, location) => make_spans!(
                "Use ",
                game_view.my_state().person_slot(location).unwrap().styled_name(),
                "'s ability: ",
                ability.description(),
                WATER_COST: ability.cost(game_view),
            ),
            Action::UseCampAbility(ability, column_index) => make_spans!(
                "Use ",
                game_view.my_state().column(column_index).camp.styled_name(),
                "'s ability: ",
                ability.description(),
                WATER_COST: ability.cost(game_view),
            ),
            Action::EndTurn => make_spans!(
                "End turn, taking ",
                Span::styled("Water Silo", *WATER),
                " if possible",
            ),
        }
    }
}

/// Enum for playable card types (people or events).
#[derive(Clone, Copy, Debug)]
pub enum PersonOrEventType<'ctype> {
    Person(&'ctype PersonType),
    Event(&'ctype dyn EventType),
}

impl PersonOrEventType<'_> {
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
    fn styled_name(&self) -> Span<'static> {
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
impl Ord for PersonOrEventType<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (*self, *other) {
            (PersonOrEventType::Person(person), PersonOrEventType::Person(other_person)) => {
                ByAddress(person).cmp(&ByAddress(other_person))
            }
            (PersonOrEventType::Event(event), PersonOrEventType::Event(other_event)) => {
                ByAddress(event).cmp(&ByAddress(other_event))
            }
            (PersonOrEventType::Person(_), PersonOrEventType::Event(_)) => Ordering::Less,
            (PersonOrEventType::Event(_), PersonOrEventType::Person(_)) => Ordering::Greater,
        }
    }
}
impl PartialOrd for PersonOrEventType<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Trait for a type of event card.
pub trait EventType: fmt::Debug + Sync {
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

    /// Resolves the event. Takes a view from the perspective of this event's owner.
    /// Returns a ChoiceFuture for the event's resolution.
    fn resolve<'v, 'g: 'v, 'ctype: 'g>(
        &'ctype self,
        game_view: &'v mut GameViewMut<'g, 'ctype>,
    ) -> ChoiceFuture<'g, 'ctype>;

    /// Returns this event if it is the Raiders event, otherwise None.
    fn as_raiders(&self) -> Option<&RaidersEvent> {
        None
    }
}

impl<T: EventType + ?Sized> StyledName for T {
    /// Returns this event's name, styled for display.
    fn styled_name(&self) -> Span<'static> {
        Span::styled(self.name(), *EVENT)
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

    fn resolve<'v, 'g: 'v, 'ctype: 'g>(
        &'ctype self,
        game_view: &'v mut GameViewMut<'g, 'ctype>,
    ) -> ChoiceFuture<'g, 'ctype> {
        // have the other player choose one of their (non-destroyed) camps to damage
        let target_locs = game_view
            .other_state()
            .enumerate_camps()
            .filter_map(|(location, camp)| {
                if camp.is_destroyed() {
                    None
                } else {
                    Some(location.for_player(game_view.player.other()))
                }
            })
            .collect_vec();

        // have the other player choose one of their (non-destroyed) camps to damage
        DamageChoice::future(game_view.player.other(), false, target_locs).ignore_result()
    }

    fn as_raiders(&self) -> Option<&RaidersEvent> {
        Some(self)
    }
}

impl fmt::Debug for RaidersEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventType[Raiders]")
    }
}

/// Enum representing basic icon effects for abilities and junk effects.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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
    /// Returns whether this icon effect can be performed given a game view.
    pub fn can_perform(&self, game_view: &GameView) -> bool {
        match self {
            IconEffect::Damage => true, // if there's nothing to damage, the game is over!
            IconEffect::Injure => game_view.other_state().people().next().is_some(),
            IconEffect::Restore => game_view.my_state().has_restorable_card(),
            IconEffect::Draw => true, // it's always possible to draw a card
            IconEffect::Water => true, // it's always possible to gain water
            IconEffect::GainPunk => game_view.my_state().has_empty_person_slot(),
            IconEffect::Raid => game_view.can_raid(),
        }
    }

    /// Performs the effect for the current player.
    pub fn perform<'g, 'ctype: 'g>(
        &self,
        mut game_view: GameViewMut<'g, 'ctype>,
    ) -> Result<ChoiceFuture<'g, 'ctype>, GameResult> {
        match *self {
            IconEffect::Damage => {
                return Ok(game_view.damage_enemy().ignore_result());
            }
            IconEffect::Injure => {
                return Ok(game_view.injure_enemy().ignore_result());
            }
            IconEffect::Restore => {
                return Ok(game_view.restore_card());
            }
            IconEffect::Draw => {
                game_view.draw_card_into_hand()?;
            }
            IconEffect::Water => {
                game_view.game_state.gain_water();
            }
            IconEffect::GainPunk => {
                return Ok(game_view.gain_punk());
            }
            IconEffect::Raid => {
                return Ok(game_view.game_state.raid(game_view.player));
            }
        }
        Ok(ChoiceFuture::immediate(game_view.game_state))
    }
}
