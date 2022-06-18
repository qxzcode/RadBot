use rand::thread_rng;

use crate::radlands::choices::*;
use crate::radlands::*;

pub struct RandomController {
    pub quiet: bool,
}

impl<'ctype> PlayerController<'ctype> for RandomController {
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        let chosen_option = thread_rng().gen_range(0..choice.num_options(game_view.game_state));
        if !self.quiet {
            println!(
                "{BOLD}RandomController chose:{RESET} {}",
                choice.format_option(chosen_option, game_view),
            );
        }
        chosen_option
    }
}
