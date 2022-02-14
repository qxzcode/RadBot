use super::abilities::*;
use super::styles::*;
use super::IconEffect;

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
    // TODO: traits
}

impl StyledName for PersonType {
    /// Returns this person's name, styled for display.
    fn styled_name(&self) -> StyledString {
        StyledString::new(self.name, PERSON_READY)
    }
}

pub fn get_person_types() -> Vec<PersonType> {
    vec![
        PersonType {
            name: "Cult Leader",
            num_in_deck: 2,
            junk_effect: IconEffect::Draw,
            cost: 1,
            abilities: vec![ability! {
                description => "Destroy one of your people, then damage";
                cost => 0;
                can_perform => true;
                perform(game_view) => {
                    game_view.destroy_own_person();
                    IconEffect::Damage.perform(game_view)
                };
            }],
        },
        PersonType {
            name: "Gunner",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            abilities: vec![ability! {
                description => "Injure all unprotected enemies";
                cost => 2;
                can_perform(game_view) => IconEffect::Injure.can_perform(game_view);
                perform(game_view) => {
                    game_view.injure_all_unprotected_enemies();
                    Ok(())
                };
            }],
        },
        PersonType {
            name: "Rabble Rouser",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            abilities: vec![
                icon_ability(1, IconEffect::GainPunk),
                ability! {
                    description => "(If you have a punk) Damage";
                    cost => 1;
                    can_perform(game_view) => game_view.my_state().has_punk();
                    perform => IconEffect::Damage;
                },
            ],
        },
        PersonType {
            name: "Sniper",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            abilities: vec![ability! {
                description => "Damage any (opponent) card";
                cost => 2;
                can_perform => true;
                perform(game_view) => game_view.damage_any_enemy();
            }],
        },
        PersonType {
            name: "Vigilante",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: vec![icon_ability(1, IconEffect::Injure)],
        },
        PersonType {
            name: "Scout",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            abilities: vec![icon_ability(1, IconEffect::Raid)],
        },
    ]
}
