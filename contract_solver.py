from typing import NamedTuple, Tuple
from collections import Counter
import random
import math

from util import *

import cppimport.import_hook
from contract_solver_stuff import *


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

    requirements = Requirements(reactors=2, shields=2)
    print(f'requirements: {requirements}')

    draw_pile, hand = get_default_deck().draw_random(5)
    # hand = RRTTS  ->  prob > 1.0
    print(f'hand: {hand.to_console_string()}  |  draw pile: {draw_pile.to_console_string()}')

    """
    # sample
    NUM_SAMP = 100000
    num_draw = min(3, len(draw_pile))
    c = Counter(
        ''.join(sorted(str(draw_pile.draw_random(num_draw)[1])))
        for _ in range(NUM_SAMP)
    )

    # analyze
    dp_c = Counter(str(draw_pile))
    for k, v in c.items():
        k_c = Counter(k)
        prob = math.prod(
            math.comb(dp_c[t], k_c[t])
            for t in dp_c.keys()  # or, equiv.: k_c.keys()
        ) / math.comb(len(draw_pile), num_draw)
        print(k, prob*NUM_SAMP, v/(prob*NUM_SAMP))
    return
    """

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
        extra_str = 'always possible'
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
