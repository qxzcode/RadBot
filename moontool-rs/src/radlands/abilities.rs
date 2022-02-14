use super::locations::CardLocation;
use super::{GameResult, GameView, IconEffect};

/// An ability on a camp or person.
pub trait Ability {
    /// Returns a description of this ability for display.
    fn description(&self) -> String;

    /// Returns the water cost of this ability.
    fn cost<'v, 'g: 'v, 'ctype: 'g>(&self, game_view: &'v GameView<'g, 'ctype>) -> u32;

    /// Returns whether this ability can be used given the game state.
    /// Does not need to check for the water cost.
    fn can_perform<'v, 'g: 'v, 'ctype: 'g>(&self, game_view: &'v GameView<'g, 'ctype>) -> bool;

    /// Performs this ability.
    fn perform<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v mut GameView<'g, 'ctype>,
        card_loc: CardLocation,
    ) -> Result<(), GameResult>;

    /// Returns whether this ability can be afforded and used given the game state.
    fn can_afford_and_perform<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
    ) -> bool {
        game_view.game_state.cur_player_water >= self.cost(game_view) && self.can_perform(game_view)
    }
}

/// An ability that performs an IconEffect.
struct IconAbility {
    cost: u32,
    effect: IconEffect,
}

impl Ability for IconAbility {
    fn description(&self) -> String {
        format!("{:?}", self.effect)
    }

    fn cost<'v, 'g: 'v, 'ctype: 'g>(&self, _game_view: &'v GameView<'g, 'ctype>) -> u32 {
        self.cost
    }

    fn can_perform<'v, 'g: 'v, 'ctype: 'g>(&self, game_view: &'v GameView<'g, 'ctype>) -> bool {
        self.effect.can_perform(game_view)
    }

    fn perform<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v mut GameView<'g, 'ctype>,
        _card_loc: CardLocation,
    ) -> Result<(), GameResult> {
        self.effect.perform(game_view)
    }
}

/// Creates an ability that performs an IconEffect.
pub fn icon_ability(cost: u32, effect: IconEffect) -> Box<dyn Ability> {
    Box::new(IconAbility { cost, effect })
}

/// Macro for easily creating custom abilities.
macro_rules! ability {
    {
        description => $description:literal;
        cost => $cost:expr;
        can_perform($game_view_1:ident) => $can_perform:expr;
        perform($game_view_2:ident, $card_loc:ident) => $perform:expr;
    } => {{
        use $crate::radlands::{GameView, GameResult};
        use $crate::radlands::locations::CardLocation;
        use std::string::String;
        use std::result::Result;
        struct MacroAbility;
        impl $crate::abilities::Ability for MacroAbility {
            fn description(&self) -> String {
                $description.to_string()
            }

            fn cost<'v, 'g: 'v, 'ctype: 'g>(&self, _game_view: &'v GameView<'g, 'ctype>) -> u32 {
                $cost
            }

            fn can_perform<'v, 'g: 'v, 'ctype: 'g>(
                &self,
                $game_view_1: &'v GameView<'g, 'ctype>,
            ) -> bool {
                $can_perform
            }

            fn perform<'v, 'g: 'v, 'ctype: 'g>(
                &self,
                $game_view_2: &'v mut GameView<'g, 'ctype>,
                $card_loc: CardLocation,
            ) -> Result<(), GameResult> {
                $perform
            }
        }
        std::boxed::Box::new(MacroAbility)
    }};

    // version where can_perform is always true
    {
        description => $description:literal;
        cost => $cost:expr;
        can_perform => true;
        perform($game_view_2:ident, $card_loc:ident) => $perform:expr;
    } => {
        ability! {
            description => $description;
            cost => $cost;
            can_perform(_game_view) => true;
            perform($game_view_2, $card_loc) => $perform;
        }
    };

    // version without card_loc parameter for perform(...)
    {
        description => $description:literal;
        cost => $cost:expr;
        can_perform($game_view_1:ident) => $can_perform:expr;
        perform($game_view_2:ident) => $perform:expr;
    } => {
        ability! {
            description => $description;
            cost => $cost;
            can_perform($game_view_1) => $can_perform;
            perform($game_view_2, _card_loc) => $perform;
        }
    };

    // version without card_loc that performs an IconEffect
    {
        description => $description:literal;
        cost => $cost:expr;
        can_perform($game_view_1:ident) => $can_perform:expr;
        perform => IconEffect::$perform_effect:ident;
    } => {
        ability! {
            description => $description;
            cost => $cost;
            can_perform($game_view_1) => $can_perform;
            perform(game_view) => IconEffect::$perform_effect.perform(game_view);
        }
    };

    // version without card_loc where can_perform is always true
    {
        description => $description:literal;
        cost => $cost:expr;
        can_perform => true;
        perform($game_view_2:ident) => $perform:expr;
    } => {
        ability! {
            description => $description;
            cost => $cost;
            can_perform(_game_view) => true;
            perform($game_view_2) => $perform;
        }
    };
}

pub(crate) use ability;
