pub mod human;
pub mod monte_carlo;
pub mod random;

use super::*;
use crate::choices::*;

macro_rules! choose_function {
    ($name:ident(..., $ChoiceType:ty) -> $ReturnType:ty) => {
        fn $name<'a, 'v, 'g: 'v, 'ctype: 'g>(
            &self,
            game_view: &'v GameView<'g, 'ctype>,
            choice: &'a $ChoiceType,
        ) -> $ReturnType;
    };
}

/// Trait for a player controller.
/// All functions take a GameView for the player that this controller is responsible for.
pub trait PlayerController {
    choose_function!( choose_action(..., ActionChoice<'ctype>) -> &'a Action<'ctype> );
    choose_function!( choose_play_location(..., PlayChoice<'ctype>) -> PlayLocation );
    choose_function!( choose_card_to_damage(..., DamageChoice<'ctype>) -> CardLocation );
    choose_function!( choose_card_to_restore(..., RestoreChoice<'ctype>) -> PlayerCardLocation );
    choose_function!( choose_icon_effect(..., IconEffectChoice<'ctype>) -> Option<IconEffect> );
    choose_function!( choose_to_move_events(..., MoveEventsChoice<'ctype>) -> bool );
    choose_function!( choose_column_to_damage(..., DamageColumnChoice<'ctype>) -> ColumnIndex );
}

/// Converts a slice of IconEffects into a slice of Option<IconEffect> that includes None.
pub fn icon_effects_with_none(icon_effects: &[IconEffect]) -> Vec<Option<IconEffect>> {
    let icon_effects = icon_effects.iter().map(|icon_effect| Some(*icon_effect));
    [None].into_iter().chain(icon_effects).collect()
}
