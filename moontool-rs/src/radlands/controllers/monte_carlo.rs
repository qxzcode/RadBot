use crossterm::style::Stylize;
use crossterm::{cursor, QueueableCommand};
use ordered_float::NotNan;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;
use std::io::stdout;
use std::time::{Duration, Instant};

use crate::play_to_end;
use crate::radlands::choices::*;
use crate::radlands::*;

fn randomize_unobserved<'g, 'ctype: 'g>(game_state: &'g GameState<'ctype>) -> GameState<'ctype> {
    let mut rng = thread_rng();
    let mut new_game_state = game_state.clone();

    // shuffle the deck
    new_game_state.deck.shuffle(&mut rng);

    // TODO: shuffle all unobserved cards (deck, other player's hand, punks)

    new_game_state
}

struct ChoiceStats<'c, C> {
    choice: &'c C,
    num_rollouts: u32,
    total_score: u32,
}

impl<'c, C> ChoiceStats<'c, C> {
    fn win_rate(&self) -> NotNan<f64> {
        let win_rate = (self.total_score as f64) / ((self.num_rollouts * 2) as f64);
        NotNan::new(win_rate).expect("win rate is NaN")
    }

    /// The UCB1 score for a choice.
    /// https://gibberblot.github.io/rl-notes/single-agent/multi-armed-bandits.html
    fn ucb1_score(&self, rollout_num: usize) -> NotNan<f64> {
        self.win_rate() + (2.0 * (rollout_num as f64).ln() / (self.num_rollouts as f64)).sqrt()
    }
}

fn print_choice_stats<C>(
    choice_stats_vec: &[ChoiceStats<C>],
    format_choice: impl Fn(&C) -> String,
    is_first_print: bool,
) {
    let mut stdout = stdout();

    if !is_first_print {
        let num_lines = choice_stats_vec.len().try_into().unwrap();
        stdout.queue(cursor::MoveToPreviousLine(num_lines)).unwrap();
    }

    let max_win_rate = choice_stats_vec
        .iter()
        .map(|choice_stats| choice_stats.win_rate())
        .max()
        .expect("choice_stats_vec is empty");

    for choice_stats in choice_stats_vec {
        let win_rate = choice_stats.win_rate();
        let mut win_rate_str = format!("{:6.2}%", win_rate * 100.0);
        if win_rate == max_win_rate {
            win_rate_str = win_rate_str.bold().yellow().to_string();
        }
        println!(
            "{:6}  {}   {}",
            choice_stats.num_rollouts,
            win_rate_str,
            format_choice(choice_stats.choice),
        );
    }
}

pub struct MonteCarloController<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool = false>
{
    pub player: Player,
    pub choice_time_limit: Duration,
    pub make_rollout_controller: F,
}

impl<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool> MonteCarloController<C, F, QUIET> {
    fn get_score(&self, game_result: GameResult) -> u32 {
        // TODO: this returns the score for player 1
        match game_result {
            GameResult::P1Wins => match self.player {
                Player::Player1 => 2,
                Player::Player2 => 0,
            },
            GameResult::Tie => 1,
            GameResult::P2Wins => match self.player {
                Player::Player1 => 0,
                Player::Player2 => 2,
            },
        }
    }

    fn monte_carlo_choose<'c, 'v, 'g: 'v, 'ctype: 'g, T: fmt::Debug>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choose_func: impl Fn(&mut GameState<'ctype>, &T) -> Result<Choice<'ctype>, GameResult>,
        choices: &'c [T],
    ) -> &'c T {
        self.monte_carlo_choose_impl(game_view, choose_func, choices, |choice| {
            format!("{:?}", choice)
        })
    }

    fn monte_carlo_choose_impl<'c, 'v, 'g: 'v, 'ctype: 'g, T>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choose_func: impl Fn(&mut GameState<'ctype>, &T) -> Result<Choice<'ctype>, GameResult>,
        choices: &'c [T],
        format_choice: impl Fn(&T) -> String,
    ) -> &'c T {
        if choices.len() == 1 {
            return &choices[0];
        }

        let start_time = Instant::now();

        let mut choice_stats_vec = choices
            .iter()
            .map(|choice| ChoiceStats {
                choice,
                num_rollouts: 1,
                total_score: self.compute_rollout_score(game_view.game_state, &choose_func, choice),
            })
            .collect::<Vec<_>>();

        if !QUIET {
            print_choice_stats(&choice_stats_vec, &format_choice, true);
        }
        let mut last_print_time = start_time;
        let mut rollout_num = choices.len();
        while start_time.elapsed() < self.choice_time_limit {
            // choose a choice to simulate using UCB1
            let choice_stats = choice_stats_vec
                .iter_mut()
                .max_by_key(|choice_stats| choice_stats.ucb1_score(rollout_num))
                .unwrap();

            // perform a rollout for that choice
            rollout_num += 1;
            choice_stats.num_rollouts += 1;
            choice_stats.total_score +=
                self.compute_rollout_score(game_view.game_state, &choose_func, choice_stats.choice);

            // update the live stats display
            if !QUIET {
                let now = Instant::now();
                let elapsed = now.duration_since(last_print_time);
                if elapsed > Duration::from_millis(100) {
                    print_choice_stats(&choice_stats_vec, &format_choice, false);
                    last_print_time = now;
                }
            }
        }
        if !QUIET {
            print_choice_stats(&choice_stats_vec, &format_choice, false);
        }

        // TODO: if multiple choices have the same win rate, choose one at random
        choice_stats_vec
            .into_iter()
            .max_by_key(|choice_stats| choice_stats.win_rate())
            .expect("choice_stats_vec is empty")
            .choice
    }

    fn compute_rollout_score<'ctype, T>(
        &self,
        game_state: &GameState<'ctype>,
        choose_func: impl Fn(&mut GameState<'ctype>, &T) -> Result<Choice<'ctype>, GameResult>,
        choice: &T,
    ) -> u32 {
        let mut game_state = randomize_unobserved(game_state);

        let game_result = match choose_func(&mut game_state, choice) {
            Err(game_result) => game_result,
            Ok(choice) => play_to_end(
                &mut game_state,
                choice,
                &(self.make_rollout_controller)(Player::Player1),
                &(self.make_rollout_controller)(Player::Player2),
            ),
        };

        self.get_score(game_result)
    }
}

impl<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool> PlayerController
    for MonteCarloController<C, F, QUIET>
{
    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &ActionChoice<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype> {
        if !QUIET {
            println!("\nBoard state:\n{}", game_view.game_state);
        }
        let chosen_action = self.monte_carlo_choose_impl(
            game_view,
            |game_state, action| choice.choose(game_state, action),
            actions,
            |action| action.format(game_view),
        );
        if !QUIET {
            println!(
                "{BOLD}{self:?} chose action:{RESET} {}",
                chosen_action.format(game_view)
            );
        }
        chosen_action
    }

    fn choose_play_location<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &PlayChoice<'ctype>,
        _person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation {
        let chosen_location = self.monte_carlo_choose(
            game_view,
            |game_state, location| choice.choose(game_state, *location),
            locations,
        );
        if !QUIET {
            println!("{BOLD}{self:?} chose location:{RESET} {chosen_location:?}");
        }
        *chosen_location
    }

    fn choose_card_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &DamageChoice<'ctype>,
        destroy: bool,
        target_locs: &[CardLocation],
    ) -> CardLocation {
        let chosen_target = self.monte_carlo_choose(
            game_view,
            |game_state, target_loc| choice.choose(game_state, *target_loc),
            target_locs,
        );
        let verb = if destroy { "destroy" } else { "damage" };
        if !QUIET {
            println!("{BOLD}{self:?} chose {verb} target:{RESET} {chosen_target:?}");
        }
        *chosen_target
    }

    fn choose_card_to_restore<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &RestoreChoice<'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation {
        let chosen_target = self.monte_carlo_choose(
            game_view,
            |game_state, target_loc| choice.choose(game_state, *target_loc),
            target_locs,
        );
        if !QUIET {
            println!("{BOLD}{self:?} chose restore target:{RESET} {chosen_target:?}");
        }
        *chosen_target
    }

    fn choose_icon_effect<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &IconEffectChoice<'ctype>,
        icon_effects: &[IconEffect],
    ) -> IconEffect {
        let chosen_icon_effect = self.monte_carlo_choose(
            game_view,
            |game_state, icon_effect| choice.choose(game_state, *icon_effect),
            icon_effects,
        );
        if !QUIET {
            println!("{BOLD}{self:?} chose icon effect:{RESET} {chosen_icon_effect:?}");
        }
        *chosen_icon_effect
    }
}

impl<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool> fmt::Debug
    for MonteCarloController<C, F, QUIET>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MonteCarloController[{:?}]", self.player)
    }
}
