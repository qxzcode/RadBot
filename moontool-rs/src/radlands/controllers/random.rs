use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::radlands::*;

pub struct RandomController;

impl PlayerController for RandomController {
    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        actions: &'a [Action<'ctype>],
    ) -> &'a Action<'ctype> {
        let mut rng = thread_rng();
        let chosen_action = actions
            .choose(&mut rng)
            .expect("choose_action called with empty actions list");
        println!("{BOLD}RandomController chose action:{RESET} {}", chosen_action.format(game_view));
        chosen_action
    }

    fn choose_play_location<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
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

    fn choose_card_to_damage<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        target_locs: &[CardLocation],
    ) -> CardLocation {
        let mut rng = thread_rng();
        let chosen_target = target_locs
            .choose(&mut rng)
            .expect("choose_card_to_damage called with empty target_locs list");
        println!("{BOLD}RandomController chose damage target:{RESET} {chosen_target:?}");
        *chosen_target
    }

    fn choose_card_to_restore<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        target_locs: &[PlayerCardLocation],
    ) -> PlayerCardLocation {
        let mut rng = thread_rng();
        let chosen_target = target_locs
            .choose(&mut rng)
            .expect("choose_card_to_restore called with empty target_locs list");
        println!("{BOLD}RandomController chose restore target:{RESET} {chosen_target:?}");
        *chosen_target
    }
}
