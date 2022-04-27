mod cards;
mod radlands;

use radlands::camps::CampType;
use radlands::choices::Choice;
use radlands::locations::Player;
use radlands::people::PersonType;
use radlands::*;

use crate::radlands::controllers::{
    human::HumanController, monte_carlo::MonteCarloController, random::RandomController,
    PlayerController,
};

fn main() {
    println!("AutoRad, version {}\n", env!("CARGO_PKG_VERSION"));

    let camp_types = camps::get_camp_types();
    let person_types = people::get_person_types();

    let do_random = std::env::args().any(|arg| arg == "--random");
    let do_hvm = std::env::args().any(|arg| arg == "--hvm");

    let num_games = if do_random { 10_000 } else { 1 };
    let sum: u32 = (0..num_games)
        .into_iter()
        .map(|_| do_game(&camp_types, &person_types, do_random, do_hvm))
        .sum();
    if num_games > 1 {
        println!("Average final turn: {}", (sum as f64) / (num_games as f64));
    }
}

fn do_game(camp_types: &[CampType], person_types: &[PersonType], random: bool, hvm: bool) -> u32 {
    let p1: &dyn PlayerController;
    let p2: &dyn PlayerController;
    if random {
        p1 = &RandomController { quiet: false };
        p2 = &RandomController { quiet: false };
    } else if hvm {
        p1 = &MonteCarloController::<_, _, false> {
            player: Player::Player1,
            num_simulations: 50_000,
            make_rollout_controller: |_| RandomController { quiet: true },
        };
        p2 = &HumanController { label: "Human" };
    } else {
        p1 = &HumanController { label: "Human 1" };
        p2 = &HumanController { label: "Human 2" };
    }
    // let hc1 = RandomController;
    // let hc2 = RandomController;
    let (mut game_state, choice) = GameState::new(camp_types, person_types);

    let result = play_to_end(&mut game_state, choice, p1, p2);
    println!(
        "\nGame ended; {}",
        match result {
            GameResult::P1Wins => "player 1 wins!",
            GameResult::P2Wins => "player 2 wins!",
            GameResult::Tie => "tie!",
        }
    );

    println!("\nFinal state:\n{}", game_state);

    // TODO: get the final turn number
    0
}

pub fn play_to_end<'ctype>(
    game_state: &mut GameState<'ctype>,
    mut choice: Choice<'ctype>,
    p1: &dyn PlayerController,
    p2: &dyn PlayerController,
) -> GameResult {
    loop {
        match do_one_choice(game_state, &choice, p1, p2) {
            Ok(new_choice) => choice = new_choice,
            Err(game_result) => return game_result,
        }
    }
}

fn do_one_choice<'ctype>(
    game_state: &mut GameState<'ctype>,
    choice: &Choice<'ctype>,
    p1: &dyn PlayerController,
    p2: &dyn PlayerController,
) -> Result<Choice<'ctype>, GameResult> {
    let get_controller_for = |player: Player| match player {
        Player::Player1 => p1,
        Player::Player2 => p2,
    };

    match choice {
        Choice::Action(action_choice) => {
            let action = get_controller_for(game_state.cur_player).choose_action(
                &game_state.view_for_cur(),
                action_choice,
                action_choice.actions(),
            );
            action_choice.choose(game_state, action)
        }
        Choice::PlayLoc(play_choice) => {
            let loc = get_controller_for(play_choice.chooser()).choose_play_location(
                &game_state.view_for(play_choice.chooser()),
                play_choice,
                play_choice.person(),
                play_choice.locations(),
            );
            play_choice.choose(game_state, loc)
        }
        Choice::Damage(damage_choice) => {
            let loc = get_controller_for(damage_choice.chooser()).choose_card_to_damage(
                &game_state.view_for(damage_choice.chooser()),
                damage_choice,
                damage_choice.destroy(),
                damage_choice.locations(),
            );
            damage_choice.choose(game_state, loc)
        }
        Choice::Restore(restore_choice) => {
            let loc = get_controller_for(restore_choice.chooser()).choose_card_to_restore(
                &game_state.view_for(restore_choice.chooser()),
                restore_choice,
                restore_choice.locations(),
            );
            restore_choice.choose(game_state, loc)
        }
        Choice::IconEffect(icon_effect_choice) => {
            let icon_effect = get_controller_for(icon_effect_choice.chooser()).choose_icon_effect(
                &game_state.view_for(icon_effect_choice.chooser()),
                icon_effect_choice,
                icon_effect_choice.icon_effects(),
            );
            icon_effect_choice.choose(game_state, icon_effect)
        }
    }
}
