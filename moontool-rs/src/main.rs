mod cards;
mod radlands;

use radlands::*;

/// A `PlayerController` that allows manual, human input.
struct HumanController {
    label: &'static str,
}

impl PlayerController for HumanController {
    fn choose_action<'ctype>(&mut self, actions: &[Action<'ctype>]) -> Action<'ctype> {
        println!("{}'s turn: {} actions available", self.label, actions.len());
        todo!()
    }
}

fn main() {
    println!("AutoRad, version {}\n", env!("CARGO_PKG_VERSION"));

    let camp_types = camps::get_camp_types();
    let person_types = people::get_person_types();

    let hc1 = HumanController { label: "Human 1" };
    let hc2 = HumanController { label: "Human 2" };
    let mut game_state = GameState::new(&camp_types, &person_types, Box::new(hc1), Box::new(hc2));

    game_state.do_turn(true);
    game_state.do_turn(false);
}
