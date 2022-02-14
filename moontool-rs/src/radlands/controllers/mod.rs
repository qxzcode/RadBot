pub mod human;
pub mod random;

use super::*;

/// Trait for a player controller.
/// All functions take a GameView for the player that this controller is responsible for.
pub trait PlayerController {
    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype>;

    fn choose_play_location<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation;

    fn choose_card_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        target_locs: &[CardLocation],
    ) -> CardLocation;

    fn choose_card_to_destroy<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        target_locs: &[CardLocation],
    ) -> CardLocation;

    fn choose_card_to_restore<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation;
}
