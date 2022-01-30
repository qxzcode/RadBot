use crate::cards::Cards;

struct GameState<'ctype> {
    player1: Player<'ctype>,
    player2: Player<'ctype>,

    deck: Vec<&'ctype dyn PersonOrEventType>,

    /// Whether it is currently player 1's turn.
    is_player1_turn: bool,

    /// The amount of water that the current player has available for use.
    cur_player_water: u32,
}

impl<'ctype> GameState<'ctype> {
    pub fn do_turn(&mut self) {
        // let (cur_player, other_player) = if self.is_player1_turn {
        //     (&mut self.player1, &mut self.player2)
        // } else {
        //     (&mut self.player2, &mut self.player1)
        // };

        // resolve/advance events
        if let Some(event) = self.cur_player().state.events[0].take() {
            event.resolve(self);
        }
        self.cur_player().state.events.rotate_left(1);

        // replenish water
        self.cur_player_water = 3;
        if self.cur_player().state.has_water_silo {
            self.cur_player_water += 1;
        }

        // draw a card
        //...

        // perform actions
        //...
    }

    pub fn cur_player(&mut self) -> &mut Player<'ctype> {
        if self.is_player1_turn {
            &mut self.player1
        } else {
            &mut self.player2
        }
    }

    pub fn other_player(&mut self) -> &mut Player<'ctype> {
        if self.is_player1_turn {
            &mut self.player2
        } else {
            &mut self.player1
        }
    }
}

struct Player<'ctype> {
    state: PlayerState<'ctype>,
}

struct PlayerState<'ctype> {
    /// The cards in the player's hand, not including Water Silo.
    hand: Cards<'ctype>,

    /// When it is not this player's turn, whether this player has Water Silo
    /// in their hand. (They are assumed to not have it in their hand when it
    /// *is* this player's turn.)
    has_water_silo: bool,

    /// The three columns of the player's board.
    columns: [CardColumn<'ctype>; 3],

    /// The three event slots of the player's board.
    events: [Option<&'ctype dyn EventType>; 3],
}

struct CardColumn<'ctype> {
    /// The column's camp.
    camp: Camp<'ctype>,

    /// The people in the column.
    /// Maximum size is 2; the last element is the unprotected one.
    people: Vec<Person<'ctype>>, // TODO: use an array-backed collection?
}

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

/// A person played on the board (punk or other person).
enum Person<'ctype> {
    Punk,
    NonPunk(NonPunk<'ctype>),
}

/// A non-punk person played on the board.
struct NonPunk<'ctype> {
    person_type: &'ctype dyn PersonType,
    is_injured: bool,
}

/// A type of camp card.
struct CampType {
    /// The camp's name.
    name: &'static str,

    /// The number of cards this camp grants at the start of the game.
    num_initial_cards: u8,
}

/// Supertrait for playable cards (people or events).
trait PersonOrEventType {
    /// Returns the card's name.
    fn name(&self) -> &'static str;

    /// Returns how many of this person type are in the deck.
    fn num_in_deck(&self) -> u8;

    /// Returns the card's junk action.
    fn junk_action(&self) -> JunkAction;

    /// Returns the water cost to play this card.
    fn water_cost(&self) -> u8;
}

/// Trait for a type of person card.
trait PersonType: PersonOrEventType {
    //...
}

/// Trait for a type of event card.
trait EventType: PersonOrEventType {
    /// Returns the number of turns before the event resolves after being played.
    fn resolve_turns(&self) -> u8;

    /// Resolves the event.
    fn resolve(&self, game_state: &mut GameState);
}

/// Enum representing possible actions when junking a card.
pub enum JunkAction {
    Injure,
    Restore,
    Draw,
    Water,
    GainPunk,
    Raid,
}
