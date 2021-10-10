from typing import NamedTuple, Tuple
from collections import Counter
import random
import math

from util import *

import cppimport.import_hook
from contract_solver_stuff import *


CONTRACTS = [
    Contract(
        'Abandoned Vessel',
        Contract.EXPLORE,
        Rewards(prestige=1, credits=4, cards=1),
        Requirements(reactors=3, damage=3),
        hazard_dice=2,
    ),
    Contract(
        'Derelict Planet',
        Contract.EXPLORE,
        Rewards(prestige=3, credits=8),
        Requirements(reactors=5, crew=3, thrusters=2),
        hazard_dice=2,
    ),
    Contract(
        'Reactor Failure',
        Contract.RESCUE,
        Rewards(prestige=0, credits=3),
        Requirements(shields=1, reactors=1),
        hazard_dice=0,
    ),
    Contract(
        'Supernova Escape',
        Contract.RESCUE,
        Rewards(prestige=1, credits=3),
        Requirements(shields=2, thrusters=1),
        hazard_dice=1,
    ),
    Contract(
        'Asteroid Field',
        Contract.EXPLORE,
        Rewards(prestige=2, credits=8),
        Requirements(reactors=4, crew=3),
        hazard_dice=2,
    ),
    Contract(
        'Icarus Run',
        Contract.RESCUE,
        Rewards(prestige=2, credits=8),
        Requirements(shields=3, thrusters=3),
        hazard_dice=2,
    ),
    Contract(
        'Space Anomaly',
        Contract.EXPLORE,
        Rewards(prestige=0, credits=3),
        Requirements(reactors=1, damage=1),
        hazard_dice=0,
    ),
    Contract(
        'Gauntlet Run',
        Contract.DELIVERY,
        Rewards(prestige=3, cards=2),
        Requirements(thrusters=4, damage=4),
        hazard_dice=2,
    ),
    Contract(
        'Nova Bloom',
        Contract.EXPLORE,
        Rewards(prestige=3, credits=7),
        Requirements(reactors=5, shields=3),
        hazard_dice=3,
    ),

    Contract(
        'Decoy Target',
        Contract.RESCUE,
        Rewards(prestige=3, cards=3),
        Requirements(shields=4, thrusters=4),
        hazard_dice=3,
    ),
    Contract(
        'Kill Slavers',
        Contract.KILL,
        Rewards(prestige=0, credits=4),
        Requirements(damage=1, thrusters=1),
        hazard_dice=0,
    ),
    Contract(
        'Refugee Crisis',
        Contract.DELIVERY,
        Rewards(prestige=2, credits=7),
        Requirements(thrusters=3, crew=2),
        hazard_dice=2,
    ),
    Contract(
        'Emergency Meds',
        Contract.DELIVERY,
        Rewards(prestige=3, credits=8),
        Requirements(thrusters=4, damage=4, reactors=3),
        hazard_dice=2,
    ),
    Contract(
        'Elite Squadron',
        Contract.KILL,
        Rewards(prestige=4, credits=6, cards=1),
        Requirements(damage=8, reactors=4, shields=3),
        hazard_dice=3,
    ),
    Contract(
        'Resistance Leader',
        Contract.RESCUE,
        Rewards(prestige=4, credits=6),
        Requirements(shields=4, thrusters=2, crew=2),
        hazard_dice=3,
    ),
    Contract(
        'Core World Ace',
        Contract.KILL,
        Rewards(prestige=1, credits=5, cards=1),
        Requirements(damage=5),
        hazard_dice=1,
    ),
    Contract(
        'Prison Moon',
        Contract.RESCUE,
        Rewards(prestige=5, credits=10),
        Requirements(shields=5, thrusters=4, damage=2),
        hazard_dice=4,
    ),
    Contract(
        'Black Hole',
        Contract.EXPLORE,
        Rewards(prestige=5, credits=12),
        Requirements(crew=5, reactors=4, thrusters=4),
        hazard_dice=4,
    ),

    Contract(
        'Boarding Action',
        Contract.EXPLORE,
        Rewards(prestige=4, cards=2),
        Requirements(crew=4, damage=5),
        hazard_dice=3,
    ),
    Contract(
        'Escape Pods',
        Contract.RESCUE,
        Rewards(prestige=2, credits=7),
        Requirements(shields=3, damage=3),
        hazard_dice=2,
    ),
    Contract(
        'Transport Rescue',
        Contract.RESCUE,
        Rewards(prestige=1, credits=3),
        Requirements(shields=2, crew=1),
        hazard_dice=1,
    ),
    Contract(
        'Munitions Stockpile',
        Contract.DELIVERY,
        Rewards(prestige=3, credits=7),
        Requirements(thrusters=4, shields=3),
        hazard_dice=2,
    ),
    Contract(
        'Bomber Screen',
        Contract.KILL,
        Rewards(prestige=3, credits=9),
        Requirements(damage=6, thrusters=3),
        hazard_dice=3,
    ),
    Contract(
        'Assault on Vilonia',
        Contract.KILL,
        Rewards(prestige=3, credits=5, cards=1),
        Requirements(damage=8),
        hazard_dice=2,
    ),
    Contract(
        'Scout Cruiser',
        Contract.KILL,
        Rewards(prestige=3, credits=6),
        Requirements(damage=5, shields=2),
        hazard_dice=3,
    ),
    Contract(
        'First Contact',
        Contract.EXPLORE,
        Rewards(prestige=3, cards=2),
        Requirements(reactors=5, shields=3),
        hazard_dice=2,
    ),
    Contract(
        'Bounty Hunters',
        Contract.KILL,
        Rewards(prestige=3, credits=6),
        Requirements(damage=6, crew=2),
        hazard_dice=3,
    ),

    Contract(
        'Martial Law',
        Contract.RESCUE,
        Rewards(prestige=1, credits=4, cards=1),
        Requirements(shields=2, crew=2),
        hazard_dice=2,
    ),
    Contract(
        'Blockade Run',
        Contract.DELIVERY,
        Rewards(prestige=0, credits=3),
        Requirements(thrusters=1, shields=1),
        hazard_dice=0,
    ),
    Contract(
        'Probe Recovery',
        Contract.EXPLORE,
        Rewards(prestige=1, credits=2, cards=1),
        Requirements(reactors=3, thrusters=2),
        hazard_dice=1,
    ),
    Contract(
        'Envoy in Distress',
        Contract.RESCUE,
        Rewards(prestige=1, credits=2, cards=1),
        Requirements(shields=2, damage=2),
        hazard_dice=2,
    ),
    Contract(
        'Stim Run',
        Contract.DELIVERY,
        Rewards(prestige=1, credits=2),
        Requirements(thrusters=2, reactors=1),
        hazard_dice=1,
    ),
    Contract(
        'Proof of Life',
        Contract.DELIVERY,
        Rewards(prestige=3, credits=4),
        Requirements(thrusters=4, reactors=4),
        hazard_dice=2,
    ),
    Contract(
        'Pirate Treasure',
        Contract.EXPLORE,
        Rewards(prestige=1, credits=2),
        Requirements(reactors=2, shields=1),
        hazard_dice=1,
    ),
    Contract(
        'Ancient Ruins',
        Contract.EXPLORE,
        Rewards(prestige=2, credits=7),
        Requirements(reactors=4, thrusters=4),
        hazard_dice=2,
    ),
    Contract(
        'Rival Pirate Gang',
        Contract.KILL,
        Rewards(prestige=1, credits=3),
        Requirements(damage=2, shields=1),
        hazard_dice=1,
    ),

    Contract(
        'Distress Beacon',
        Contract.EXPLORE,
        Rewards(prestige=1, credits=3),
        Requirements(reactors=3, crew=1),
        hazard_dice=1,
    ),
    Contract(
        'Fuel Shortage',
        Contract.DELIVERY,
        Rewards(prestige=1, credits=3),
        Requirements(thrusters=2, damage=2),
        hazard_dice=1,
    ),
    Contract(
        'Negotiation Insurance',
        Contract.DELIVERY,
        Rewards(prestige=1, credits=2, cards=1),
        Requirements(thrusters=3, damage=1),
        hazard_dice=2,
    ),
    Contract(
        'Focused Fire',
        Contract.KILL,
        Rewards(prestige=3, cards=3),
        Requirements(damage=6, reactors=4),
        hazard_dice=3,
    ),
    Contract(
        'Claim Bounty',
        Contract.KILL,
        Rewards(prestige=1, credits=3),
        Requirements(damage=3, reactors=2),
        hazard_dice=1,
    ),
    Contract(
        'Royal Cargo',
        Contract.DELIVERY,
        Rewards(prestige=5, credits=10),
        Requirements(thrusters=5, damage=5, crew=2),
        hazard_dice=4,
    ),
    Contract(
        "Admiral's Flagship",
        Contract.KILL,
        Rewards(prestige=5, credits=11, cards=1),
        Requirements(damage=8, shields=5, reactors=5),
        hazard_dice=4,
    ),
    Contract(
        'Escort Duty',
        Contract.DELIVERY,
        Rewards(prestige=1, credits=2, cards=1),
        Requirements(thrusters=3, crew=1),
        hazard_dice=1,
    ),
    Contract(
        'Cryogenic Pods',
        Contract.RESCUE,
        Rewards(prestige=3, credits=7),
        Requirements(shields=4, reactors=4),
        hazard_dice=3,
    ),
]


def main():
    """
    def binomial_coefficient(n: int, k: int) -> int:
        if k < 0 or k > n:
            return 0
        if k == 0 or k == n:
            return 1
        # k = min(k, n - k)  # Take advantage of symmetry
        c = 1
        maxv = 0
        for i in range(k):
            maxv = max(maxv, c*(n-i))
            c = c * (n - i) // (i + 1)
        return c, maxv
        
    for n in range(2, 100):
        for k in range(1, n):
            c, maxv = binomial_coefficient(n, k)
            # print(maxv)
            if maxv > 18446744073709551615:
                print('exceeded with n =', n, 'k =', k)
        if maxv > 18446744073709551615:
            break
    quit()
    """

    deck = get_default_deck()
    # deck.add(REACTOR, n=2)
    # deck.add(THRUSTER, n=2)
    # deck.add(SHIELD, n=1)
    # deck.add(DAMAGE, n=1)
    # deck.remove(MISS)
    print(f'deck: {deck.to_console_string()}')
    draw_pile, hand = deck.draw_random(5)
    # hand = RRTTS  ->  prob > 1.0
    print(f'hand: {hand.to_console_string()}  |  draw pile: {draw_pile.to_console_string()}')

    #########

    solver = Solver()
    def get_expected_credits(contract):
        draw_pile, hand = deck.draw_random(5)
        start_state = State(
            actions=1,
            hand=hand,
            draw_pile=draw_pile,
            requirements=contract.requirements,
        )
        prob = solver.get_completion_probability(start_state)
        return prob * contract.rewards.credits

    EASY_CONTRACTS = [c for c in CONTRACTS if c.hazard_dice <= 2]
    with time_block('solve'):
        total_credits = 0
        N = 100000
        for _ in range(N):
            best = max([
                get_expected_credits(contract)
                for contract in random.sample(EASY_CONTRACTS, k=8)
            ])
            total_credits += best
    print(f'expected credits: {total_credits/N}')
    print(f'[explored {solver.explored_states_count()-N*8} non-root states]')
    quit()

    #########

    requirements = Requirements(reactors=2, shields=2, thrusters=1)
    requirements = random.choice(CONTRACTS).requirements
    print(f'requirements: {requirements.to_string(color=True)}')

    start_state = State(
        actions=1,
        hand=hand,
        draw_pile=draw_pile,
        requirements=requirements,
    )
    solver = Solver()

    with time_block('solve'):
        prob = solver.get_completion_probability(start_state)
    print(f'[explored {solver.explored_states_count()} states]')

    if prob == 0:
        extra_str = 'impossible'
    elif math.isclose(prob, 1, abs_tol=1e-6):  # allow for rounding error
        extra_str = 'guaranteed possible'
    else:
        in_n = 1 / prob
        if math.isclose(in_n, round(in_n), abs_tol=1e-6):
            extra_str = f'{in_n:.0f}'
        else:
            extra_str = f'{in_n:.1f}'
        extra_str = '1 in ' + extra_str
        # extra_str = prob.as_integer_ratio()
    print(f'probability of being able to meet requirements: {prob:.2%} ({extra_str})')

if __name__ == "__main__":
    main()
