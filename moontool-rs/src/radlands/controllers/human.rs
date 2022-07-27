use itertools::Itertools;
use std::io;
use std::io::Write;
use std::ops::RangeBounds;
use std::str::FromStr;

use crate::radlands::choices::*;
use crate::radlands::*;
use crate::ui::get_user_input;

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

    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &'a ActionChoice<'ctype>,
    ) -> usize {
        let actions = choice.actions();

        // print the game state
        println!("\n{}\n", game_view.game_state.action_formatter(actions));

        // print the available actions
        println!("Available actions:");
        for (i, action) in actions.iter().enumerate() {
            let action_num = format!("({})", i + 1);
            println!("{action_num:>5}  {}", action.format(game_view));
        }

        // prompt the user for an action
        let action_number = self.prompt_for_number("Choose an action: ", 1..=actions.len());
        action_number - 1
    }

    fn choose_play_location<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &PlayChoice<'ctype>,
    ) -> usize {
        let person = choice.person();
        let locations = choice.locations();

        let table_columns = game_view.my_state().columns.iter().map(|col| {
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
        loc_number - 1
    }

    fn choose_card_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &DamageChoice<'ctype>,
    ) -> usize {
        let target_locs = choice.locations();
        let destroy = choice.destroy();

        print_card_selection(game_view.game_state, target_locs);
        let prompt = format!(
            "Choose a card to {}: ",
            if destroy { "destroy" } else { "damage" }
        );
        let loc_number = self.prompt_for_number(&prompt, 1..=target_locs.len());
        loc_number - 1
    }

    fn choose_card_to_restore<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &RestoreChoice<'ctype>,
    ) -> usize {
        let target_locs = choice.locations();

        print_player_card_selection(game_view.game_state, game_view.player, target_locs);
        let loc_number =
            self.prompt_for_number("Choose a card to restore: ", 1..=target_locs.len());
        loc_number - 1
    }

    fn choose_person_to_rescue<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        _choice: &RescuePersonChoice<'ctype>,
    ) -> usize {
        let target_locs = game_view.my_state().person_locs().collect_vec();

        print_player_card_selection(
            game_view.game_state,
            game_view.player,
            &target_locs.iter().map(|&loc| loc.into()).collect_vec(),
        );
        let loc_number =
            self.prompt_for_number("Choose a person to rescue: ", 1..=target_locs.len());
        loc_number - 1
    }

    fn choose_column_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &DamageColumnChoice<'ctype>,
    ) -> usize {
        print_player_card_selection(
            game_view.game_state,
            game_view.player.other(),
            &choice
                .columns()
                .iter()
                .map(|col| PlayerCardLocation::new(*col, CardRowIndex::camp()))
                .collect_vec(),
        );
        let column_number =
            self.prompt_for_number("Choose a column to damage: ", 1..=choice.columns().len());
        column_number - 1
    }
}

impl<'ctype> PlayerController<'ctype> for HumanController {
    fn choose_option<'g>(
        &mut self,
        game_view: &GameView<'g, 'ctype>,
        choice: &Choice<'ctype>,
    ) -> usize {
        loop {
            let input = get_user_input();
            if let Ok(action_number) = input.parse() {
                if (1..=choice.num_options(game_view.game_state)).contains(&action_number) {
                    return action_number - 1;
                }
            }
        }

        match choice {
            Choice::Action(choice) => self.choose_action(game_view, choice),
            Choice::PlayLoc(choice) => self.choose_play_location(game_view, choice),
            Choice::Damage(choice) => self.choose_card_to_damage(game_view, choice),
            Choice::Restore(choice) => self.choose_card_to_restore(game_view, choice),
            Choice::RescuePerson(choice) => self.choose_person_to_rescue(game_view, choice),
            Choice::DamageColumn(choice) => self.choose_column_to_damage(game_view, choice),
            _ => {
                // print the available options
                println!("Available options:");
                let num_options = choice.num_options(game_view.game_state);
                for option_index in 0..num_options {
                    let option_num = format!("({})", option_index + 1);
                    println!(
                        "{option_num:>5}  {}",
                        choice.format_option(option_index, game_view),
                    );
                }

                // prompt the user for an option
                let option_number = self.prompt_for_number("Choose an option: ", 1..=num_options);
                option_number - 1
            }
        }
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
