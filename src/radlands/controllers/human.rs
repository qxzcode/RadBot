use crate::radlands::choices::*;
use crate::radlands::*;
use crate::ui::get_user_input;

/// A `PlayerController` that allows manual, human input.
pub struct HumanController;

impl<'ctype> PlayerController<'ctype> for HumanController {
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        loop {
            let input = get_user_input();
            if let Ok(action_number) = input.parse() {
                if (1..=choice.num_options(game_view.game_state)).contains(&action_number) {
                    return action_number - 1;
                }
            }
        }
    }
}
