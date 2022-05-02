pub mod human;
pub mod monte_carlo;
pub mod random;

use super::*;
use crate::choices::*;

/// Trait for a player controller.
/// All functions take a GameView for the player that this controller is responsible for.
pub trait PlayerController {
    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &ActionChoice<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype>;

    fn choose_play_location<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &PlayChoice<'ctype>,
        person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation;

    fn choose_card_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &DamageChoice<'ctype>,
        destroy: bool,
        target_locs: &[CardLocation],
    ) -> CardLocation;

    fn choose_card_to_restore<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &RestoreChoice<'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation;

    fn choose_icon_effect<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &IconEffectChoice<'ctype>,
        icon_effects: &[IconEffect],
    ) -> Option<IconEffect>;

    fn choose_to_move_events<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &MoveEventsChoice<'ctype>,
    ) -> bool;
}

/// Converts a slice of IconEffects into a slice of Option<IconEffect> that includes None.
pub fn icon_effects_with_none(icon_effects: &[IconEffect]) -> Vec<Option<IconEffect>> {
    let icon_effects = icon_effects.iter().map(|icon_effect| Some(*icon_effect));
    [None].into_iter().chain(icon_effects).collect()
}
