use super::abilities::*;
use super::locations::PlayLocation;
use super::styles::*;
use super::{GameResult, GameView, IconEffect};

/// Type alias for on_enter_play handler functions.
type OnEnterPlayHandler = fn(&mut GameView, PlayLocation) -> Result<(), GameResult>;

/// Enum for identifying "special" people that require special handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialType {
    None,
    Holdout,
    Mimic,
}

/// A type of person card.
pub struct PersonType {
    /// The person's name.
    pub name: &'static str,

    /// How many of this person type are in the deck.
    pub num_in_deck: u32,

    /// The person's junk effect.
    pub junk_effect: IconEffect,

    /// The water cost to play this person.
    pub cost: u32,

    /// The person's abilities.
    pub abilities: Vec<Box<dyn Ability>>,

    /// The person's on-enter-play handler, if any.
    pub on_enter_play: Option<OnEnterPlayHandler>,

    /// The special identity of this person type (if any). Used for people that require special
    /// handling elsewhere in the code.
    pub special_type: SpecialType,
}

impl StyledName for PersonType {
    /// Returns this person's name, styled for display.
    fn styled_name(&self) -> StyledString {
        StyledString::new(self.name, PERSON_READY)
    }
}

macro_rules! on_enter_play {
    () => {
        None
    };
    (($game_view:ident) => $on_enter_play:expr) => {
        Some(|$game_view, _play_loc| $on_enter_play)
    };
    (($game_view:ident, $play_loc:ident) => $on_enter_play:expr) => {
        Some(|$game_view, $play_loc| $on_enter_play)
    };
}

macro_rules! special_type {
    () => {
        SpecialType::None
    };
    ($type:ident) => {
        SpecialType::$type
    };
}

/// Convenience macro to allow omitting certain fields with common defaults.
macro_rules! person_type {
    // basic person type with abilities
    {
        name: $name:literal,
        num_in_deck: $num_in_deck:literal,
        junk_effect: $junk_effect:expr,
        cost: $cost:literal,
        abilities: [$($ability:expr),* $(,)?],
        $(
            on_enter_play($($on_enter_play_param:ident),+) => $on_enter_play_expr:expr,
        )?
        $(special_type: $special_type:tt,)?
    } => {
        PersonType {
            name: $name,
            num_in_deck: $num_in_deck,
            junk_effect: $junk_effect,
            cost: $cost,
            abilities: vec![$($ability),*],
            on_enter_play: on_enter_play!($(($($on_enter_play_param),+) => $on_enter_play_expr)?),
            special_type: special_type!($($special_type)?),
        }
    };
}

pub fn get_person_types() -> Vec<PersonType> {
    vec![
        person_type! {
            name: "Cult Leader",
            num_in_deck: 2,
            junk_effect: IconEffect::Draw,
            cost: 1,
            abilities: [ability! {
                description => "Destroy one of your people, then damage";
                cost => 0;
                can_perform => true;
                perform(game_view) => {
                    game_view.destroy_own_person();
                    IconEffect::Damage.perform(game_view)
                };
            }],
        },
        person_type! {
            name: "Gunner",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            abilities: [ability! {
                description => "Injure all unprotected enemies";
                cost => 2;
                can_perform(game_view) => IconEffect::Injure.can_perform(game_view);
                perform(game_view) => {
                    game_view.injure_all_unprotected_enemies();
                    Ok(())
                };
            }],
        },
        person_type! {
            name: "Holdout",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 2,
            abilities: [icon_ability(1, IconEffect::Damage)],
            special_type: Holdout, // costs 0 to play in the column of a destroyed camp
        },
        person_type! {
            name: "Repair Bot",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [icon_ability(2, IconEffect::Restore)],
            on_enter_play(game_view) => {
                // when this card enters play, restore
                game_view.restore_card();
                Ok(())
            },
        },
        person_type! {
            name: "Rabble Rouser",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            abilities: [
                icon_ability(1, IconEffect::GainPunk),
                ability! {
                    description => "(If you have a punk) Damage";
                    cost => 1;
                    can_perform(game_view) => game_view.my_state().has_punk();
                    perform => IconEffect::Damage;
                },
            ],
        },
        person_type! {
            name: "Looter",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            abilities: [ability! {
                description => "Damage; if this hits a camp, draw";
                cost => 2;
                can_perform => true;
                perform(game_view) => {
                    let damaged_loc = game_view.damage_enemy()?;
                    if damaged_loc.row().is_camp() {
                        game_view.draw_card_into_hand()?;
                    }
                    Ok(())
                };
            }],
        },
        person_type! {
            name: "Mimic",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [], // mimic gets its abilities from other people
            special_type: Mimic,
        },
        person_type! {
            name: "Sniper",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            abilities: [ability! {
                description => "Damage any (opponent) card";
                cost => 2;
                can_perform => true;
                perform(game_view) => game_view.damage_any_enemy();
            }],
        },
        person_type! {
            name: "Vigilante",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [icon_ability(1, IconEffect::Injure)],
        },
        person_type! {
            name: "Scout",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            abilities: [icon_ability(1, IconEffect::Raid)],
        },
    ]
}
