use itertools::Itertools;
use std::io;
use std::io::Write;
use std::ops::RangeBounds;
use std::str::FromStr;

use crate::radlands::*;

/// A `PlayerController` that allows manual, human input.
pub struct HumanController {
    pub label: &'static str,
}

impl HumanController {
    /// Prompts the user for a valid number within some range and returns it.
    fn prompt_for_number<T: FromStr + PartialOrd>(
        &self,
        prompt: &str,
        range: impl RangeBounds<T>,
    ) -> T {
        loop {
            print!("[{}] {prompt}", self.label);
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
}

impl PlayerController for HumanController {
    fn choose_action<'a, 'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype> {
        // print the game state
        println!("\n{}\n", game_state);

        // print the available actions
        println!("Available actions:");
        for (i, action) in actions.iter().enumerate() {
            println!("  ({})  {action}", i + 1);
        }

        // prompt the user for an action
        let action_number = self.prompt_for_number("Choose an action: ", 1..=actions.len());
        &actions[action_number - 1]
    }

    fn choose_play_location<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation {
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
            table_columns[loc.column().as_usize()][(1 - loc.row().as_usize()) * 2] =
                StyledString::plain(&format!("({}) play here", i + 1));
        }

        println!();
        print!("{}", StyledTable::new(table_columns, "").reduce_rows());
        let loc_number = self.prompt_for_number(
            &format!("Choose a location to play {}: ", person.styled_name()),
            1..=locations.len(),
        );
        locations[loc_number - 1]
    }

    fn choose_card_to_damage<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        target_locs: &[CardLocation],
    ) -> CardLocation {
        print_card_selection(game_state, target_locs);
        let loc_number = self.prompt_for_number("Choose a card to damage: ", 1..=target_locs.len());
        target_locs[loc_number - 1]
    }

    fn choose_card_to_restore<'g, 'ctype: 'g>(
        &self,
        game_state: &'g GameState<'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation {
        print_player_card_selection(game_state, game_state.cur_player, target_locs);
        let loc_number =
            self.prompt_for_number("Choose a card to restore: ", 1..=target_locs.len());
        target_locs[loc_number - 1]
    }
}

fn style_person_slot(slot: &Option<Person>) -> StyledString {
    match slot {
        Some(person) => person.styled_name(),
        None => StyledString::empty(),
    }
}

/// Prints the board with the target cards numbered for the user to choose.
fn print_card_selection(game_state: &GameState, target_locs: &[CardLocation]) {
    assert!(!target_locs.is_empty());
    // TODO: allow mixing locations for different players
    // (some cards can target cards belonging to either player)
    assert!(
        target_locs.iter().map(|loc| loc.player()).dedup().count() == 1,
        "print_card_selection() called with cards from multiple players"
    );
    print_player_card_selection(
        game_state,
        target_locs[0].player(),
        &target_locs.iter().map(|loc| loc.player_loc()).collect_vec(),
    );
}

/// Prints the board with the target cards numbered for the user to choose.
fn print_player_card_selection(
    game_state: &GameState,
    player: Player,
    target_locs: &[PlayerCardLocation],
) {
    assert!(!target_locs.is_empty());

    let player_state = game_state.player(player);

    let table_columns = player_state.columns.iter().map(|col| {
        vec![
            style_person_slot(&col.person_slots[1]),
            style_person_slot(&col.person_slots[0]),
            col.camp.styled_name(),
        ]
    });
    let mut table_columns = table_columns.collect_vec();

    for (i, loc) in target_locs.iter().enumerate() {
        let cell = &mut table_columns[loc.column().as_usize()][2 - loc.row().as_usize()];
        *cell = &StyledString::plain(&format!("({}) ", i + 1)) + cell;
    }

    println!();
    print!("{}", StyledTable::new(table_columns, "").reduce_rows());
}
