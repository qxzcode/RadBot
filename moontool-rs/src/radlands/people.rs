use std::collections::HashSet;
use std::fmt;

use itertools::Itertools;

use super::abilities::*;
use super::choices::*;
use super::locations::PlayLocation;
use super::styles::*;
use super::{GameResult, GameView, IconEffect};

/// Type alias for on_enter_play handler functions.
type OnEnterPlayHandler = for<'g, 'ctype> fn(
    GameView<'g, 'ctype>,
    PlayLocation,
) -> Result<ChoiceFuture<'g, 'ctype>, GameResult>;

/// Enum for identifying "special" people that require special handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialType {
    None,
    Holdout,
    Mimic,
    ArgoYesky,
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

    /// Whether this person enters play ready.
    pub enters_play_ready: bool,

    /// The special identity of this person type (if any). Used for people that require special
    /// handling elsewhere in the code.
    pub special_type: SpecialType,
}

impl fmt::Debug for PersonType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PersonType[{}]", self.name)
    }
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
    ((mut $game_view:ident, $play_loc:ident) => $on_enter_play:expr) => {
        Some(|mut $game_view, $play_loc| $on_enter_play)
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

macro_rules! enters_play_ready {
    () => {
        false
    };
    ($enters_play_ready:literal) => {
        $enters_play_ready
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
            on_enter_play($($on_enter_play_param:ident $($on_enter_play_mut:ident)?),+)
                => $on_enter_play_expr:expr,
        )?
        $(enters_play_ready: $enters_play_ready:literal,)?
        $(special_type: $special_type:tt,)?
    } => {
        PersonType {
            name: $name,
            num_in_deck: $num_in_deck,
            junk_effect: $junk_effect,
            cost: $cost,
            abilities: vec![$($ability),*],
            on_enter_play: on_enter_play!(
                $(($($on_enter_play_param $($on_enter_play_mut)?),+) => $on_enter_play_expr)?
            ),
            enters_play_ready: enters_play_ready!($($enters_play_ready)?),
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
                    let player = game_view.player;
                    Ok(game_view.destroy_own_person().then_future_chain(move |game_state, _| {
                        IconEffect::Damage.perform(game_state.view_for(player))
                    }))
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
                perform(mut game_view) => {
                    game_view.injure_all_unprotected_enemies();
                    Ok(game_view.immediate_future())
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
                Ok(game_view.restore_card())
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
                    let player = game_view.player;
                    Ok(game_view.damage_enemy().then_future(move |game_state, damaged_loc| {
                        if damaged_loc.row().is_camp() {
                            game_state.view_for(player).draw_card_into_hand()?;
                        }
                        Ok(())
                    }))
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
                perform(game_view) => Ok(game_view.damage_any_enemy().ignore_result());
            }],
        },
        person_type! {
            name: "Scientist",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            abilities: [ability! {
                description => "Discard the top 3; may use the junk effect of one";
                cost => 1;
                can_perform => true;
                perform(game_view) => {
                    // discard the top 3 cards and collect the unique junk effects that can be used
                    let junk_effects: HashSet<IconEffect> = (0..3)
                        .filter_map(|_| {
                            // draw a card, propagating any end-game condition
                            let card_type = match game_view.game_state.draw_card() {
                                Ok(card_type) => card_type,
                                Err(game_result) => return Some(Err(game_result)),
                            };

                            // discard the card
                            game_view.game_state.discard.push(card_type);

                            // return the card's junk effect (if it can be performed)
                            let effect = card_type.junk_effect();
                            if effect.can_perform(&game_view) {
                                Some(Ok(effect))
                            } else {
                                None
                            }
                        })
                        .collect::<Result<_, GameResult>>()?;

                    // ask the player which junk effect to use (if any)
                    if junk_effects.is_empty() {
                        Ok(game_view.immediate_future())
                    } else {
                        let junk_effects: Vec<IconEffect> = junk_effects.into_iter().collect();
                        Ok(IconEffectChoice::future(game_view.player, junk_effects))
                    }
                };
            }],
        },
        person_type! {
            name: "Mutant",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [ability! {
                description => "Damage and/or Restore, then damage this card";
                cost => 0;
                can_perform => true;
                perform(game_view, card_loc) => {
                    Ok(IconEffectChoice::future(game_view.player, vec![IconEffect::Damage])
                        .then_future_chain(move |_game_state, _| {
                            Ok(IconEffectChoice::future(game_view.player, vec![IconEffect::Restore])
                                .then_future(move |game_state, _| {
                                    game_state.damage_card_at(card_loc, false, true)
                                }))
                        })
                    )
                };
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
            name: "Rescue Team",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [ability! {
                description => "Return one of your people to your hand";
                cost => 0;
                can_perform => true;
                perform(game_view) => Ok(RescuePersonChoice::future(game_view.player));
            }],
            enters_play_ready: true,
        },
        person_type! {
            name: "Vanguard",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            abilities: [ability! {
                description => "Damage, then opponent does damage back to you";
                cost => 1;
                can_perform => true;
                perform(game_view) => {
                    let player = game_view.player;
                    Ok(game_view.damage_enemy().then_future_chain(move |game_state, _| {
                        let opponent_view = game_state.view_for(player.other());
                        Ok(opponent_view.damage_enemy().ignore_result())
                    }))
                };
            }],
            on_enter_play(game_view) => {
                // when this card enters play, punk
                game_view.gain_punk()
            },
        },
        person_type! {
            name: "Assassin",
            num_in_deck: 2,
            junk_effect: IconEffect::Raid,
            cost: 1,
            abilities: [ability! {
                description => "Destroy an unprotected (opponent) person";
                cost => 2;
                can_perform(game_view) => IconEffect::Injure.can_perform(game_view);
                perform(game_view) => Ok(game_view.destroy_enemy().ignore_result());
            }],
        },
        person_type! {
            name: "Pyromaniac",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [ability! {
                description => "Damage an unprotected (opponent) camp";
                cost => 1;
                can_perform(game_view) => {
                    // can perform if the opponent has an unprotected camp
                    game_view.other_state().unprotected_card_locs().any(|loc| loc.row().is_camp())
                };
                perform(game_view) => Ok(game_view.damage_unprotected_camp().ignore_result());
            }],
        },
        person_type! {
            name: "Scout",
            num_in_deck: 2,
            junk_effect: IconEffect::Water,
            cost: 1,
            abilities: [icon_ability(1, IconEffect::Raid)],
        },
        person_type! {
            name: "Wounded Soldier",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [icon_ability(1, IconEffect::Damage)],
            on_enter_play(mut game_view, play_loc) => {
                // when this card enters play, draw, then damage this card
                game_view.draw_card_into_hand()?;

                let play_loc = play_loc.for_player(game_view.player);
                game_view.game_state.damage_card_at(play_loc, false, true)
                    .expect("Damaging Wounded Soldier should not end the game");

                Ok(game_view.immediate_future())
            },
        },
        person_type! {
            name: "Muse",
            num_in_deck: 2,
            junk_effect: IconEffect::Injure,
            cost: 1,
            abilities: [icon_ability(0, IconEffect::Water)],
        },
        person_type! {
            name: "Doomsayer",
            num_in_deck: 2,
            junk_effect: IconEffect::Draw,
            cost: 1,
            abilities: [ability! {
                description => "(If opponent has an event in play) Damage";
                cost => 1;
                can_perform(game_view) => game_view.other_state().has_event();
                perform => IconEffect::Damage;
            }],
            on_enter_play(game_view) => {
                // when this card enters play, you may move all the opponent's events back 1
                if game_view.other_state().has_event() {
                    // the AI (and humans) pretty much always choose to move events back...
                    // so should this even be a choice?
                    Ok(MoveEventsChoice::future(game_view.player))
                } else {
                    Ok(game_view.immediate_future())
                }
            },
        },
        person_type! {
            name: "Exterminator",
            num_in_deck: 2,
            junk_effect: IconEffect::Draw,
            cost: 1,
            abilities: [ability! {
                description => "Destroy all damaged enemies";
                cost => 1;
                can_perform(game_view) => {
                    // can perform if the opponent has any injured people
                    game_view.other_state().people().any(|person| person.is_injured())
                };
                perform(mut game_view) => {
                    game_view.destroy_all_injured_enemies();
                    Ok(game_view.immediate_future())
                };
            }],
        },
        person_type! {
            name: "Argo Yesky",
            num_in_deck: 1,
            junk_effect: IconEffect::GainPunk,
            cost: 3,
            abilities: [icon_ability(1, IconEffect::Damage)],
            on_enter_play(game_view) => {
                // when this card enters play, punk
                game_view.gain_punk()
            },
            special_type: ArgoYesky, // Argo Yesky gives its ability to other people (when uninjured)
        },
        person_type! {
            name: "Magnus Karv",
            num_in_deck: 1,
            junk_effect: IconEffect::GainPunk,
            cost: 3,
            abilities: [ability! {
                description => "Damage all cards in one of the opponent's columns";
                cost => 2;
                can_perform => true;
                perform(game_view) => {
                    let non_empty_cols = game_view.other_state().enumerate_columns()
                        .filter(|(_, col)| !col.is_empty())
                        .map(|(col_idx, _)| col_idx)
                        .collect_vec();
                    Ok(DamageColumnChoice::future(game_view.player, non_empty_cols))
                };
            }],
        },
        // TODO: Zeto Khan
        // TODO: Karli Blaze
        // TODO: Vera Vosh
        person_type! {
            name: "Molgur Stang",
            num_in_deck: 1,
            junk_effect: IconEffect::GainPunk,
            cost: 4,
            abilities: [ability! {
                description => "Destroy any (opponent) camp";
                cost => 1;
                can_perform => true;
                perform(game_view) => Ok(game_view.destroy_enemy_camp().ignore_result());
            }],
        },
    ]
}
