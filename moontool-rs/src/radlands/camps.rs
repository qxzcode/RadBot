use std::hash::{Hash, Hasher};

use by_address::ByAddress;
use itertools::Itertools;

use super::choices::DamageChoice;

use super::abilities::*;
use super::IconEffect;

/// A type of camp card.
pub struct CampType {
    /// The camp's name.
    pub name: &'static str,

    /// The number of cards this camp grants at the start of the game.
    pub num_initial_cards: u32,

    /// The camp's abilities.
    pub abilities: Vec<Box<dyn Ability>>,
}

// hash references by address
impl Hash for &CampType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ByAddress(*self).hash(state);
    }
}

// compare references by address
impl PartialEq for &CampType {
    fn eq(&self, other: &Self) -> bool {
        ByAddress(*self) == ByAddress(*other)
    }
}
impl Eq for &CampType {}

pub fn get_camp_types() -> Vec<CampType> {
    vec![
        CampType {
            name: "Outpost",
            num_initial_cards: 1,
            abilities: vec![
                icon_ability(2, IconEffect::Raid),
                icon_ability(2, IconEffect::Restore),
            ],
        },
        CampType {
            name: "Railgun",
            num_initial_cards: 0,
            abilities: vec![icon_ability(2, IconEffect::Damage)],
        },
        CampType {
            name: "Victory Totem",
            num_initial_cards: 1,
            abilities: vec![
                icon_ability(2, IconEffect::Injure),
                icon_ability(2, IconEffect::Raid),
            ],
        },
        CampType {
            name: "Scud Launcher",
            num_initial_cards: 0,
            abilities: vec![ability! {
                description => "Damage an opponent's card of their choice";
                cost => 1;
                can_perform => true;
                perform(game_view) => {
                    let target_locs = game_view
                        .other_state()
                        .card_locs()
                        .map(|loc| loc.for_player(game_view.player.other()))
                        .collect_vec();
                    // let damage_future = game_view.other_view_mut().choose_and_damage_card(target_locs);
                    let damage_future = DamageChoice::future(game_view.player.other(), false, target_locs);
                    Ok(damage_future.ignore_result())
                };
            }],
        },
        CampType {
            name: "Cannon",
            num_initial_cards: 1,
            // ability: damage this card, then damage (costs 1 water)
            abilities: vec![ability! {
                description => "Damage this card, then damage";
                cost => 1;
                can_perform => true;
                perform(game_view, card_loc) => {
                    game_view.game_state.damage_card_at(card_loc, false, false)?;
                    IconEffect::Damage.perform(game_view)
                };
            }],
        },
        CampType {
            name: "Garage",
            num_initial_cards: 0,
            abilities: vec![icon_ability(1, IconEffect::Raid)],
        },
    ]
}
