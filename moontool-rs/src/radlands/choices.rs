use std::sync::Arc;

use itertools::Itertools;
use tui::text::Spans;

use crate::make_spans;

use super::locations::*;
use super::player_state::Person;
use super::styles::StyledName;
use super::{Action, GameResult, GameState, IconEffect};

/// A choice between several options that must be made by a player, along with the logic for
/// advancing the game state based on the choice.
#[derive(Clone)]
#[must_use]
pub enum Choice<'ctype> {
    Action(ActionChoice<'ctype>),
    PlayLoc(PlayChoice<'ctype>),
    Damage(DamageChoice<'ctype>),
    Restore(RestoreChoice<'ctype>),
    IconEffect(IconEffectChoice<'ctype>), // only used for Scientist's ability
    RescuePerson(RescuePersonChoice<'ctype>), // only used for Rescue Team's ability
    MoveEvents(MoveEventsChoice<'ctype>), // only used for Doomsayer's on-enter-play effect
    DamageColumn(DamageColumnChoice<'ctype>), // only used for Magnus Karv's ability
}

impl<'v, 'g: 'v, 'ctype: 'g> Choice<'ctype> {
    /// Returns a choice for top-level turn Actions for the current player.
    pub fn new_actions(game_state: &mut GameState<'ctype>) -> Choice<'ctype> {
        let view = game_state.view_for_cur();
        let actions = view.my_state().actions(&view);
        Choice::Action(ActionChoice { actions })
    }

    /// Returns the number of options available for this choice.
    pub fn num_options(&self, game_state: &GameState<'ctype>) -> usize {
        match self {
            Choice::Action(action_choice) => action_choice.actions().len(),
            Choice::PlayLoc(play_choice) => play_choice.locations().len(),
            Choice::Damage(damage_choice) => damage_choice.locations().len(),
            Choice::Restore(restore_choice) => restore_choice.locations().len(),
            Choice::IconEffect(icon_effect_choice) => icon_effect_choice.icon_effects().len() + 1,
            Choice::RescuePerson(rescue_person_choice) => game_state
                .player(rescue_person_choice.chooser())
                .people()
                .count(),
            Choice::MoveEvents(_move_events_choice) => 2,
            Choice::DamageColumn(damage_column_choice) => damage_column_choice.columns().len(),
        }
    }

    /// Returns which player must make the choice.
    pub fn chooser(&self, game_state: &GameState<'ctype>) -> Player {
        match self {
            Choice::Action(_action_choice) => game_state.cur_player,
            Choice::PlayLoc(play_choice) => play_choice.chooser(),
            Choice::Damage(damage_choice) => damage_choice.chooser(),
            Choice::Restore(restore_choice) => restore_choice.chooser(),
            Choice::IconEffect(icon_effect_choice) => icon_effect_choice.chooser(),
            Choice::RescuePerson(rescue_person_choice) => rescue_person_choice.chooser(),
            Choice::MoveEvents(move_events_choice) => move_events_choice.chooser(),
            Choice::DamageColumn(damage_column_choice) => damage_column_choice.chooser(),
        }
    }

    /// Chooses the option with the given index, updating the game state and
    /// returning the next Choice.
    ///
    /// Panics if the index is is greater than equal to the number of options for this choice.
    pub fn choose(
        &self,
        game_state: &'g mut GameState<'ctype>,
        option: usize,
    ) -> Result<Choice<'ctype>, GameResult> {
        match self {
            Choice::Action(action_choice) => {
                action_choice.choose(game_state, &action_choice.actions()[option])
            }
            Choice::PlayLoc(play_choice) => {
                play_choice.choose(game_state, play_choice.locations()[option])
            }
            Choice::Damage(damage_choice) => {
                damage_choice.choose(game_state, damage_choice.locations()[option])
            }
            Choice::Restore(restore_choice) => {
                restore_choice.choose(game_state, restore_choice.locations()[option])
            }
            Choice::IconEffect(icon_effect_choice) => {
                let icon_effect = if option == 0 {
                    None
                } else {
                    Some(icon_effect_choice.icon_effects()[option - 1])
                };
                icon_effect_choice.choose(game_state, icon_effect)
            }
            Choice::RescuePerson(rescue_person_choice) => {
                let (location, _) = game_state
                    .player(rescue_person_choice.chooser())
                    .nth_person(option);
                rescue_person_choice.choose(game_state, location)
            }
            Choice::MoveEvents(move_events_choice) => move_events_choice.choose(
                game_state,
                match option {
                    0 => false,
                    1 => true,
                    _ => panic!("Invalid option for Choice::MoveEvents"),
                },
            ),
            Choice::DamageColumn(damage_column_choice) => {
                damage_column_choice.choose(game_state, damage_column_choice.columns()[option])
            }
        }
    }

    /// Formats the option with the given index for human-readable display.
    ///
    /// Panics if the index is is greater than equal to the number of options for this choice.
    pub fn format_option(
        &self,
        option: usize,
        game_state: &'g mut GameState<'ctype>,
    ) -> Spans<'ctype> {
        match self {
            Choice::Action(action_choice) => {
                action_choice.actions()[option].format(&game_state.view_for_cur())
            }
            Choice::PlayLoc(play_choice) => make_spans!(
                "Play ",
                play_choice.person().styled_name(),
                format!(" at {:?}", play_choice.locations()[option]),
            ),
            Choice::Damage(damage_choice) => Spans::from(format!(
                "{} {:?}",
                if damage_choice.destroy() { "Destroy" } else { "Damage" },
                damage_choice.locations()[option],
            )),
            Choice::Restore(restore_choice) => {
                Spans::from(format!("Restore {:?}", restore_choice.locations()[option]))
            }
            Choice::IconEffect(icon_effect_choice) => Spans::from({
                if option == 0 {
                    "Don't use an icon effect".to_string()
                } else {
                    format!(
                        "Use icon effect {:?}",
                        icon_effect_choice.icon_effects()[option - 1],
                    )
                }
            }),
            Choice::RescuePerson(rescue_person_choice) => {
                let (location, person) = game_state
                    .player(rescue_person_choice.chooser())
                    .nth_person(option);
                make_spans!(
                    "Rescue ",
                    person.styled_name(),
                    format!(" at {:?}", location),
                )
            }
            Choice::MoveEvents(_move_events_choice) => Spans::from(match option {
                0 => "Don't move events back",
                1 => "Move events back",
                _ => panic!("Invalid option for Choice::MoveEvents"),
            }),
            Choice::DamageColumn(damage_column_choice) => Spans::from(format!(
                "Damage column {:?}",
                damage_column_choice.columns()[option]
            )),
        }
    }
}

type ThenCallback<'ctype, T> = Arc<
    dyn Fn(&mut GameState<'ctype>, T) -> Result<Choice<'ctype>, GameResult> + Sync + Send + 'ctype,
>;

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
        callback: impl Fn(&mut GameState<'ctype>, T) -> Result<Choice<'ctype>, GameResult>
            + Sync
            + Send
            + 'ctype,
    ) -> Result<Choice<'ctype>, GameResult> {
        (self.choice_builder)(Arc::new(callback))
    }

    /// Returns a new future that encapsulates the given logic for advancing the game state after
    /// this future resolves, but still needs more logic added to determine the next choice.
    pub fn then_future<U: 'ctype>(
        self,
        callback: impl Fn(&mut GameState<'ctype>, T) -> Result<U, GameResult> + Sync + Send + 'ctype,
    ) -> ChoiceFuture<'g, 'ctype, U> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback2| {
                (self.choice_builder)(Arc::new(move |game_state, value| {
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
            + Sync
            + Send
            + 'ctype,
    ) -> ChoiceFuture<'g, 'ctype, U> {
        ChoiceFuture {
            choice_builder: Box::new(move |callback2| {
                (self.choice_builder)(Arc::new(move |game_state, value| {
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
                (self.choice_builder)(Arc::new(move |game_state, _| callback(game_state, ())))
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

#[derive(Clone)]
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

macro_rules! choice_struct {
    {
        #[doc = $choice_doc:literal]
        $VariantName:ident:
        pub struct $StructName:ident => $result_type:ty {
            $($(#[$field_meta:meta])* $field:ident: ($($field_type:tt)+),)*
        }

        $(#[$choose_meta:meta])*
        pub fn choose(&$self:ident, $game_state:ident, $action:ident: $action_type:ty $(,)?)
            $perform_action:block
    } => {
        #[derive(Clone)]
        pub struct $StructName<'ctype> {
            /// The player who must choose.
            chooser: Player,

            $($(#[$field_meta])* $field: $($field_type)+,)*

            /// A callback for what to do after the player chooses and the game state is updated.
            then: Arc<dyn Fn(&mut GameState<'ctype>, $result_type) -> Result<Choice<'ctype>, GameResult> + Sync + Send + 'ctype>,
        }

        impl<'g, 'ctype: 'g> $StructName<'ctype> {
            /// The player who must choose.
            pub fn chooser(&self) -> Player {
                self.chooser
            }

            $(
                $(#[$field_meta])*
                pub fn $field(&self) -> field_return_type!($($field_type)+) {
                    field_return_expr!(self, $field, $($field_type)+)
                }
            )*

            #[doc = "Creates a new future that"]
            #[doc = $choice_doc]
            #[doc = "before resolving."]
            pub fn future(
                chooser: Player,
                $($field: $($field_type)+,)*
            ) -> ChoiceFuture<'g, 'ctype, $result_type> {
                ChoiceFuture {
                    choice_builder: Box::new(move |callback| {
                        Ok(Choice::$VariantName($StructName {
                            chooser,
                            $($field,)*
                            then: callback,
                        }))
                    }),
                }
            }

            $(#[$choose_meta])*
            pub fn choose(
                &$self,
                $game_state: &'g mut GameState<'ctype>,
                $action: $action_type,
            ) -> Result<Choice<'ctype>, GameResult> {
                $perform_action
            }
        }
    };
}

macro_rules! field_return_type {
    (bool) => {
        bool
    };
    (Vec<$type:ty>) => {
        &[$type]
    };
    ($type:ty) => {
        &$type
    };
}

macro_rules! field_return_expr {
    ($self:ident, $field:ident, bool) => {
        $self.$field
    };
    ($self:ident, $field:ident, $type:ty) => {
        &$self.$field
    };
}

choice_struct! {
    /// asks the player to choose a play location
    PlayLoc:
    pub struct PlayChoice => () {
        /// The person who is being played.
        person: (Person<'ctype>),
        /// The locations where the card can be played.
        locations: (Vec<PlayLocation>),
    }

    /// Plays the person at the given location,
    /// updating the game state and returning the next Choice.
    pub fn choose(&self, game_state, play_loc: PlayLocation) {
        let mut view = game_state.view_for(self.chooser);

        // place the card onto the board
        let col = view.my_state_mut().column_mut(play_loc.column());
        let row_index = play_loc.row().as_usize();
        if let Some(old_person) = col.person_slots[row_index].replace(self.person.clone()) {
            // if there was a person already in the slot, move the old person to the other slot
            let other_row_index = 1 - row_index;
            let other_slot_old = col.person_slots[other_row_index].replace(old_person);
            assert!(other_slot_old.is_none()); // the other slot should have been empty
        }

        // activate any "when this card enters play" effect of the person
        if let Person::NonPunk { person_type, .. } = col.person_slots[row_index].as_ref().unwrap() {
            if let Some(on_enter_play) = person_type.on_enter_play {
                let future = on_enter_play(view, play_loc)?;
                return (future.choice_builder)(self.then.clone());
            }
        }

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}

choice_struct! {
    /// asks the player to damage a card
    Damage:
    pub struct DamageChoice => CardLocation {
        /// Whether to destroy the card (versus just damaging it).
        destroy: (bool),
        /// The locations of the cards that can be damaged.
        locations: (Vec<CardLocation>),
    }

    /// Chooses the given card to damage, updating the game state and returning the next Choice.
    pub fn choose(&self, game_state, target_loc: CardLocation) {
        // damage the card
        game_state.damage_card_at(target_loc, self.destroy, true)?;

        // advance the game state until the next choice
        (self.then)(game_state, target_loc)
    }
}

choice_struct! {
    /// asks the player to restore a card
    Restore:
    pub struct RestoreChoice => () {
        /// The locations of the cards that can be restored.
        locations: (Vec<PlayerCardLocation>),
    }

    /// Chooses the given card to restore, updating the game state and returning the next Choice.
    pub fn choose(&self, game_state, target_loc: PlayerCardLocation) {
        // restore the card
        game_state
            .player_mut(self.chooser)
            .restore_card_at(target_loc);

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}

choice_struct! {
    /// asks the player to perform an icon effect
    IconEffect:
    pub struct IconEffectChoice => () {
        /// The icon effects that can be performed.
        icon_effects: (Vec<IconEffect>),
    }

    /// Chooses the given icon effect to perform (or None), updating the game state
    /// and returning the next Choice.
    pub fn choose(&self, game_state, icon_effect: Option<IconEffect>) {
        if let Some(icon_effect) = icon_effect {
            // perform the icon effect
            let future = icon_effect.perform(game_state.view_for(self.chooser))?;
            (future.choice_builder)(self.then.clone())
        } else {
            // no icon effect was chosen, so just advance the game state until the next choice
            (self.then)(game_state, ())
        }
    }
}

choice_struct! {
    /// asks the player to rescue one of their people
    RescuePerson:
    pub struct RescuePersonChoice => () {}

    /// Chooses the given person to rescue, updating the game state
    /// and returning the next Choice.
    pub fn choose(&self, game_state, person_loc: PlayLocation) {
        let player_state = game_state.player_mut(self.chooser);

        // remove the person from the board
        let person = player_state.remove_person_at(person_loc);

        // add the card to the player's hand
        player_state.hand.add_one(person.card_type());

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}

choice_struct! {
    /// asks the player whether to move their opponent's events back 1
    MoveEvents:
    pub struct MoveEventsChoice => () {}

    /// Chooses whether to move the opponent's events back 1, updating the game state
    /// and returning the next Choice.
    pub fn choose(&self, game_state, move_events: bool) {
        if move_events {
            // move the events back 1
            game_state
                .player_mut(self.chooser.other())
                .move_events_back();
        }

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}

choice_struct! {
    /// asks the player to damage an opponent column
    DamageColumn:
    pub struct DamageColumnChoice => () {
        /// The columns that can be damaged.
        columns: (Vec<ColumnIndex>),
    }

    /// Chooses the given column to damage, updating the game state and returning the next Choice.
    pub fn choose(&self, game_state, column: ColumnIndex) {
        // damage all cards in the column
        let target_locs = game_state
            .player(self.chooser.other())
            .column(column)
            .card_rows()
            .map(|row| CardLocation::new(column, row, self.chooser.other()))
            .collect_vec();
        game_state.damage_cards_at(target_locs, false)?;

        // advance the game state until the next choice
        (self.then)(game_state, ())
    }
}
