use crossterm::style::Stylize;
use crossterm::{cursor, QueueableCommand};
use ordered_float::NotNan;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cmp::Ordering;
use std::fmt;
use std::io::stdout;
use std::time::{Duration, Instant};

use crate::play_to_end;
use crate::radlands::choices::*;
use crate::radlands::*;

use super::icon_effects_with_none;

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

fn get_best_choices<T>(choice_stats_vec: Vec<ChoiceStats<T>>) -> Vec<&T> {
    let mut best_win_rate = choice_stats_vec[0].win_rate();
    let mut best_choices = vec![choice_stats_vec[0].choice];
    for choice_stats in &choice_stats_vec[1..] {
        let win_rate = choice_stats.win_rate();
        match win_rate.cmp(&best_win_rate) {
            Ordering::Equal => best_choices.push(choice_stats.choice),
            Ordering::Greater => {
                best_choices.clear();
                best_choices.push(choice_stats.choice);
                best_win_rate = win_rate;
            }
            Ordering::Less => {}
        }
    }
    best_choices
}

pub struct MonteCarloController<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool = false>
{
    pub player: Player,
    pub choice_time_limit: Duration,
    pub make_rollout_controller: F,
}

impl<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool> MonteCarloController<C, F, QUIET> {
    fn get_score(&self, game_result: GameResult) -> u32 {
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
            .collect_vec();

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

        // return a random best (maximum win rate) choice
        get_best_choices(choice_stats_vec)
            .choose(&mut thread_rng())
            .unwrap()
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

macro_rules! monte_carlo_choose_impl {
    (
        $name:ident($choice:ident: $ChoiceType:ty) -> $ReturnType:ty,
        $options:expr, $phrase:expr
    ) => {
        monte_carlo_choose_impl! {
            $name(game_view, $choice: $ChoiceType) -> $ReturnType,
            {}
            $options => chosen_option,
            option => *option,
            |choice| format!("{:?}", choice),
            $phrase, ":?", chosen_option,
            return *chosen_option
        }
    };
    (
        $name:ident($game_view:ident, $choice:ident: $ChoiceType:ty) -> $ReturnType:ty,
        $initial_print:block
        $options:expr => $chosen_option:ident,
        $option:ident => $option_choose_arg:expr,
        $format_choice:expr,
        $phrase:expr, $option_disp_format:literal, $option_disp:expr,
        return $return:expr
    ) => {
        fn $name<'a, 'v, 'g: 'v, 'ctype: 'g>(
            &self,
            $game_view: &'v GameView<'g, 'ctype>,
            $choice: &'a $ChoiceType,
        ) -> $ReturnType {
            if !QUIET {
                $initial_print
            }
            let options = $options;
            let $chosen_option = self.monte_carlo_choose_impl(
                $game_view,
                |game_state, $option| $choice.choose(game_state, $option_choose_arg),
                options,
                $format_choice,
            );
            if !QUIET {
                let phrase = $phrase;
                print!("{BOLD}{self:?} chose {phrase}:{RESET} ");
                println!(concat!("{", $option_disp_format, "}"), $option_disp);
            }
            $return
        }
    };
}

impl<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool> PlayerController
    for MonteCarloController<C, F, QUIET>
{
    monte_carlo_choose_impl! {
        choose_action(game_view, choice: ActionChoice<'ctype>) -> &'a Action<'ctype>,
        { println!("\nBoard state:\n{}", game_view.game_state) }
        choice.actions() => chosen_action,
        action => action,
        |action| action.format(game_view),
        "action", "", chosen_action.format(game_view),
        return chosen_action
    }
    monte_carlo_choose_impl! {
        choose_play_location(choice: PlayChoice<'ctype>) -> PlayLocation,
        choice.locations(), "play location"
    }
    monte_carlo_choose_impl! {
        choose_card_to_damage(choice: DamageChoice<'ctype>) -> CardLocation,
        choice.locations(),
        if choice.destroy() { "destroy target" } else { "damage target" }
    }
    monte_carlo_choose_impl! {
        choose_card_to_restore(choice: RestoreChoice<'ctype>) -> PlayerCardLocation,
        choice.locations(), "restore target"
    }
    monte_carlo_choose_impl! {
        choose_icon_effect(choice: IconEffectChoice<'ctype>) -> Option<IconEffect>,
        &icon_effects_with_none(choice.icon_effects()), "icon effect"
    }
    monte_carlo_choose_impl! {
        choose_to_move_events(game_view, choice: MoveEventsChoice<'ctype>) -> bool,
        {}
        &[false, true] => move_events,
        move_events => *move_events,
        |move_events| {
            if *move_events {
                "move events back".to_string()
            } else {
                "don't move events back".to_string()
            }
        },
        "to move events back", "", if *move_events { "yes" } else { "no" },
        return *move_events
    }
    monte_carlo_choose_impl! {
        choose_column_to_damage(choice: DamageColumnChoice<'ctype>) -> ColumnIndex,
        choice.columns(), "column to damage"
    }
}

impl<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool> fmt::Debug
    for MonteCarloController<C, F, QUIET>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MonteCarloController[{:?}]", self.player)
    }
}
