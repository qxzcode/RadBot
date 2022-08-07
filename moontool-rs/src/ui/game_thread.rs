use std::{
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use super::{HistoryEntry, RedrawEvent};
use crate::{
    do_one_choice,
    radlands::{
        choices::Choice,
        controllers::{human::HumanController, mcts::MCTSController, random::RandomController},
        locations::Player,
        GameResult, GameState,
    },
};

/// The main function that runs on the game thread.
pub(super) fn game_thread_main(
    initial_state: GameState<'static>,
    initial_choice: Result<Choice<'static>, GameResult>,
    event_tx: mpsc::Sender<RedrawEvent>,
    game_history: Arc<Mutex<Vec<HistoryEntry<'static>>>>,
) {
    let mut game_state = initial_state;
    let mut cur_choice = initial_choice;

    let p1 = &mut RandomController;
    // let p1 =
    //     &mut MCTSController::<_, true>::new(Player::Player2, Duration::from_secs_f64(3.0), |_| {
    //         RandomController
    //     });
    let p2 = &mut HumanController;
    // let p2 = &mut RandomController;

    while let Ok(choice) = &cur_choice {
        // save the game state and choice for the history entry
        let history_game_state = game_state.clone();
        let history_choice = choice.clone();

        // do one choice, updating the GameState and Choice
        let (chosen_option, new_choice) = do_one_choice(&mut game_state, choice, p1, p2);
        cur_choice = new_choice;

        // add a history entry
        game_history.lock().unwrap().push(HistoryEntry {
            game_state: history_game_state,
            choice: history_choice,
            chosen_option,
        });

        // update the UI's state and choice
        event_tx
            .send(RedrawEvent::GameUpdate(
                game_state.clone(),
                cur_choice.clone(),
            ))
            .expect("Failed to send GameUpdate event");
    }
}
