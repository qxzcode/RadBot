mod cards;
mod radlands;
mod ui;

use clap::Parser;
use std::time::Duration;

use radlands::camps::CampType;
use radlands::choices::Choice;
use radlands::locations::Player;
use radlands::people::PersonType;
use radlands::*;

use radlands::controllers::{
    human::HumanController, mcts::MCTSController, monte_carlo::MonteCarloController,
    random::RandomController, PlayerController,
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
    /// Run with the fancy UI
    #[clap(short, long)]
    ui: bool,

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

    if args.ui {
        ui::main().expect("UI error");
    } else if args.random {
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
    let mut p1: Box<dyn PlayerController>;
    let mut p2: Box<dyn PlayerController>;
    if args.random {
        p1 = Box::new(RandomController { quiet: true });
        p2 = Box::new(RandomController { quiet: true });
    } else if args.humans {
        p1 = Box::new(HumanController { label: "Human 1" });
        p2 = Box::new(HumanController { label: "Human 2" });
    } else {
        let ai_time_limit = Duration::from_secs_f64(args.ai_time_limit);
        println!("AI time limit: {:?}", ai_time_limit);
        p1 = Box::new(MonteCarloController::<_, false> {
            player: Player::Player1,
            choice_time_limit: ai_time_limit,
            make_rollout_controller: |_| RandomController { quiet: true },
        });
        p2 = Box::new(HumanController { label: "Human" });
    }

    let (mut game_state, choice) = GameState::new(camp_types, person_types);

    let result = play_to_end(&mut game_state, choice, p1.as_mut(), p2.as_mut());

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
    p1: &mut dyn PlayerController<'ctype>,
    p2: &mut dyn PlayerController<'ctype>,
) -> GameResult {
    loop {
        match do_one_choice(game_state, &choice, p1, p2).1 {
            Ok(new_choice) => choice = new_choice,
            Err(game_result) => return game_result,
        }
    }
}

fn do_one_choice<'c, 'ctype>(
    game_state: &mut GameState<'ctype>,
    choice: &Choice<'ctype>,
    p1: &'c mut dyn PlayerController<'ctype>,
    p2: &'c mut dyn PlayerController<'ctype>,
) -> (usize, Result<Choice<'ctype>, GameResult>) {
    // get the choosing player and their controller
    let chooser = choice.chooser(game_state);
    let controller = match chooser {
        Player::Player1 => p1,
        Player::Player2 => p2,
    };

    // have the controller choose an option
    let chosen_option = controller.choose_option(&game_state.view_for(chooser), choice);

    // apply the choice to the game state
    let choice_result = choice.choose(game_state, chosen_option);

    (chosen_option, choice_result)
}
