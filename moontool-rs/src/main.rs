mod cards;
mod radlands;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::io;
use std::io::Write;

use radlands::*;

/// A `PlayerController` that allows manual, human input.
struct HumanController {
    label: &'static str,
}

impl PlayerController for HumanController {
    fn choose_action<'a, 'ctype>(&mut self, actions: &'a [Action<'ctype>]) -> &'a Action<'ctype> {
        println!("{}'s turn: {} actions available", self.label, actions.len());
        for (i, action) in actions.iter().enumerate() {
            println!("  ({})  {action}", i + 1);
        }

        // prompt the user for an action
        loop {
            print!("Choose an action: ");
            io::stdout().flush().unwrap();

            let mut input_line = String::new();
            io::stdin()
                .read_line(&mut input_line)
                .expect("Failed to read input");

            if let Ok(index) = input_line.trim().parse::<usize>() {
                if index > 0 && index <= actions.len() {
                    return &actions[index - 1];
                }
            }
        }
    }
}

struct RandomController;

impl PlayerController for RandomController {
    fn choose_action<'a, 'ctype>(&mut self, actions: &'a [Action<'ctype>]) -> &'a Action<'ctype> {
        let mut rng = thread_rng();
        let chosen_action = actions
            .choose(&mut rng)
            .expect("choose_action called with empty actions list");
        println!("RandomController chose action: {chosen_action}");
        chosen_action
    }
}

fn main() {
    println!("AutoRad, version {}\n", env!("CARGO_PKG_VERSION"));

    let camp_types = camps::get_camp_types();
    let person_types = people::get_person_types();

    let hc1 = HumanController { label: "Human 1" };
    let hc2 = HumanController { label: "Human 2" };
    let hc1 = RandomController;
    let hc2 = RandomController;
    let mut game_state = GameState::new(&camp_types, &person_types, Box::new(hc1), Box::new(hc2));

    for turn_num in 1.. {
        println!("\nTurn {}\n", turn_num);
        game_state.do_turn(turn_num == 1);
    }
}
