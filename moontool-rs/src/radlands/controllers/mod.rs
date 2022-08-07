pub mod human;
pub mod mcts;
pub mod monte_carlo;
pub mod random;

use tui::{buffer::Buffer, layout::Rect};

use super::*;

/// Trait for a player controller / agent.
pub trait PlayerController<'ctype> {
    /// Choose an option index to take, given the game state and choice.
    /// Takes a GameView for the player that this controller is responsible for.
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize;
}

pub trait ControllerStats {
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}
