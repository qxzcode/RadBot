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
    ) -> IconEffect {
        let mut rng = thread_rng();
        let chosen_icon_effect = icon_effects
            .choose(&mut rng)
            .expect("choose_icon_effect called with empty icon_effects list");
        if !self.quiet {
            println!("{BOLD}RandomController chose icon effect:{RESET} {chosen_icon_effect:?}");
        }
        *chosen_icon_effect
    }
}
