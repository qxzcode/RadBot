use rand::thread_rng;

use crate::radlands::choices::*;
use crate::radlands::*;

pub struct RandomController;

impl<'ctype> PlayerController<'ctype> for RandomController {
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        thread_rng().gen_range(0..choice.num_options(game_view.game_state))
    }
}
