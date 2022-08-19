use std::collections::BTreeMap;

use crate::cards::Cards;

use super::choices::Choice;
use super::events::EventType;
use super::locations::Player;
use super::player_state::CardColumn;
use super::{GameState, PersonOrEventType};

/// A hashable multiset of cards.
#[derive(Clone, PartialEq, Eq, Hash, Default)]
struct HashableCards<'ctype> {
    cards: BTreeMap<PersonOrEventType<'ctype>, usize>,
}

impl<'iter, 'ctype: 'iter, I> From<I> for HashableCards<'ctype>
where
    I: IntoIterator<Item = &'iter PersonOrEventType<'ctype>>,
{
    fn from(iterable: I) -> Self {
        let mut cards = BTreeMap::new();
        for card in iterable {
            cards
                .entry(*card)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        HashableCards { cards }
    }
}

impl<'ctype> From<&Cards<PersonOrEventType<'ctype>>> for HashableCards<'ctype> {
    fn from(cards: &Cards<PersonOrEventType<'ctype>>) -> Self {
        HashableCards {
            cards: BTreeMap::from_iter(cards.iter()),
        }
    }
}

/// Stores the game state observed by a single player.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObservedStateFull<'ctype> {
    undrawn_cards: HashableCards<'ctype>,
    discard: HashableCards<'ctype>,

    /// The cards I have in my hand.
    my_hand: HashableCards<'ctype>,
    /// The cards I know my opponent has in their hand.
    opponent_hand_known: HashableCards<'ctype>,
    /// The number of cards in my opponent's hand whose identity is unknown to me.
    opponent_hand_unknown_count: usize,

    my_columns: [CardColumn<'ctype>; 3],
    my_events: [Option<&'ctype EventType>; 3],
    opponent_columns: [CardColumn<'ctype>; 3],
    opponent_events: [Option<&'ctype EventType>; 3],

    cur_player: Player,
    cur_player_water: u32,
    other_player_has_water_silo: bool,
    has_paid_to_draw: bool,
    has_played_event: bool,
    has_reshuffled_deck: bool,
    // TODO: Does this struct need to include the current choice too?
    // I think it just needs to uniquely identify nodes in the game search tree.
    // edit: YES, it needs to include some info about the current choice.
    //       For example, which ability was just selected on Rabble Rouser / Mimic?
    //       Asserting that option counts match should catch issues like this.
    choice_type: std::mem::Discriminant<Choice<'ctype>>,
    num_options: usize,
}

impl<'ctype> ObservedStateFull<'ctype> {
    /// Creates a new `ObservedState` from the given game state.
    pub fn from_game_state(
        game_state: &GameState<'ctype>,
        choice: &Choice<'ctype>,
        player: Player,
    ) -> Self {
        ObservedStateFull {
            undrawn_cards: (&game_state.deck).into(),
            discard: (&game_state.discard).into(),
            my_hand: (&game_state.player(player).hand).into(),
            opponent_hand_known: HashableCards::default(), // TODO: track known cards
            opponent_hand_unknown_count: game_state.player(player.other()).hand.count(),
            my_columns: game_state.player(player).columns.clone(),
            my_events: game_state.player(player).events,
            opponent_columns: game_state.player(player.other()).columns.clone(),
            opponent_events: game_state.player(player.other()).events,
            cur_player: game_state.cur_player,
            cur_player_water: game_state.cur_player_water,
            other_player_has_water_silo: game_state
                .player(game_state.cur_player.other())
                .has_water_silo,
            has_paid_to_draw: game_state.has_paid_to_draw,
            has_played_event: game_state.has_played_event,
            has_reshuffled_deck: game_state.has_reshuffled_deck,
            choice_type: std::mem::discriminant(choice),
            num_options: choice.num_options(game_state),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObservedState<'ctype>(u64, std::marker::PhantomData<&'ctype ()>);

impl<'ctype> ObservedState<'ctype> {
    /// Creates a new `ObservedState` from the given game state.
    pub fn from_game_state(
        game_state: &GameState<'ctype>,
        choice: &Choice<'ctype>,
        player: Player,
    ) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ObservedStateFull::from_game_state(game_state, choice, player).hash(&mut hasher);
        ObservedState(hasher.finish(), std::marker::PhantomData)
    }
}
