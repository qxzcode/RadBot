use super::CampType;

pub fn get_camp_types() -> Vec<CampType> {
    vec![
        CampType {
            name: "Railgun",
            num_initial_cards: 0,
            // ability: damage (costs 2 water)
        },
        CampType {
            name: "Outpost",
            num_initial_cards: 1,
            // ability: raid (costs 2 water)
            // ability: restore (costs 2 water)
        },
        CampType {
            name: "Victory Totem",
            num_initial_cards: 1,
            // ability: injure (costs 2 water)
            // ability: raid (costs 2 water)
        },
        CampType {
            name: "Scud Launcher",
            num_initial_cards: 0,
            // ability: damage an opponent's card of their choice (costs 1 water)
        },
        CampType {
            name: "Cannon",
            num_initial_cards: 1,
            // ability: damage this card, then damage (costs 1 water)
        },
        CampType {
            name: "Garage",
            num_initial_cards: 0,
            // ability: raid (costs 1 water)
        },
    ]
}
