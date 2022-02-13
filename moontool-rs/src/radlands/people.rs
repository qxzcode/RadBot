use super::abilities::{Ability, IconAbility};
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
            name: "Rabble Rouser",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            // ability: punk (costs 1 water)
            // ability: if you have a punk, damage (costs 1 water)
            abilities: vec![
                Box::new(IconAbility {
                    effect: IconEffect::GainPunk,
                    cost: 1,
                }),
                Box::new(IconAbility {
                    // TODO: only if you have a punk
                    effect: IconEffect::Damage,
                    cost: 1,
                }),
            ],
        },
        PersonType {
            name: "Sniper",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            // ability: damage any [opponent?] card (costs 2 water)
            abilities: vec![], // TODO
        },
        PersonType {
            name: "Vigilante",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            // ability: injure (costs 1 water)
            abilities: vec![
                Box::new(IconAbility {
                    effect: IconEffect::Injure,
                    cost: 1,
                }),
            ],
        },
        PersonType {
            name: "Scout",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            // ability: raid (costs 1 water)
            abilities: vec![
                Box::new(IconAbility {
                    effect: IconEffect::Raid,
                    cost: 1,
                }),
            ],
        },
    ]
}
