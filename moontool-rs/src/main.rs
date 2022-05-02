mod cards;
mod radlands;

use clap::Parser;
use std::time::Duration;

use radlands::camps::CampType;
use radlands::choices::Choice;
use radlands::locations::Player;
use radlands::people::PersonType;
use radlands::*;

use radlands::controllers::{
    human::HumanController, monte_carlo::MonteCarloController, random::RandomController,
    PlayerController,
};

fn validate_secs(s: &str) -> Result<(), String> {
    let secs = s.parse::<f64>().map_err(|_| "invalid number".to_string())?;
    if secs > 0.0 {
        Ok(())
    } else {
        Err("number of seconds must be positive".to_string())
    }
}

#[derive(Parser, Debug)]
#[clap(
    name = "RadBot",
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
struct Args {
    /// Play a bunch of random games to fuzz-test the game logic
    #[clap(short, long, conflicts_with = "humans")]
    random: bool,

    /// Run a game with 2 human/manual players
    #[clap(short, long, conflicts_with = "random")]
    humans: bool,

    /// The number of seconds the AI is allowed to think for each decision
    #[clap(
        short = 't', long,
        value_name = "SECONDS",
        default_value = "3.0",
        validator = validate_secs,
    )]
    ai_time_limit: f64,
}

fn main() {
    let args = Args::parse();

    println!("RadBot, version {}\n", env!("CARGO_PKG_VERSION"));

    let camp_types = camps::get_camp_types();
    let person_types = people::get_person_types();

    if args.random {
        let num_games = 100_000;
        println!("Running {} random games...", num_games);
        for _ in 0..num_games {
            do_game(&camp_types, &person_types, &args);
        }
    } else {
        do_game(&camp_types, &person_types, &args);
    }
}

fn do_game(camp_types: &[CampType], person_types: &[PersonType], args: &Args) {
    let p1: Box<dyn PlayerController>;
    let p2: Box<dyn PlayerController>;
    if args.random {
        p1 = Box::new(RandomController { quiet: true });
        p2 = Box::new(RandomController { quiet: true });
    } else if args.humans {
        p1 = Box::new(HumanController { label: "Human 1" });
        p2 = Box::new(HumanController { label: "Human 2" });
    } else {
        let ai_time_limit = Duration::from_secs_f64(args.ai_time_limit);
        println!("AI time limit: {:?}", ai_time_limit);
        p1 = Box::new(MonteCarloController::<_, _, false> {
            player: Player::Player1,
            choice_time_limit: ai_time_limit,
            make_rollout_controller: |_| RandomController { quiet: true },
        });
        p2 = Box::new(HumanController { label: "Human" });
    }

    let (mut game_state, choice) = GameState::new(camp_types, person_types);

    let result = play_to_end(&mut game_state, choice, p1.as_ref(), p2.as_ref());

    if !args.random {
        println!(
            "\nGame ended; {}",
            match result {
                GameResult::P1Wins => "player 1 wins!",
                GameResult::P2Wins => "player 2 wins!",
                GameResult::Tie => "tie!",
            }
        );
        println!("\nFinal state:\n{}", game_state);
    }
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
        Choice::MoveEvents(move_events_choice) => {
            let move_events = get_controller_for(move_events_choice.chooser())
                .choose_to_move_events(
                    &game_state.view_for(move_events_choice.chooser()),
                    move_events_choice,
                );
            move_events_choice.choose(game_state, move_events)
        }
        Choice::DamageColumn(damage_column_choice) => {
            let column = get_controller_for(damage_column_choice.chooser())
                .choose_column_to_damage(
                    &game_state.view_for(damage_column_choice.chooser()),
                    damage_column_choice,
                );
            damage_column_choice.choose(game_state, column)
        }
    }
}
