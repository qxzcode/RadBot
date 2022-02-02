use super::{IconEffect, PersonType};

pub fn get_person_types() -> Vec<PersonType> {
    vec![
        PersonType {
            name: "Rabble Rouser",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            // ability: punk (costs 1 water)
            // ability: if you have a punk, damage (costs 1 water)
        },
        PersonType {
            name: "Sniper",
            num_in_deck: 2,
            junk_effect: IconEffect::Restore,
            cost: 1,
            // ability: damage any [opponent?] card (costs 2 water)
        },
        PersonType {
            name: "Vigilante",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            // ability: injure (costs 1 water)
        },
        PersonType {
            name: "Scout",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            // ability: raid (costs 1 water)
        },
    ]
}
