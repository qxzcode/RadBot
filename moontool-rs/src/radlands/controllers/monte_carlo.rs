use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;

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

pub struct MonteCarloController<C: PlayerController, F: Fn(Player) -> C, const QUIET: bool = false>
{
    pub player: Player,
    pub num_simulations: usize,
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

        choices
            .iter()
            .max_by_key(|choice| {
                // compute the win rate of this choice
                let sum_scores: u32 = (0..self.num_simulations)
                    .map(|_| {
                        let mut game_state = randomize_unobserved(game_view.game_state);

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
                    })
                    .sum();

                if !QUIET {
                    println!(
                        "{:6.2}%  <- win rate for: {}",
                        (sum_scores as f64) / ((self.num_simulations * 2) as f64) * 100.0,
                        format_choice(choice),
                    );
                }

                sum_scores
            })
            .expect("choices is empty")
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
