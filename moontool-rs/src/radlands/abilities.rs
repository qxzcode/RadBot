use super::{GameResult, GameView, IconEffect};

/// An ability on a camp or person.
pub trait Ability {
    /// Returns the water cost of this ability.
    fn cost<'v, 'g: 'v, 'ctype: 'g>(&self, game_view: &'v GameView<'g, 'ctype>) -> u32;

    /// Returns whether this ability can be used given the game state.
    /// Does not need to check for the water cost.
    fn can_perform<'v, 'g: 'v, 'ctype: 'g>(&self, game_view: &'v GameView<'g, 'ctype>) -> bool;

    /// Performs this ability.
    fn perform<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v mut GameView<'g, 'ctype>,
    ) -> Result<(), GameResult>;

    /// Returns whether this ability can be afforded and used given the game state.
    fn can_afford_and_perform<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
    ) -> bool {
        game_view.game_state.cur_player_water >= self.cost(game_view) && self.can_perform(game_view)
    }
}

pub struct IconAbility {
    pub effect: IconEffect,
    pub cost: u32,
}

impl Ability for IconAbility {
    fn cost<'v, 'g: 'v, 'ctype: 'g>(&self, _game_view: &'v GameView<'g, 'ctype>) -> u32 {
        self.cost
    }

    fn can_perform<'v, 'g: 'v, 'ctype: 'g>(&self, game_view: &'v GameView<'g, 'ctype>) -> bool {
        self.effect.can_perform(game_view)
    }

    fn perform<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v mut GameView<'g, 'ctype>,
    ) -> Result<(), GameResult> {
        self.effect.perform(game_view)
    }
}
