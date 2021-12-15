mod draws;

use std::collections::{HashMap, hash_map::Entry};
use std::fmt;
use rand::seq::SliceRandom;
use by_address::ByAddress;

use self::draws::Draws;


#[derive(Debug)]
pub struct CardType {
    /// A function that implements the behavior of playing this card.
    /// Returns the completion probability after playing this card
    /// when starting in the given state.
    pub play_func: fn() -> f64,

    /// The letter used to represent this card type.
    pub letter: char,

    /// The ANSI color escape code for console printing.
    pub color: &'static str,
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
        if n == 0 {
            return; // adding 0 cards is a no-op
        }
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
        if n == 0 {
            return; // removing 0 cards is a no-op
        }
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

impl Default for Cards<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'iter, 'ctype: 'iter> FromIterator<&'iter &'ctype CardType> for Cards<'ctype> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'iter &'ctype CardType>,
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
    where
        I: IntoIterator<Item = &'ctype CardType>,
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
