use super::abilities::*;
use super::player_state::Person;
use super::styles::*;
use super::{GameResult, GameView, IconEffect};

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
            abilities: vec![
                // punk (costs 1 water):
                icon_ability(1, IconEffect::GainPunk),
                // if you have a punk, damage (costs 1 water):
                ability! {
                    cost => 1;
                    can_perform(game_view) => {
                        let has_punk = game_view
                            .my_state()
                            .people()
                            .any(|person| matches!(person, Person::Punk(_)));
                        has_punk && IconEffect::Damage.can_perform(game_view)
                    };
                    perform => IconEffect::Damage;
                },
            ],
        },
        PersonType {
            name: "Sniper",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            abilities: vec![
                // damage any [opponent?] card (costs 2 water):
                // TODO
            ],
        },
        PersonType {
            name: "Vigilante",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: vec![
                // injure (costs 1 water):
                icon_ability(1, IconEffect::Injure),
            ],
        },
        PersonType {
            name: "Scout",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            abilities: vec![
                // raid (costs 1 water):
                icon_ability(1, IconEffect::Raid),
            ],
        },
    ]
}
