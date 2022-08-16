use ordered_float::NotNan;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;
use std::time::{Duration, Instant};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{List, ListItem, Widget};

use crate::play_to_end;
use crate::radlands::choices::*;
use crate::radlands::*;
use crate::ui::set_controller_stats;

use super::ControllerStats;

pub fn randomize_unobserved<'ctype>(game_state: &GameState<'ctype>) -> GameState<'ctype> {
    let mut rng = thread_rng();
    let mut new_game_state = game_state.clone();

    // shuffle the deck
    new_game_state.deck.shuffle(&mut rng);

    // TODO: shuffle all unobserved cards (deck, other player's hand, punks)

    new_game_state
}

pub fn get_score(game_result: GameResult, for_player: Player) -> u32 {
    match game_result {
        GameResult::P1Wins => match for_player {
            Player::Player1 => 2,
            Player::Player2 => 0,
        },
        GameResult::Tie => 1,
        GameResult::P2Wins => match for_player {
            Player::Player1 => 0,
            Player::Player2 => 2,
        },
    }
}

pub fn compute_rollout_score<'ctype, C: PlayerController<'ctype>>(
    for_player: Player,
    game_state: &GameState<'ctype>,
    choice: &Choice<'ctype>,
    make_rollout_controller: &impl Fn(Player) -> C,
    option_index: usize,
) -> u32 {
    let mut game_state = randomize_unobserved(game_state);

    let game_result = match choice.choose(&mut game_state, option_index) {
        Err(game_result) => game_result,
        Ok(choice) => play_to_end(
            &mut game_state,
            choice,
            &mut (make_rollout_controller)(Player::Player1),
            &mut (make_rollout_controller)(Player::Player2),
        ),
    };

    get_score(game_result, for_player)
}

#[derive(Debug, Clone)]
pub struct OptionStats {
    pub num_rollouts: u32,
    pub total_score: u32,
}

impl OptionStats {
    pub fn win_rate(&self) -> NotNan<f64> {
        if self.num_rollouts == 0 {
            NotNan::new(0.5).unwrap()
        } else {
            let win_rate = (self.total_score as f64) / ((self.num_rollouts * 2) as f64);
            NotNan::new(win_rate).expect("win rate is NaN")
        }
    }

    /// The UCB1 score for a choice.
    /// https://gibberblot.github.io/rl-notes/single-agent/multi-armed-bandits.html
    pub fn ucb1_score(&self, rollout_num: usize) -> NotNan<f64> {
        self.win_rate() + (2.0 * (rollout_num as f64).ln() / (self.num_rollouts as f64)).sqrt()
    }

    /// A variant of the PUCT score, similar to that used in AlphaZero.
    pub fn puct_score(&self, parent_rollouts: u32) -> NotNan<f64> {
        let exploration_rate = 1.0; // TODO: make this a tunable parameter
        let exploration_score =
            exploration_rate * (parent_rollouts as f64).sqrt() / ((1 + self.num_rollouts) as f64);
        self.win_rate() + exploration_score
    }
}

pub fn format_option_stats<'g, 'ctype: 'g>(
    option_stats_vec: &[OptionStats],
    parent_rollouts: usize,
    game_view: &GameView<'g, 'ctype>,
    choice: &Choice<'ctype>,
) -> Vec<ListItem<'static>> {
    let max_visit_count = option_stats_vec
        .iter()
        .map(|option_stats| option_stats.num_rollouts)
        .max()
        .expect("self.option_stats is empty");

    option_stats_vec
        .iter()
        .enumerate()
        .map(|(option_index, option_stats)| {
            let stats = format!(
                "{:8}  {:6.2}%  {:6.2}%",
                option_stats.num_rollouts,
                (option_stats.num_rollouts as f64) / (parent_rollouts as f64) * 100.0,
                option_stats.win_rate() * 100.0,
            );
            let stats_style = if option_stats.num_rollouts == max_visit_count {
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow)
            } else {
                Style::default()
            };
            let mut spans = choice.format_option(option_index, game_view.game_state);
            spans
                .0
                .splice(0..0, [Span::styled(stats, stats_style), "   ".into()]);
            ListItem::new(spans)
        })
        .collect()
}

pub fn show_option_stats<'g, 'ctype: 'g>(
    option_stats_vec: &[OptionStats],
    parent_rollouts: usize,
    game_view: &GameView<'g, 'ctype>,
    choice: &Choice<'ctype>,
) {
    let lines = format_option_stats(option_stats_vec, parent_rollouts, game_view, choice);
    set_controller_stats(Some(Box::new(StatsWidget { lines })), game_view.player);
}

pub struct StatsWidget<'a> {
    pub lines: Vec<ListItem<'a>>,
}
impl ControllerStats for StatsWidget<'_> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        List::new(self.lines.clone()).render(area, buf);
    }
}

pub fn get_best_options(option_stats_vec: &[OptionStats]) -> Vec<usize> {
    let max_visit_count = option_stats_vec
        .iter()
        .map(|option_stats| option_stats.num_rollouts)
        .max()
        .expect("option_stats_vec is empty");

    option_stats_vec
        .iter()
        .enumerate()
        .filter(|(_, option_stats)| option_stats.num_rollouts == max_visit_count)
        .map(|(option_index, _)| option_index)
        .collect()
}

pub struct MonteCarloController<F> {
    pub player: Player,
    pub choice_time_limit: Duration,
    pub make_rollout_controller: F,
}

impl<'ctype, C: PlayerController<'ctype>, F: Fn(Player) -> C> MonteCarloController<F> {
    fn monte_carlo_choose_impl<'g>(
        &self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        let num_options = choice.num_options(game_view.game_state);
        if num_options == 1 {
            return 0;
        }

        let start_time = Instant::now();

        let mut option_stats_vec = (0..num_options)
            .map(|option_index| OptionStats {
                num_rollouts: 1,
                total_score: compute_rollout_score(
                    self.player,
                    game_view.game_state,
                    choice,
                    &self.make_rollout_controller,
                    option_index,
                ),
            })
            .collect_vec();

        let mut last_print_time = start_time;
        let mut rollout_num = num_options;
        show_option_stats(&option_stats_vec, rollout_num, game_view, choice);
        while start_time.elapsed() < self.choice_time_limit {
            // choose a choice to simulate using UCB1
            let (option_index, option_stats) = option_stats_vec
                .iter_mut()
                .enumerate()
                .max_by_key(|(_, option_stats)| option_stats.ucb1_score(rollout_num))
                .unwrap();

            // perform a rollout for that choice
            rollout_num += 1;
            option_stats.num_rollouts += 1;
            option_stats.total_score += compute_rollout_score(
                self.player,
                game_view.game_state,
                choice,
                &self.make_rollout_controller,
                option_index,
            );

            // update the live stats display
            let now = Instant::now();
            let elapsed = now.duration_since(last_print_time);
            if elapsed > Duration::from_millis(100) {
                show_option_stats(&option_stats_vec, rollout_num, game_view, choice);
                last_print_time = now;
            }
        }
        show_option_stats(&option_stats_vec, rollout_num, game_view, choice);

        // return a random best (maximum visit count) choice
        *get_best_options(&option_stats_vec)
            .choose(&mut thread_rng())
            .unwrap()
    }
}

impl<'ctype, C: PlayerController<'ctype>, F: Fn(Player) -> C> PlayerController<'ctype>
    for MonteCarloController<F>
{
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        self.monte_carlo_choose_impl(game_view, choice)
    }
}

impl<F> fmt::Debug for MonteCarloController<F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MonteCarloController[{:?}]", self.player)
    }
}
