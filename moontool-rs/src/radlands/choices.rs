use std::rc::Rc;

use super::locations::*;
use super::player_state::Person;
use super::{Action, GameResult, GameState, IconEffect};

/// A choice between several options that must be made by a player, along with the logic for
/// advancing the game state based on the choice.
#[must_use]
pub enum Choice<'ctype> {
    Action(ActionChoice<'ctype>),
    PlayLoc(PlayChoice<'ctype>),
    Damage(DamageChoice<'ctype>),
    Restore(RestoreChoice<'ctype>),
    IconEffect(Vec<IconEffect>), // only used for Scientist's ability
}

impl<'ctype> Choice<'ctype> {
    /// Returns a choice for top-level turn Actions for the current player.
    pub fn new_actions(game_state: &mut GameState<'ctype>) -> Choice<'ctype> {
        let view = game_state.view_for_cur();
        let actions = view.my_state().actions(&view);
        Choice::Action(ActionChoice { actions })
    }
}

type ThenCallback<'ctype, T> =
    Rc<dyn Fn(&mut GameState<'ctype>, T) -> Result<Choice<'ctype>, GameResult> + 'ctype>;

/// A future that may need to wait for a player to make a choice.
/// Can be converted into a full `Choice` by attaching a callback with `.then(...)`.
#[must_use]
pub struct ChoiceFuture<'g, 'ctype: 'g, T = ()> {
    choice_builder:
        Box<dyn FnOnce(ThenCallback<'ctype, T>) -> Result<Choice<'ctype>, GameResult> + 'g>,
}

impl<'g, 'ctype: 'g, T: 'ctype> ChoiceFuture<'g, 'ctype, T> {
    /// Returns a `Choice` that encapsulates the given logic for advancing the game state after
    /// this future resolves.
    pub fn then(
        self,
        callback: impl Fn(&mut GameState<'ctype>, T) -> Result<Choice<'ctype>, GameResult> + 'ctype,
    ) -> Result<Choice<'ctype>, GameResult> {
        (self.choice_builder)(Rc::new(callback))
    }

    /// Returns a new future that encapsulates the given logic for advancing the game state after
    /// this future resolves, but still needs more logic added to determine the next choice.
    pub fn then_future<U: 'ctype>(
        self,
        callback: impl Fn(&mut GameState<'ctype>, T) -> Result<U, GameResult> + 'ctype,
    ) -> ChoiceFuture<'g, 'ctype, U> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback2| {
                (self.choice_builder)(Rc::new(move |game_state, value| {
                    let value2 = callback(game_state, value)?;
                    callback2(game_state, value2)
                }))
            }),
        }
    }

    /// Returns a new future that encapsulates the given logic for advancing the game state after
    /// this future resolves, but still needs more logic added to determine the next choice.
    pub fn then_future_chain<U: 'ctype>(
        self,
        callback: impl for<'g2> Fn(
                &'g2 mut GameState<'ctype>,
                T,
            ) -> Result<ChoiceFuture<'g2, 'ctype, U>, GameResult>
            + 'ctype,
    ) -> ChoiceFuture<'g, 'ctype, U> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback2| {
                (self.choice_builder)(Rc::new(move |game_state, value| {
                    let future2 = callback(game_state, value)?;
                    (future2.choice_builder)(callback2.clone())
                }))
            }),
        }
    }

    /// Converts this future into one that has no extra result value.
    pub fn ignore_result(self) -> ChoiceFuture<'g, 'ctype> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback| {
                (self.choice_builder)(Rc::new(move |game_state, _| callback(game_state, ())))
            }),
        }
    }
}

impl<'g, 'ctype: 'g> ChoiceFuture<'g, 'ctype> {
    /// Returns a future that resolves immediately with no value using the given `GameState`.
    pub fn immediate(game_state: &'g mut GameState<'ctype>) -> ChoiceFuture<'g, 'ctype> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback| callback(game_state, ())),
        }
    }

    /// Returns a future that ends the game immediately with the given `GameResult`.
    pub fn end_game(game_result: GameResult) -> ChoiceFuture<'g, 'ctype> {
        ChoiceFuture {
            choice_builder: Box::new(move |_| Err(game_result)),
        }
    }
}

pub struct ActionChoice<'ctype> {
    actions: Vec<Action<'ctype>>,
}

impl<'g, 'ctype: 'g> ActionChoice<'ctype> {
    /// Returns the set of actions that can be taken by the current player.
    pub fn actions(&self) -> &[Action<'ctype>] {
        &self.actions
    }

    /// Chooses the given action, updating the game state and returning the next Choice.
    pub fn choose(
        &self,
        game_state: &'g mut GameState<'ctype>,
        action: &Action<'ctype>,
    ) -> Result<Choice<'ctype>, GameResult> {
        action.perform(game_state.view_for_cur())
    }
}

pub struct PlayChoice<'ctype> {
    /// The player who is playing the card.
    chooser: Player,
    /// The person who is being played.
    person: Person<'ctype>,
    /// The locations where the card can be played.
    locations: Vec<PlayLocation>,
    /// A callback for what to do after the player has chosen and the card has been played.
    then: Rc<dyn Fn(&mut GameState<'ctype>, ()) -> Result<Choice<'ctype>, GameResult> + 'ctype>,
}

impl<'g, 'ctype: 'g> PlayChoice<'ctype> {
    /// Returns the player who is playing the card.
    pub fn chooser(&self) -> Player {
        self.chooser
    }

    /// Returns the Person to be played.
    pub fn person(&self) -> &Person<'ctype> {
        &self.person
    }

    /// Returns the set of possible play locations.
    pub fn locations(&self) -> &[PlayLocation] {
        &self.locations
    }

    /// Creates a new future that asks the player to choose a play location.
    pub fn future(
        chooser: Player,
        person: Person<'ctype>,
        locations: Vec<PlayLocation>,
    ) -> ChoiceFuture<'g, 'ctype> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback| {
                Ok(Choice::PlayLoc(PlayChoice {
                    chooser,
                    person,
                    locations,
                    then: callback,
                }))
            }),
        }
    }

    /// Plays the person at the given location, updating the game state and
    /// returning the next Choice.
    pub fn choose(
        &self,
        game_state: &'g mut GameState<'ctype>,
        play_loc: PlayLocation,
    ) -> Result<Choice<'ctype>, GameResult> {
        let mut view = game_state.view_for(self.chooser);

        // place the card onto the board
        let col_index = play_loc.column().as_usize();
        let row_index = play_loc.row().as_usize();
        let col = &mut view.my_state_mut().columns[col_index];
        if let Some(old_person) = col.person_slots[row_index].replace(self.person.clone()) {
            // if there was a person already in the slot, move the old person to the other slot
            let other_row_index = 1 - row_index;
            let other_slot_old = col.person_slots[other_row_index].replace(old_person);
            assert!(other_slot_old.is_none()); // the other slot should have been empty
        }

        // activate any "when this card enters play" effect of the person
        if let Person::NonPunk { person_type, .. } = col.person_slots[row_index].as_ref().unwrap() {
            if let Some(on_enter_play) = person_type.on_enter_play {
                let future = on_enter_play(view, play_loc);
                return (future.choice_builder)(self.then.clone());
            }
        }

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}

pub struct DamageChoice<'ctype> {
    /// The player that must choose a card to damage.
    chooser: Player,
    /// Whether to destroy the card (versus just damaging it).
    destroy: bool,
    /// The locations of the cards that can be damaged.
    locations: Vec<CardLocation>,
    /// A callback for what to do after the player has chosen and the card has been damaged.
    then: Rc<
        dyn Fn(&mut GameState<'ctype>, CardLocation) -> Result<Choice<'ctype>, GameResult> + 'ctype,
    >,
}

impl<'g, 'ctype: 'g> DamageChoice<'ctype> {
    /// Returns the player who must choose a card to damage.
    pub fn chooser(&self) -> Player {
        self.chooser
    }

    /// Returns whether the chosen card will be destroyed instead of just damaged.
    pub fn destroy(&self) -> bool {
        self.destroy
    }

    /// Returns the set of possible locations to damage.
    pub fn locations(&self) -> &[CardLocation] {
        &self.locations
    }

    /// Creates a new future that asks the player to damage a card before resolving.
    pub fn future(
        chooser: Player,
        destroy: bool,
        locations: Vec<CardLocation>,
    ) -> ChoiceFuture<'g, 'ctype, CardLocation> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback| {
                Ok(Choice::Damage(DamageChoice {
                    chooser,
                    destroy,
                    locations,
                    then: callback,
                }))
            }),
        }
    }

    /// Chooses the given card to damage, updating the game state and returning the next Choice.
    pub fn choose(
        &self,
        game_state: &'g mut GameState<'ctype>,
        target_loc: CardLocation,
    ) -> Result<Choice<'ctype>, GameResult> {
        // damage the card
        game_state.damage_card_at(target_loc, self.destroy, true)?;

        // advance the game state until the next choice
        (self.then)(game_state, target_loc)
    }
}

pub struct RestoreChoice<'ctype> {
    /// The player that must choose a card to restore.
    chooser: Player,
    /// The locations of the cards that can be restored.
    locations: Vec<PlayerCardLocation>,
    /// A callback for what to do after the player has chosen and the card has been restored.
    then: Rc<dyn Fn(&mut GameState<'ctype>, ()) -> Result<Choice<'ctype>, GameResult> + 'ctype>,
}

impl<'g, 'ctype: 'g> RestoreChoice<'ctype> {
    /// Returns the player who must choose a card to restore.
    pub fn chooser(&self) -> Player {
        self.chooser
    }

    /// Returns the set of possible locations to restore.
    pub fn locations(&self) -> &[PlayerCardLocation] {
        &self.locations
    }

    /// Creates a new future that asks the player to damage a card before resolving.
    pub fn future(chooser: Player, locations: Vec<PlayerCardLocation>) -> ChoiceFuture<'g, 'ctype> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback| {
                Ok(Choice::Restore(RestoreChoice {
                    chooser,
                    locations,
                    then: callback,
                }))
            }),
        }
    }

    /// Chooses the given card to restore, updating the game state and returning the next Choice.
    pub fn choose(
        &self,
        game_state: &'g mut GameState<'ctype>,
        target_loc: PlayerCardLocation,
    ) -> Result<Choice<'ctype>, GameResult> {
        // restore the card
        game_state
            .player_mut(self.chooser)
            .restore_card_at(target_loc);

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}
