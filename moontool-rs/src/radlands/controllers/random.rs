use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::radlands::choices::*;
use crate::radlands::*;

pub struct RandomController {
    pub quiet: bool,
}

macro_rules! print_choice {
    ($self:ident, $phrase:expr, $choice:expr, :?) => {
        if !$self.quiet {
            println!(
                "{}RandomController chose {}:{} {:?}",
                BOLD, $phrase, RESET, $choice
            );
        }
    };
    ($self:ident, $phrase:expr, $choice:expr $(,)?) => {
        if !$self.quiet {
            println!(
                "{}RandomController chose {}:{} {}",
                BOLD, $phrase, RESET, $choice
            );
        }
    };
}

macro_rules! random_choose_impl {
    {
        $name:ident($game_view:ident, $choice:ident: $ChoiceType:ty) -> $ReturnType:ty,
        $options:expr, $phrase:expr
    } => {
        fn $name<'a, 'v, 'g: 'v, 'ctype: 'g>(
            &self,
            $game_view: &'v GameView<'g, 'ctype>,
            $choice: &'a $ChoiceType,
        ) -> $ReturnType {
            let chosen_option = *$options
                .choose(&mut thread_rng())
                .expect(concat!(
                    stringify!($name), " called with empty options list"
                ));
            print_choice!(self, $phrase, chosen_option, :?);
            chosen_option
        }
    };
    (
        $name:ident($choice:ident: $ChoiceType:ty) -> $ReturnType:ty,
        $options:expr, $phrase:expr
    ) => {
        random_choose_impl! {
            $name(_game_view, $choice: $ChoiceType) -> $ReturnType,
            $options, $phrase
        }
    };
}

impl PlayerController for RandomController {
    fn choose_action<'a, 'v, 'g: 'v, 'ctype: 'g>(
        &self,
        game_view: &'v GameView<'g, 'ctype>,
        choice: &'a ActionChoice<'ctype>,
    ) -> &'a Action<'ctype> {
        let chosen_action = choice
            .actions()
            .choose(&mut thread_rng())
            .expect("choose_action called with empty actions list");
        print_choice!(self, "action", chosen_action.format(game_view));
        chosen_action
    }

    random_choose_impl! {
        choose_play_location(choice: PlayChoice<'ctype>) -> PlayLocation,
        choice.locations(), "play location"
    }
    random_choose_impl! {
        choose_card_to_damage(choice: DamageChoice<'ctype>) -> CardLocation,
        choice.locations(),
        if choice.destroy() { "destroy target" } else { "damage target" }
    }
    random_choose_impl! {
        choose_card_to_restore(choice: RestoreChoice<'ctype>) -> PlayerCardLocation,
        choice.locations(), "restore target"
    }

    fn choose_icon_effect<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        choice: &IconEffectChoice<'ctype>,
    ) -> Option<IconEffect> {
        let icon_effects = choice.icon_effects();

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
        print_choice!(self, "icon effect", chosen_icon_effect, :?);
        chosen_icon_effect
    }

    random_choose_impl! {
        choose_person_to_rescue(game_view, _choice: RescuePersonChoice<'ctype>) -> PlayLocation,
        game_view.my_state().person_locs().collect_vec(), "rescue target"
    }

    fn choose_to_move_events<'v, 'g: 'v, 'ctype: 'g>(
        &self,
        _game_view: &'v GameView<'g, 'ctype>,
        _choice: &MoveEventsChoice<'ctype>,
    ) -> bool {
        let move_events = thread_rng().gen();
        print_choice!(
            self,
            "to move events back",
            if move_events { "yes" } else { "no" },
        );
        move_events
    }

    random_choose_impl! {
        choose_column_to_damage(choice: DamageColumnChoice<'ctype>) -> ColumnIndex,
        choice.columns(), "column to damage"
    }
}
