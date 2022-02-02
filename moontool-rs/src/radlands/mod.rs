pub mod camps;
pub mod people;

use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

use crate::cards::Cards;

pub struct GameState<'ctype> {
    player1: Player<'ctype>,
    player2: Player<'ctype>,

    deck: Vec<&'ctype (dyn PersonOrEventType + 'ctype)>,
    discard: Vec<&'ctype (dyn PersonOrEventType + 'ctype)>,

    /// Whether it is currently player 1's turn.
    is_player1_turn: bool,

    /// The amount of water that the current player has available for use.
    cur_player_water: u32,
}

impl<'g, 'ctype: 'g> GameState<'ctype> {
    pub fn new(
        camp_types: &'ctype [CampType],
        person_types: &'ctype [PersonType],
        player1: Box<dyn PlayerController>,
        player2: Box<dyn PlayerController>,
    ) -> Self {
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
            player1: Player::new(player1, p1_camps, &mut deck),
            player2: Player::new(player2, p2_camps, &mut deck),
            deck,
            discard: Vec::new(),
            is_player1_turn: thread_rng().gen(), // randomly pick which player goes first
            cur_player_water: 0,
        }
    }

    pub fn do_turn(&'g mut self, is_first_turn: bool) {
        // resolve/advance events
        if let Some(event) = self.cur_player_mut().state.events[0].take() {
            event.resolve(self);
        }
        self.cur_player_mut().state.events.rotate_left(1);

        // replenish water
        self.cur_player_water = if is_first_turn { 1 } else { 3 };
        if self.cur_player_mut().state.has_water_silo {
            self.cur_player_water += 1;
            self.cur_player_mut().state.has_water_silo = false;
        }

        // draw a card
        self.draw_card();

        // perform actions
        loop {
            // get all the possible actions
            let actions = self.cur_player().state.actions(self);

            // ask the player what to do
            let action = self.cur_player_mut().controller.choose_action(&actions);

            // perform the action
            if action.perform(self) {
                break;
            }

            // check for win condition
            //...
        }

        // finally, switch whose turn it is
        self.is_player1_turn = !self.is_player1_turn;
    }

    fn perform_action(action: Action<'ctype>, game_state: &'g mut GameState) {
        todo!();
    }

    /// Draws a card from the deck and puts it in the current player's hand.
    pub fn draw_card(&'g mut self) {
        if self.deck.is_empty() {
            // TODO: reshuffle from discard pile, and check for tie condition
            todo!();
        }
        let card = self.deck.pop().unwrap();
        self.cur_player_mut().state.hand.add_one(card);
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
        for i in 0..self.cur_player_mut().state.events.len() {
            if let Some(event) = self.cur_player_mut().state.events[i] {
                if let Some(raiders) = event.as_raiders() {
                    // found the raiders event in the event queue
                    if i == 0 {
                        // it's the first event, so resolve it
                        raiders.resolve(self);
                    } else {
                        // it's not the first event, so advance it if possible
                        let events = &mut self.cur_player_mut().state.events;
                        if events[i - 1].is_none() {
                            events[i - 1] = events[i].take();
                        }
                    }
                }
            }
        }
    }

    pub fn cur_player_mut(&'g mut self) -> &'g mut Player<'ctype> {
        if self.is_player1_turn {
            &mut self.player1
        } else {
            &mut self.player2
        }
    }

    pub fn cur_player(&'g self) -> &'g Player<'ctype> {
        if self.is_player1_turn {
            &self.player1
        } else {
            &self.player2
        }
    }

    pub fn other_player_mut(&'g mut self) -> &'g mut Player<'ctype> {
        if self.is_player1_turn {
            &mut self.player2
        } else {
            &mut self.player1
        }
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
    pub fn perform(self, game_state: &'g mut GameState<'ctype>) -> bool {
        match self {
            Action::PlayCard(card) => {
                // pay the card's cost and remove it from the player's hand
                game_state.spend_water(card.cost());
                game_state.cur_player_mut().state.hand.remove_one(card);

                // determine where to place the card
                // TODO
                // let person = Person::new_non_punk(card);
                // game_state.cur_player().state.columns[0].people.push(person);
                false
            },
            Action::DrawCard => {
                game_state.spend_water(2);
                game_state.draw_card();
                false
            },
            Action::JunkCard(card) => {
                // move the card to the discard pile
                game_state.cur_player_mut().state.hand.remove_one(card);
                game_state.discard.push(card);

                // perform the card's junk effect
                card.junk_effect().perform(game_state);

                false
            },
            Action::UseAbility(/*TODO*/) => {
                todo!();
                false
            },
            Action::EndTurn => {
                game_state.cur_player_mut().state.has_water_silo = game_state.cur_player_water >= 1;
                true
            },
        }
    }
}

/// Represents a player with a controller and game state.
pub struct Player<'ctype> {
    /// The controller that chooses actions for this player.
    controller: Box<dyn PlayerController>,

    /// The state specific to this player.
    state: PlayerState<'ctype>,
}

impl<'ctype> Player<'ctype> {
    /// Creates a new `Player` with the given camps, drawing an initial hand
    /// from the given deck.
    pub fn new(
        controller: Box<dyn PlayerController>,
        camps: &[&'ctype CampType],
        deck: &mut Vec<&'ctype (dyn PersonOrEventType + 'ctype)>,
    ) -> Self {
        Player {
            controller,
            state: PlayerState::new(camps, deck),
        }
    }
}

pub trait PlayerController {
    fn choose_action<'ctype>(&mut self, actions: &[Action<'ctype>]) -> Action<'ctype>;
}

/// Represents the state of a player's board and hand.
struct PlayerState<'ctype> {
    /// The cards in the player's hand, not including Water Silo.
    hand: Cards<'ctype, dyn PersonOrEventType + 'ctype>,

    /// When it is not this player's turn, whether this player has Water Silo
    /// in their hand. (They are assumed to not have it in their hand when it
    /// *is* this player's turn.)
    has_water_silo: bool,

    /// The three columns of the player's board.
    columns: [CardColumn<'ctype>; 3],

    /// The three event slots of the player's board.
    events: [Option<&'ctype (dyn EventType + 'ctype)>; 3],
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

    pub fn actions(&self, game: &'g GameState<'ctype>) -> Vec<Action<'ctype>> {
        let mut actions = Vec::new();

        // actions to play or junk a card
        for card_type in self.hand.iter_unique() {
            if game.cur_player_water >= card_type.cost() {
                actions.push(Action::PlayCard(card_type));
            }
            actions.push(Action::JunkCard(card_type));
        }

        // action to pay 2 water to draw a card
        if game.cur_player_water >= 2 {
            actions.push(Action::DrawCard);
        }

        // actions to use an ability
        for person in self.columns[0].people() {
            match person {
                Person::Punk(_) => {
                    // punks don't have abilities
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
}

struct CardColumn<'ctype> {
    /// The column's camp.
    camp: Camp<'ctype>,

    /// The people in the column.
    /// Maximum size is 2; the last element is the unprotected one.
    people: [Option<Person<'ctype>>; 2],
}

impl<'ctype> CardColumn<'ctype> {
    /// Creates a new column with the given camp.
    pub fn new(camp_type: &'ctype CampType) -> Self {
        CardColumn {
            camp: Camp {
                camp_type,
                status: CampStatus::Undamaged,
            },
            people: [None, None],
        }
    }

    /// Returns an iterator over the people in the column.
    pub fn people(&self) -> impl Iterator<Item = &Person<'ctype>> {
        self.people.iter().filter_map(|person| person.as_ref())
    }
}

/// A camp on the board.
struct Camp<'ctype> {
    /// The camp type.
    camp_type: &'ctype CampType,

    /// The damage status of the camp.
    status: CampStatus,
}

/// Enum representing the damage status of a camp.
enum CampStatus {
    Undamaged,
    Damaged,
    Destroyed,
}

/// A person played on the board (a punk or face-up person).
enum Person<'ctype> {
    Punk(&'ctype PersonType),
    NonPunk(NonPunk<'ctype>),
}

impl<'ctype> Person<'ctype> {
    /// Creates a fresh person from a person type.
    fn new_non_punk(person_type: &'ctype PersonType) -> Person<'ctype> {
        Person::NonPunk(NonPunk {
            person_type,
            is_injured: false,
        })
    }
}

/// A non-punk (face-up) person played on the board.
struct NonPunk<'ctype> {
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
pub trait PersonOrEventType {
    /// Returns the card's name.
    fn name(&self) -> &'static str;

    /// Returns how many of this person type are in the deck.
    fn num_in_deck(&self) -> u32;

    /// Returns the card's junk effect.
    fn junk_effect(&self) -> IconEffect;

    /// Returns the water cost to play this card.
    fn cost(&self) -> u32;
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
}

/// Trait for a type of event card.
trait EventType: PersonOrEventType {
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

struct RaidersEvent;

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
    pub fn perform<'g, 'ctype: 'g>(&self, game_state: &'g mut GameState<'ctype>) {
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
                game_state.draw_card();
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
    }
}
