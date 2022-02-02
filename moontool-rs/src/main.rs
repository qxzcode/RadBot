mod cards;
mod radlands;

use radlands::*;

struct HumanController {
    label: &'static str,
}

impl PlayerController for HumanController {
    fn choose_action<'ctype>(&mut self, actions: &[Action<'ctype>]) -> Action<'ctype> {
        todo!()
    }
}

fn main() {
    println!("Radlands AI, version {}\n", env!("CARGO_PKG_VERSION"));

    let camp_types = camps::get_camp_types();
    let person_types = people::get_person_types();

    let hc1 = HumanController { label: "Human 1" };
    let hc2 = HumanController { label: "Human 2" };
    let mut game_state = GameState::new(&camp_types, Box::new(hc1), Box::new(hc2));

    game_state.do_turn(true);
    game_state.do_turn(false);
}
