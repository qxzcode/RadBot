mod cards;
mod radlands;

use radlands::*;

use crate::radlands::controllers::{human::HumanController, random::RandomController};

fn main() {
    println!("AutoRad, version {}\n", env!("CARGO_PKG_VERSION"));

    let camp_types = camps::get_camp_types();
    let person_types = people::get_person_types();

    let hc1 = HumanController { label: "Human 1" };
    let hc2 = HumanController { label: "Human 2" };
    // let hc1 = RandomController;
    // let hc2 = RandomController;
    let mut game_state = GameState::new(&camp_types, &person_types);

    for turn_num in 1.. {
        println!("\nTurn {}\n", turn_num);
        if let Err(result) = game_state.do_turn(&hc1, &hc2, turn_num == 1) {
            println!(
                "\nGame ended; {}",
                match result {
                    GameResult::P1Wins => "player 1 wins!",
                    GameResult::P2Wins => "player 2 wins!",
                    GameResult::Tie => "tie!",
                }
            );
            break;
        }
    }
}
