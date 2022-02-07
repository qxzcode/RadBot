mod cards;
mod radlands;

use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::io;
use std::io::Write;
use std::ops::RangeBounds;
use std::str::FromStr;

use radlands::styles::*;
use radlands::*;

/// Prompts the user for a valid number within some range and returns it.
fn prompt_for_number<T: FromStr + PartialOrd>(prompt: &str, range: impl RangeBounds<T>) -> T {
    loop {
        print!("{prompt}");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input_line = String::new();
        io::stdin()
            .read_line(&mut input_line)
            .expect("Failed to read input");

        if let Ok(number) = input_line.trim().parse::<T>() {
            if range.contains(&number) {
                return number;
            }
        }
    }
}

/// A `PlayerController` that allows manual, human input.
struct HumanController {
    label: &'static str,
}

impl PlayerController for HumanController {
    fn choose_action<'a, 'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype> {
        // clear the screen and print the game state
        print!("\x1b[2J\x1b[H");
        println!("{}\n", game_state);

        // print the available actions
        println!("{} - choose an action:", self.label);
        for (i, action) in actions.iter().enumerate() {
            println!("  ({})  {action}", i + 1);
        }

        // prompt the user for an action
        let action_number = prompt_for_number("Choose an action: ", 1..=actions.len());
        &actions[action_number - 1]
    }

    fn choose_play_location<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation {
        let style_person_slot = |slot: &Option<Person>| match slot {
            Some(person) => person.styled_name(),
            None => StyledString::empty(),
        };
        let table_columns = game_state.cur_player().columns.iter().map(|col| {
            vec![
                style_person_slot(&col.person_slots[1]),
                style_person_slot(&col.person_slots[0]),
                StyledString::empty(),
                col.camp.styled_name(),
            ]
        });
        let mut table_columns = table_columns.collect_vec();

        for (i, loc) in locations.iter().enumerate() {
            table_columns[loc.column() as usize][((1 - loc.row()) * 2) as usize] =
                StyledString::plain(&format!("({}) play here", i + 1));
        }

        println!();
        print!("{}", StyledTable::new(table_columns, "").reduce_rows());
        let loc_number = prompt_for_number(
            &format!("Choose a location to play {}: ", person.styled_name()),
            1..=locations.len(),
        );
        locations[loc_number - 1]
    }
}

struct RandomController;

impl PlayerController for RandomController {
    fn choose_action<'a, 'g, 'ctype: 'g>(
        &self,
        _game_state: &'g GameState<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype> {
        let mut rng = thread_rng();
        let chosen_action = actions
            .choose(&mut rng)
            .expect("choose_action called with empty actions list");
        println!("{BOLD}RandomController chose action:{RESET} {chosen_action}");
        chosen_action
    }

    fn choose_play_location<'g, 'ctype: 'g>(
        &self,
        _game_state: &'g GameState<'ctype>,
        _person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation {
        let mut rng = thread_rng();
        let chosen_location = locations
            .choose(&mut rng)
            .expect("choose_play_location called with empty locations list");
        println!("{BOLD}RandomController chose location:{RESET} {chosen_location:?}");
        *chosen_location
    }
}

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
