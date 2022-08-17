use rand::thread_rng;
use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};
use tui::widgets::ListItem;

use crate::radlands::choices::*;
use crate::radlands::observed_state::ObservedState;
use crate::radlands::*;
use crate::ui;

use super::monte_carlo::{
    compute_rollout_score, format_option_stats, get_best_options, get_score, randomize_unobserved,
    OptionStats, StatsWidget,
};

#[derive(Debug, Clone)]
struct StateStats {
    options: Vec<OptionStats>,
    num_rollouts: u32,
    last_visit_ply: u32,
}

impl StateStats {
    fn new(num_options: usize, current_ply: u32) -> Self {
        debug_assert!(num_options > 1, "Expanded a state with less than 2 options");
        Self {
            options: vec![
                OptionStats {
                    num_rollouts: 0,
                    total_score: 0,
                };
                num_options
            ],
            num_rollouts: 0,
            last_visit_ply: current_ply,
        }
    }
}

pub struct MCTSController<'ctype, F> {
    pub player: Player,
    pub choice_time_limit: Duration,
    pub make_rollout_controller: F,

    explored_states: HashMap<ObservedState<'ctype>, StateStats>,
    current_ply: u32,
}

impl<'g, 'ctype: 'g, C: PlayerController<'ctype>, F: Fn(Player) -> C> MCTSController<'ctype, F> {
    pub fn new(player: Player, choice_time_limit: Duration, make_rollout_controller: F) -> Self {
        Self {
            player,
            choice_time_limit,
            make_rollout_controller,
            explored_states: HashMap::new(),
            current_ply: 0,
        }
    }

    fn get_root_option_stats(
        &self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> (u32, &[OptionStats]) {
        let game_state = game_view.game_state;
        let chooser = choice.chooser(game_state);
        let observed_state = ObservedState::from_game_state(game_state, choice, chooser);
        self.explored_states
            .get(&observed_state)
            .map(|stats| (stats.num_rollouts, stats.options.as_slice()))
            .expect("root state not explored")
    }

    fn show_stats(
        &self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
        num_samples: i32,
        start_time: Instant,
    ) {
        let mut lines;
        let title;
        if ui::get_debug_counter() % 2 == 0 {
            title = "Options at current choice root:";
            let (rollouts, option_stats) = self.get_root_option_stats(game_view, choice);
            lines = format_option_stats(option_stats, rollouts as usize, game_view, choice);
        } else {
            title = "Most visited sequence:";
            lines = self.format_predicted_sequence(game_view, choice);
        }

        let elapsed = start_time.elapsed();
        let top_lines = [
            format!(
                "For this choice, {num_samples} samples performed in {elapsed:.1?} ({:.1} samples/sec)",
                (num_samples as f64) / elapsed.as_secs_f64(),
            ),
            format!("Nodes in cache: {}", self.explored_states.len()),
            " ".into(), // creates a blank line
            title.into(),
            "# Visits    Visit %    Win %    Option".into(),
            "--------  ----------  -------   ------".into(),
        ];
        lines.splice(0..0, top_lines.into_iter().map(ListItem::new));

        ui::set_controller_stats(Some(Box::new(StatsWidget { lines })), game_view.player);
    }

    fn format_predicted_sequence(
        &self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> Vec<ListItem<'static>> {
        let mut game_state = randomize_unobserved(game_view.game_state);
        let mut choice = Cow::Borrowed(choice);

        // collect most likely move sequence
        let mut lines = Vec::new();
        let mut root_count = None;
        loop {
            // immediately continue to the next move if there's only one option
            let num_options = choice.num_options(&game_state);

            let (option_index, stats) = if num_options == 1 {
                (0, None)
            } else {
                // get which player needs to make a move
                let chooser = choice.chooser(&game_state);

                // get the observed state of the game (hash table key)
                let observed_state = ObservedState::from_game_state(&game_state, &choice, chooser);

                if let Some(stats) = self.explored_states.get(&observed_state) {
                    if root_count.is_none() {
                        root_count = Some(stats.num_rollouts);
                    }

                    let (i, opt) = stats
                        .options
                        .iter()
                        .enumerate()
                        .max_by_key(|(_i, opt)| opt.num_rollouts)
                        .unwrap();
                    (
                        i,
                        Some((
                            opt.num_rollouts,
                            (opt.num_rollouts as f64) / (root_count.unwrap() as f64),
                            opt.win_rate() * 100.0,
                        )),
                    )
                } else {
                    // we've reached a state that hasn't been visited, so stop the traversal
                    break;
                }
            };

            lines.push(ListItem::new({
                let mut spans = choice.format_option(option_index, &game_state);
                spans.0.splice(
                    0..0,
                    [if let Some((count, visit_proportion, win_rate)) = stats {
                        let bar_width = (visit_proportion * 10.0).round() as usize;
                        format!(
                            "{count:8}  {}{}  {win_rate:6.2}%   ",
                            ".".repeat(10 - bar_width),
                            "#".repeat(bar_width),
                        )
                    } else {
                        " ".repeat(8 + (2 + 10) + (2 + 6 + 1) + 3)
                    }
                    .into()],
                );
                spans
            }));

            match choice.choose(&mut game_state, option_index) {
                Err(_game_result) => break,
                Ok(next_choice) => choice = Cow::Owned(next_choice),
            };
        }

        lines
    }

    fn prune_explored_states(&mut self) {
        const PAST_PLIES_TO_KEEP: u32 = 5;
        if self.current_ply > PAST_PLIES_TO_KEEP {
            let cutoff_ply = self.current_ply - PAST_PLIES_TO_KEEP;
            self.explored_states
                .retain(|_, state_stats| state_stats.last_visit_ply >= cutoff_ply);
        }
    }

    /// Runs MCTS to choose an option.
    fn mcts_choose_impl(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        // return immediately without searching if there's only one option
        let num_options = choice.num_options(game_view.game_state);
        if num_options == 1 {
            return 0;
        }

        let start_time = Instant::now();

        self.current_ply += 1;
        self.prune_explored_states();

        let mut last_print_time = start_time;
        let mut num_samples = 0;
        while start_time.elapsed() < self.choice_time_limit {
            // sample a sequence of moves and update the tree
            let mut game_state = randomize_unobserved(game_view.game_state);
            self.sample_move(&mut game_state, choice);
            num_samples += 1;

            // update the live stats display
            let now = Instant::now();
            let elapsed = now.duration_since(last_print_time);
            if elapsed > Duration::from_millis(100) {
                self.show_stats(game_view, choice, num_samples, start_time);
                last_print_time = now;
            }
        }
        self.show_stats(game_view, choice, num_samples, start_time);

        // return a random best (maximum visit count) choice
        *get_best_options(self.get_root_option_stats(game_view, choice).1)
            .choose(&mut thread_rng())
            .unwrap()
    }

    /// Samples a move that a player might make from a state, updating the search tree.
    /// Returns a tuple of (chosen option index, rollout score for Player 1).
    fn sample_move(
        &mut self,
        game_state: &mut GameState<'ctype>,
        choice: &Choice<'ctype>,
    ) -> (usize, u32) {
        // immediately continue to the next move if there's only one option
        let num_options = choice.num_options(game_state);
        if num_options == 1 {
            let score = match choice.choose(game_state, 0) {
                Err(game_result) => get_score(game_result, Player::Player1),
                Ok(next_choice) => self.sample_move(game_state, &next_choice).1,
            };
            return (0, score);
        }

        // get which player needs to make a move
        let chooser = choice.chooser(game_state);

        // get the observed state of the game (hash table key)
        let observed_state = ObservedState::from_game_state(game_state, choice, chooser);

        // sample an option and the score for Player 1
        let (option_index, rollout_score) = match self.explored_states.entry(observed_state.clone())
        {
            Entry::Vacant(entry) => {
                // this is the first time we've seen this state, so create a new entry
                entry.insert(StateStats::new(num_options, self.current_ply));

                // at leaf nodes, start by sampling a random option
                let first_move = thread_rng().gen_range(0..num_options);

                // perform a rollout from this state
                let final_score = compute_rollout_score(
                    Player::Player1,
                    game_state,
                    choice,
                    &self.make_rollout_controller,
                    first_move,
                );

                (first_move, final_score)
            }
            Entry::Occupied(entry) => {
                // this state has been seen before; get the stored stats
                let state_stats = entry.into_mut();
                state_stats.last_visit_ply = self.current_ply;

                // choose an option based on the current stats
                let (option_index, _) = state_stats
                    .options
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, option_stats)| {
                        option_stats.puct_score(state_stats.num_rollouts)
                    })
                    .unwrap();

                // get the next state and recurse (or return the result if the game ended)
                let score = match choice.choose(game_state, option_index) {
                    Err(game_result) => get_score(game_result, Player::Player1),
                    Ok(next_choice) => self.sample_move(game_state, &next_choice).1,
                };

                (option_index, score)
            }
        };

        // update the stats for this option
        let state_stats = self.explored_states.get_mut(&observed_state).unwrap();
        state_stats.num_rollouts += 1;
        let option_stats = &mut state_stats.options[option_index];
        option_stats.num_rollouts += 1;
        option_stats.total_score += match chooser {
            Player::Player1 => rollout_score,
            Player::Player2 => 2 - rollout_score,
        };

        // return the chosen option index and rollout score
        (option_index, rollout_score)
    }
}

impl<'ctype, C: PlayerController<'ctype>, F: Fn(Player) -> C> PlayerController<'ctype>
    for MCTSController<'ctype, F>
{
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        self.mcts_choose_impl(game_view, choice)
    }
}

impl<F> fmt::Debug for MCTSController<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MCTSController[{:?}]", self.player)
    }
}
