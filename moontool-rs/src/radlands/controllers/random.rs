use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::radlands::choices::*;
use crate::radlands::*;

pub struct RandomController {
    pub quiet: bool,
}

impl PlayerController for RandomController {
    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        _choice: &ActionChoice<'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype> {
        let mut rng = thread_rng();
        let chosen_action = actions
            .choose(&mut rng)
            .expect("choose_action called with empty actions list");
        if !self.quiet {
            println!(
                "{BOLD}RandomController chose action:{RESET} {}",
                chosen_action.format(game_view)
            );
        }
        chosen_action
    }

    fn choose_play_location<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        _choice: &PlayChoice<'ctype>,
        _person: &Person<'ctype>,
        locations: &[PlayLocation],
    ) -> PlayLocation {
        let mut rng = thread_rng();
        let chosen_location = locations
            .choose(&mut rng)
            .expect("choose_play_location called with empty locations list");
        if !self.quiet {
            println!("{BOLD}RandomController chose location:{RESET} {chosen_location:?}");
        }
        *chosen_location
    }

    fn choose_card_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        _choice: &DamageChoice<'ctype>,
        destroy: bool,
        target_locs: &[CardLocation],
    ) -> CardLocation {
        let mut rng = thread_rng();
        let chosen_target = target_locs
            .choose(&mut rng)
            .expect("choose_card_to_damage called with empty target_locs list");
        let verb = if destroy { "destroy" } else { "damage" };
        if !self.quiet {
            println!("{BOLD}RandomController chose {verb} target:{RESET} {chosen_target:?}");
        }
        *chosen_target
    }

    fn choose_card_to_restore<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        _choice: &RestoreChoice<'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation {
        let mut rng = thread_rng();
        let chosen_target = target_locs
            .choose(&mut rng)
            .expect("choose_card_to_restore called with empty target_locs list");
        if !self.quiet {
            println!("{BOLD}RandomController chose restore target:{RESET} {chosen_target:?}");
        }
        *chosen_target
    }

    fn choose_icon_effect<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        _choice: &IconEffectChoice<'ctype>,
        icon_effects: &[IconEffect],
    ) -> Option<IconEffect> {
        let mut rng = thread_rng();
        let none_probability = 1.0 / ((icon_effects.len() + 1) as f64);
        let chosen_icon_effect = if rng.gen_bool(none_probability) {
            // choose not to perform an icon effect
            None
        } else {
            // choose a random icon effect from the list
            let effect = icon_effects
                .choose(&mut rng)
                .expect("choose_icon_effect called with empty icon_effects list");
            Some(*effect)
        };
        if !self.quiet {
            println!("{BOLD}RandomController chose icon effect:{RESET} {chosen_icon_effect:?}");
        }
        chosen_icon_effect
    }

    fn choose_to_move_events<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        _choice: &MoveEventsChoice<'ctype>,
    ) -> bool {
        let mut rng = thread_rng();
        let move_events = rng.gen();
        if !self.quiet {
            println!(
                "{BOLD}RandomController chose to move events back:{RESET} {}",
                if move_events { "yes" } else { "no" },
            );
        }
        move_events
    }

    fn choose_column_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        choice: &DamageColumnChoice<'ctype>,
    ) -> ColumnIndex {
        let mut rng = thread_rng();
        let chosen_column = choice
            .columns()
            .choose(&mut rng)
            .expect("choose_column_to_damage called with empty columns list");
        if !self.quiet {
            println!("{BOLD}RandomController chose column to damage:{RESET} {chosen_column:?}");
        }
        *chosen_column
    }
}
