use std::cmp;
use std::hash::Hash;

use super::Cards;

/// An iterator that enumerates unique draws from a [`Cards`].
/// See [`Cards::enumerate_draws`] for details.
/// (Note: There may be room to optimize this.)
#[derive(Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Draws<CardType> {
    /// The reciprocal of the denominator in the probability calculation.
    prob_denom_recip: f64,
    /// A "stack" of states for each card type.
    states: Vec<CardTypeState<CardType>>,
    /// The current index into `states`.
    index: isize,
}

#[derive(Clone)]
struct CardTypeState<CardType> {
    card_type: CardType,
    num_in_deck: usize,
    n_remaining: usize,
    num_drawn: usize,
}

impl<CardType: Hash + Eq + Copy> Draws<CardType> {
    pub(super) fn new(cards: &Cards<CardType>, n: usize) -> Self {
        if cards.is_empty() {
            return Self {
                prob_denom_recip: 1.0, // arbitrary; will not be used
                states: Vec::new(),
                index: 0,
            };
        }

        let total_cards = cards.count();
        // only draw up to the total number of cards
        let n = cmp::min(n, total_cards);

        let prob_denom = num_integer::binomial(total_cards, n);

        Self {
            prob_denom_recip: 1.0 / (prob_denom as f64),
            states: cards.cards.iter()
                .map(|(&card_type, &count)| {
                    CardTypeState {
                        card_type,
                        num_in_deck: count,
                        n_remaining: n,
                        num_drawn: 0,
                    }
                })
                .collect(),
            index: 0,
        }
    }

    /// Constructs a result tuple from the current state of the iterator.
    /// Also calls `end_loop()` to prepare for the next iteration.
    fn make_result(&mut self) -> (Cards<CardType>, Cards<CardType>, f64) {
        let mut reduced_deck = Cards::new();
        let mut drawn_cards = Cards::new();
        let mut prob_numerator = 1.0;
        let i = self.index as usize;
        for state in &self.states[..=i] {
            drawn_cards.add(state.card_type, state.num_drawn);
            reduced_deck.add(state.card_type, state.num_in_deck - state.num_drawn);

            // note: these binomial coefficients could be computed incrementally
            // a la dynamic programming, which may(?) be faster in many(?) cases
            let b = num_integer::binomial(state.num_in_deck, state.num_drawn);
            prob_numerator *= b as f64;
        }
        for state in &self.states[i+1..] {
            reduced_deck.add(state.card_type, state.num_in_deck);
        }
        let prob = prob_numerator * self.prob_denom_recip;

        self.end_loop();
        (reduced_deck, drawn_cards, prob)
    }

    /// Helper function which advances one or more of the "recursive" loops.
    fn end_loop(&mut self) {
        // draw another of this type of card (and loop again if not done)
        while self.index >= 0 && {
            let state = &mut self.states[self.index as usize];
            state.num_drawn += 1;
            state.num_drawn > cmp::min(state.n_remaining, state.num_in_deck)
        } {
            // tried every count of this type of card; "return" up a level
            self.index -= 1;
        }
    }
}

impl<CardType: Hash + Eq + Copy> Iterator for Draws<CardType> {
    type Item = (Cards<CardType>, Cards<CardType>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.states.is_empty() {
            // handle the special case of drawing from an empty set of cards,
            // where there's only one possible draw (nothing)
            return if self.index == 0 {
                self.index = -1;
                Some((Cards::new(), Cards::new(), 1.0))
            } else {
                None
            };
        }

        while self.index >= 0 {
            let i = self.index as usize;
            let cur_state = &mut self.states[i];
            let remaining = cur_state.n_remaining - cur_state.num_drawn;
            if remaining == 0 {
                // found a valid draw set; return it
                return Some(self.make_result());
            } else {
                self.index += 1;
                if self.index >= self.states.len() as isize {
                    // went through all card types without drawing enough cards;
                    // don't recurse (just loop again)
                    self.index -= 1;
                } else {
                    // recurse to try drawing more cards of different types
                    let state = &mut self.states[self.index as usize];
                    state.n_remaining = remaining;
                    state.num_drawn = 0;
                    continue;
                }
            }

            self.end_loop();
        }

        // no more draws
        None
    }
}
