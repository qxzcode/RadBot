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
use self::controllers::PlayerController;
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

    pub fn do_turn(
        &'g mut self,
        p1_controller: &dyn PlayerController,
        p2_controller: &dyn PlayerController,
        is_first_turn: bool,
    ) -> Result<(), GameResult> {
        // get a view of the game state for the current player
        let mut cur_view = match self.cur_player {
            Player::Player1 => GameView::new(self, self.cur_player, p1_controller, p2_controller),
            Player::Player2 => GameView::new(self, self.cur_player, p2_controller, p1_controller),
        };

        cur_view.do_turn(is_first_turn)
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

/// A view of a game from one player's perspective.
pub struct GameView<'g, 'ctype: 'g> {
    /// The game state.
    game_state: &'g mut GameState<'ctype>,

    /// The identity of the player whose view this is for.
    player: Player,

    /// The controller for the player whose view this is for.
    my_controller: &'g dyn PlayerController,

    /// The controller for the other player.
    other_controller: &'g dyn PlayerController,
}

impl<'v, 'g: 'v, 'ctype: 'g> GameView<'g, 'ctype> {
    pub fn new(
        game_state: &'g mut GameState<'ctype>,
        player: Player,
        my_controller: &'g dyn PlayerController,
        other_controller: &'g dyn PlayerController,
    ) -> Self {
        GameView {
            game_state,
            player,
            my_controller,
            other_controller,
        }
    }

    pub fn my_state(&self) -> &PlayerState<'ctype> {
        self.game_state.player(self.player)
    }

    pub fn my_state_mut(&mut self) -> &mut PlayerState<'ctype> {
        self.game_state.player_mut(self.player)
    }

    pub fn other_state(&self) -> &PlayerState<'ctype> {
        self.game_state.player(self.player.other())
    }

    pub fn other_state_mut(&mut self) -> &mut PlayerState<'ctype> {
        self.game_state.player_mut(self.player.other())
    }

    pub fn other_view_mut(&'v mut self) -> GameView<'v, 'ctype> {
        GameView::new(
            self.game_state,
            self.player.other(),
            self.other_controller,
            self.my_controller,
        )
    }

    fn do_turn(&'v mut self, is_first_turn: bool) -> Result<(), GameResult> {
        // resolve/advance events
        if let Some(event) = self.my_state_mut().events[0].take() {
            // resolve the first event
            event.resolve(self)?;

            // discard it if it's not Raiders
            if event.as_raiders().is_none() {
                self.game_state
                    .discard
                    .push(PersonOrEventType::Event(event));
            }
        }
        self.my_state_mut().events.rotate_left(1);

        // replenish water
        self.game_state.cur_player_water = if is_first_turn { 1 } else { 3 };
        if self.my_state().has_water_silo {
            self.game_state.cur_player_water += 1;
            self.my_state_mut().has_water_silo = false;
        }

        // reset other turn state
        self.game_state.has_paid_to_draw = false;

        // draw a card
        self.draw_card_into_hand()?;

        // perform actions
        loop {
            // get all the possible actions
            let actions = self.my_state().actions(self);

            // ask the player what to do
            let action = self.my_controller.choose_action(self, &actions);

            // perform the action
            if action.perform(self)? {
                break;
            }

            // check for win condition
            //...
        }

        // finally, switch whose turn it is
        self.game_state.cur_player = self.game_state.cur_player.other();

        Ok(())
    }

    /// Has this player damage an unprotected opponent card.
    pub fn damage_enemy(&mut self) -> Result<(), GameResult> {
        // get all possible targets
        let target_player = self.player.other();
        let target_locs = self
            .other_state()
            .unprotected_card_locs()
            .map(|loc| loc.for_player(target_player))
            .collect_vec();

        // ask the player to damage one of them
        self.choose_and_damage_card(&target_locs)
    }

    /// Has this player injure an unprotected opponent person.
    /// Assumes that the opponent has at least one person.
    pub fn injure_enemy(&mut self) {
        // get all possible targets
        let target_player = self.player.other();
        let target_locs = self
            .other_state()
            .unprotected_person_locs()
            .map(|loc| loc.for_player(target_player))
            .collect_vec();

        // ask the player to injure one of them
        self.choose_and_damage_card(&target_locs)
            .expect("injure_enemy should not end the game");
    }

    /// Has this player choose and then damage a card from a given list of locations.
    pub fn choose_and_damage_card(&mut self, locs: &[CardLocation]) -> Result<(), GameResult> {
        // ask the player which one to damage
        let target_loc = self.my_controller.choose_card_to_damage(self, locs);

        // damage the card
        self.game_state.damage_card_at(target_loc)
    }

    /// Has this player restore one of their own damaged cards.
    /// Assumes that the player has at least one restorable card.
    pub fn restore_card(&mut self) {
        // get all possible targets
        let target_locs = self.my_state().restorable_card_locs().collect_vec();

        // ask the player which one to restore
        let target_loc = self
            .my_controller
            .choose_card_to_restore(self, &target_locs);

        // restore the card
        self.my_state_mut().restore_card_at(target_loc);
    }

    /// Draws a card from the deck and puts it in this player's hand.
    pub fn draw_card_into_hand(&'v mut self) -> Result<(), GameResult> {
        let card = self.game_state.draw_card()?;
        self.my_state_mut().hand.add_one(card);
        Ok(())
    }

    /// Plays or advances this player's Raiders event.
    pub fn raid(&'v mut self) -> Result<(), GameResult> {
        // search for the Raiders event in the event queue
        for i in 0..self.my_state().events.len() {
            if let Some(event) = self.my_state().events[i] {
                if let Some(raiders) = event.as_raiders() {
                    // found the raiders event
                    if i == 0 {
                        // it's the first event, so resolve and remove it
                        raiders.resolve(self)?;
                        self.my_state_mut().events[0] = None;
                    } else {
                        // it's not the first event, so advance it if possible
                        let events = &mut self.my_state_mut().events;
                        if events[i - 1].is_none() {
                            events[i - 1] = events[i].take();
                        }
                    }
                    return Ok(());
                }
            }
        }

        // if we get here, the raiders event was not found in the event queue;
        // add it to the queue
        self.play_event(&RaidersEvent);
        Ok(())
    }

    /// Plays an event into this player's event queue.
    /// Panics if there is not a free slot for the event.
    fn play_event(&'v mut self, event: &'ctype dyn EventType) {
        let slot_index = (event.resolve_turns() - 1) as usize;
        let free_slot = self.my_state_mut().events[slot_index..]
            .iter_mut()
            .find(|slot| slot.is_none())
            .expect("Tried to play an event, but there was no free slot");
        *free_slot = Some(event);
    }

    /// Has this player add a punk to their board.
    /// Does nothing if the player's board is full.
    pub fn gain_punk(&mut self) -> Result<(), GameResult> {
        if self.my_state().has_empty_person_slot() {
            let punk = Person::Punk(self.game_state.draw_card()?);
            self.play_person(punk);
        }
        Ok(())
    }

    /// Asks this player's controller to choose a location, then plays the given person
    /// onto that location.
    fn play_person(&'v mut self, person: Person<'ctype>) {
        // determine possible locations to place the card
        let mut play_locs = Vec::new();
        for (col_index, col) in self.my_state().enumerate_columns() {
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
        let play_loc = self
            .my_controller
            .choose_play_location(self, &person, &play_locs);

        // place the card onto the board
        let col_index = play_loc.column().as_usize();
        let row_index = play_loc.row().as_usize();
        let col = &mut self.my_state_mut().columns[col_index];
        if let Some(old_person) = col.person_slots[row_index].replace(person) {
            // if there was a person in the slot, move it to the other slot
            let other_row_index = 1 - row_index;
            let replaced_slot = col.person_slots[other_row_index].replace(old_person);
            assert!(replaced_slot.is_none());
        }

        // TODO: activate any on-play effect of the person
    }
}

/// An action that can be performed by a player during their turn.
pub enum Action<'ctype> {
    /// Play a person card from the hand onto the board.
    PlayPerson(&'ctype PersonType),

    /// Play an event card from the hand onto the event queue.
    PlayEvent(&'ctype dyn EventType),

    /// Draw a card (costs 2 water).
    DrawCard,

    /// Junk a card from the hand to use its junk effect.
    JunkCard(PersonOrEventType<'ctype>),

    /// Use an ability of a ready person or camp.
    UseAbility(/*TODO*/),

    /// End the current player's turn, taking Water Silo if possible.
    EndTurn,
}

impl<'v, 'g: 'v, 'ctype: 'g> Action<'ctype> {
    /// Performs the action on the given game view.
    /// Returns whether the player's turn should end after this action.
    fn perform(&self, game_view: &'v mut GameView<'g, 'ctype>) -> Result<bool, GameResult> {
        match *self {
            Action::PlayPerson(person_type) => {
                // pay the person's cost and remove it from the player's hand
                game_view.game_state.spend_water(person_type.cost);
                game_view.my_state_mut().hand.remove_one(PersonOrEventType::Person(person_type));

                // play the person onto the board
                game_view.play_person(Person::new_non_punk(person_type));

                Ok(false)
            }
            Action::PlayEvent(event_type) => {
                // pay the event's cost and remove it from the player's hand
                game_view.game_state.spend_water(event_type.cost());
                game_view.my_state_mut().hand.remove_one(PersonOrEventType::Event(event_type));

                // add the event to the event queue
                game_view.play_event(event_type);

                Ok(false)
            }
            Action::DrawCard => {
                game_view.game_state.spend_water(2);
                game_view.draw_card_into_hand()?;
                game_view.game_state.has_paid_to_draw = true;
                Ok(false)
            },
            Action::JunkCard(card) => {
                // move the card to the discard pile
                game_view.my_state_mut().hand.remove_one(card);
                game_view.game_state.discard.push(card);

                // perform the card's junk effect
                card.junk_effect().perform(game_view)?;

                Ok(false)
            },
            Action::UseAbility(/*TODO*/) => {
                todo!("perform Action::UseAbility");
                Ok(false)
            },
            Action::EndTurn => {
                // take Water Silo if possible, then end the turn
                game_view.my_state_mut().has_water_silo = game_view.game_state.cur_player_water >= 1;
                Ok(true)
            },
        }
    }
}

impl fmt::Display for Action<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Action::PlayPerson(card) => write!(f, "Play {} (costs {WATER}{} water{RESET})", card.styled_name(), card.cost),
            Action::PlayEvent(card) => write!(f, "Play {} (costs {WATER}{} water{RESET})", card.styled_name(), card.cost()),
            Action::DrawCard => write!(f, "Draw a card (costs {WATER}2 water{RESET})"),
            Action::JunkCard(card) => write!(f, "Junk {} (effect: {:?})", card.styled_name(), card.junk_effect()),
            Action::UseAbility(/*TODO*/) => write!(f, "Use ability: [TODO]"),
            Action::EndTurn => write!(f, "End turn, taking {WATER}Water Silo{RESET} if possible"),
        }
    }
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

    /// Resolves the event. Takes a view from the perspective of this event's owner.
    fn resolve<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v mut GameView<'g, 'ctype>,
    ) -> Result<(), GameResult>;

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

    fn resolve<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v mut GameView<'g, 'ctype>,
    ) -> Result<(), GameResult> {
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
        game_view
            .other_view_mut()
            .choose_and_damage_card(&target_locs)
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
    /// Returns whether this icon effect can be performed given a game view.
    pub fn can_perform(&self, game_view: &GameView) -> bool {
        match self {
            IconEffect::Damage => true, // if there's nothing to damage, the game is over!
            IconEffect::Injure => game_view.other_state().people().next().is_some(),
            IconEffect::Restore => game_view.my_state().has_restorable_card(),
            IconEffect::Draw => true, // it's always possible to draw a card
            IconEffect::Water => true, // it's always possible to gain water
            IconEffect::GainPunk => game_view.my_state().has_empty_person_slot(),
            IconEffect::Raid => game_view.my_state().can_raid(),
        }
    }

    /// Performs the effect for the current player.
    pub fn perform(&self, game_view: &mut GameView) -> Result<(), GameResult> {
        match *self {
            IconEffect::Damage => {
                game_view.damage_enemy()?;
            }
            IconEffect::Injure => {
                game_view.injure_enemy();
            }
            IconEffect::Restore => {
                game_view.restore_card();
            }
            IconEffect::Draw => {
                game_view.draw_card_into_hand()?;
            }
            IconEffect::Water => {
                game_view.game_state.gain_water();
            }
            IconEffect::GainPunk => {
                game_view.gain_punk()?;
            }
            IconEffect::Raid => {
                game_view.raid()?;
            }
        }
        Ok(())
    }
}
