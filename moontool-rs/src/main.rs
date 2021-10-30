use std::collections::{HashMap, hash_map::Entry};
use std::cmp;
use std::fmt;
use rand::seq::SliceRandom;
use by_address::ByAddress;


#[derive(Debug)]
pub struct CardType {
    /// A function that implements the behavior of playing this card.
    /// Returns the completion probability after playing this card
    /// when starting in the given state.
    pub play_func: fn() -> f64,

    /// The letter used to represent this card type.
    pub letter: char,

    /// The ANSI color escape code for console printing.
    color: &'static str,
}


/// A multiset of cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cards<'ctype> {
    /// A mapping from card types to the number of cards of that type.
    cards: HashMap<ByAddress<&'ctype CardType>, usize>,
}

impl<'ctype> Cards<'ctype> {
    /// Creates a new, empty [`Cards`].
    pub fn new() -> Self {
        Self { cards: HashMap::new() }
    }

    /// Adds 1 of the given [`CardType`] to the [`Cards`].
    pub fn add_one(&mut self, card_type: &'ctype CardType) {
        self.add(card_type, 1);
    }

    /// Adds `n` of the given [`CardType`] to the [`Cards`].
    pub fn add(&mut self, card_type: &'ctype CardType, n: usize) {
        if n == 0 { return; }  // adding 0 cards is a no-op
        self.cards.entry(ByAddress(card_type))
            .and_modify(|e| *e += n)
            .or_insert(n);
    }

    /// Removes 1 of the given [`CardType`] from the [`Cards`].
    ///
    /// # Panics
    /// Panics if the [`CardType`] is not present in the [`Cards`].
    pub fn remove_one(&mut self, card_type: &'ctype CardType) {
        self.remove(card_type, 1);
    }

    /// Removes `n` of the given [`CardType`] from the [`Cards`].
    ///
    /// # Panics
    /// Panics if there are less than `n` of the given [`CardType`] in the [`Cards`].
    pub fn remove(&mut self, card_type: &'ctype CardType, n: usize) {
        if n == 0 { return; }  // removing 0 cards is a no-op
        let entry = self.cards.entry(ByAddress(card_type));
        if let Entry::Occupied(mut o) = entry {
            let count = o.get_mut();
            if *count < n {
                panic!("Tried to remove {} of {:?} from a Cards, but only {} present",
                        n, card_type, *count);
            }
            *count -= n;
            if *count == 0 {
                o.remove();
            }
        } else {
            panic!("Tried to remove {} of {:?} from a Cards, but none present",
                    n, card_type);
        }
    }

    /// Removes all cards of the given [`CardType`] from the [`Cards`].
    ///
    /// # Panics
    /// Panics if the [`CardType`] is not present in the [`Cards`].
    pub fn remove_all(&mut self, card_type: &'ctype CardType) {
        if self.cards.remove(&ByAddress(card_type)).is_none() {
            panic!("Tried to remove all {:?} from a Cards, but none present",
                    card_type);
        }
    }

    /// Returns the number of cards in the [`Cards`], counting duplicates.
    pub fn count(&self) -> usize {
        self.cards.values().sum()
    }

    /// Returns the number of unique [`CardType`]s in the [`Cards`].
    pub fn count_unique(&self) -> usize {
        self.cards.len()
    }

    /// Returns `true` if the [`Cards`] contains no cards.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Draws (up to) `n` random cards from this [`Cards`].
    /// Returns the updated [`Cards`], and the drawn [`Cards`].
    pub fn draw_random(&self, n: usize) -> (Cards<'ctype>, Cards<'ctype>) {
        // create a list of all the cards, with repetitions
        let mut card_list = Vec::new();
        for (card_type, count) in &self.cards {
            for _ in 0..*count {
                card_list.push(**card_type);
            }
        }

        if n >= card_list.len() {
            // we're drawing as many cards as we have or more, so just draw all
            return (Cards::new(), self.clone());
        }

        // shuffle and split the card list
        card_list.partial_shuffle(&mut rand::thread_rng(), n);
        let drawn = &card_list[..n];
        let rest = &card_list[n..];
        (Cards::from_iter(drawn), Cards::from_iter(rest))
    }
}

impl<'ctype> Default for Cards<'ctype> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'iter, 'ctype: 'iter> FromIterator<&'iter&'ctype CardType> for Cards<'ctype> {
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item = &'iter &'ctype CardType>
    {
        let mut cards = Self::new();
        for card_type in iter {
            cards.add_one(card_type);
        }
        cards
    }
}

impl<'ctype> FromIterator<&'ctype CardType> for Cards<'ctype> {
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item = &'ctype CardType>
    {
        let mut cards = Self::new();
        for card_type in iter {
            cards.add_one(card_type);
        }
        cards
    }
}

impl fmt::Display for Cards<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_empty() {
            write!(f, "\x1b[90m<no cards>\x1b[0m")
        } else {
            for (card_type, &count) in &self.cards {
                write!(f, "\x1b[{}m", card_type.color)?;
                for _ in 0..count {
                    write!(f, "{}", card_type.letter)?;
                }
            }
            write!(f, "\x1b[0m")
        }
    }
}


impl<'ctype> Cards<'ctype> {
    /// Returns an iterator that enumerates the possible unique draws of `n`
    /// cards from the [`Cards`] as tuples of the form
    /// `(reduced_deck, drawn_cards, probability)`.
    /// The enumeration order is not defined, and may be different even for
    /// equal [`Cards`] objects.
    ///
    /// # Examples
    /// ```
    /// let deck: Cards = ...;
    /// for (left, drawn, prob) in cards.enumerate_draws(2) {
    ///     println!("{}, {}, {}", left, drawn, prob);
    /// }
    /// ```
    pub fn enumerate_draws(&self, n: usize) -> Draws<'ctype> {
        Draws::new(self, n)
    }
}

/// An iterator that enumerates unique draws from a [`Cards`].
/// See [`Cards::enumerate_draws`] for details.
/// (Note: There may be room to optimize this.)
pub struct Draws<'ctype> {
    /// The reciprocal of the denominator in the probability calculation.
    prob_denom_recip: f64,
    /// A "stack" of states for each card type.
    states: Vec<CardTypeState<'ctype>>,
    /// The current index into `states`.
    index: isize,
}

struct CardTypeState<'ctype> {
    card_type: &'ctype CardType,
    num_in_deck: usize,
    n_remaining: usize,
    num_drawn: usize,
}

impl<'ctype> Draws<'ctype> {
    fn new(cards: &Cards<'ctype>, n: usize) -> Self {
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
                .map(|(card_type, &count)| {
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

    /// Helper function used in the next() method.
    fn end_loop(&mut self) {
        // draw another of this type of card (and loop again if not done)
        while self.index >= 0 && {
            let state = &mut self.states[self.index as usize];
            state.num_drawn += 1;
            state.num_drawn > cmp::min(state.n_remaining, state.num_in_deck)
        } {
            // tried every number of this type of card; "return" up a level
            self.index -= 1;
        }
    }
}

impl<'ctype> Iterator for Draws<'ctype> {
    type Item = (Cards<'ctype>, Cards<'ctype>, f64);

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
                let mut reduced_deck = Cards::new();
                let mut drawn = Cards::new();
                let mut prob_numerator = 1.0;
                for state in &self.states[..=i] {
                    drawn.add(state.card_type, state.num_drawn);
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
                return Some((reduced_deck, drawn, prob));
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


fn main() {
    println!("Hello, world!");
    let n_draw = 3;

    println!("Empty:");
    for (deck, drawn, prob) in Cards::new().enumerate_draws(n_draw) {
        println!("{}, {}, {}", deck, drawn, prob);
    }

    println!("Full:");
    let a = CardType { play_func: || 1.0, letter: 'R', color: "96" };
    let cards = Cards::from_iter(&[
        &a,
        &a,
        &CardType { play_func: || 1.0, letter: 'T', color: "93" },
        &CardType { play_func: || 1.0, letter: 'S', color: "92" },
    ]);
    for (deck, drawn, prob) in cards.enumerate_draws(n_draw) {
        println!("{}, {}, {}", deck, drawn, prob);
    }

    println!("Drop:");
    let iter = cards.enumerate_draws(n_draw);
    std::mem::drop(cards);
    for (deck, drawn, prob) in iter {
        println!("{}, {}, {}", deck, drawn, prob);
    }
}
