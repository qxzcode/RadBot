mod draws;

use std::collections::{HashMap, hash_map::Entry};
use rand::seq::SliceRandom;
use by_address::ByAddress;

use self::draws::Draws;


/// A multiset of cards.
#[derive(Debug, PartialEq, Eq)]
pub struct Cards<'ctype, CardType: ?Sized> {
    /// A mapping from card types to the number of cards of that type.
    cards: HashMap<ByAddress<&'ctype CardType>, usize>,
}

impl<'ctype, CardType: ?Sized> Cards<'ctype, CardType> {
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
                panic!("Tried to remove {n} of a card type from a Cards, but only {count} present");
            }
            *count -= n;
            if *count == 0 {
                o.remove();
            }
        } else {
            panic!("Tried to remove {n} of a card type from a Cards, but none present");
        }
    }

    /// Removes all cards of the given [`CardType`] from the [`Cards`].
    ///
    /// # Panics
    /// Panics if the [`CardType`] is not present in the [`Cards`].
    #[allow(dead_code)]
    pub fn remove_all(&mut self, card_type: &'ctype CardType) {
        if self.cards.remove(&ByAddress(card_type)).is_none() {
            panic!("Tried to remove all cards of a type from a Cards, but none present");
        }
    }

    /// Returns the number of cards in the [`Cards`], counting duplicates.
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.cards.values().sum()
    }

    /// Returns the number of unique [`CardType`]s in the [`Cards`].
    #[allow(dead_code)]
    pub fn count_unique(&self) -> usize {
        self.cards.len()
    }

    /// Returns `true` if the [`Cards`] contains no cards.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Draws (up to) `n` random cards from this [`Cards`].
    /// Returns the updated [`Cards`], and the drawn [`Cards`].
    #[allow(dead_code)]
    pub fn draw_random(&self, n: usize) -> (Cards<'ctype, CardType>, Cards<'ctype, CardType>) {
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
    #[allow(dead_code)]
    pub fn enumerate_draws(&self, n: usize) -> Draws<'ctype, CardType> {
        Draws::new(self, n)
    }

    /// Returns an iterator over the unique card types in the [`Cards`].
    /// The order of the types is not defined.
    pub fn iter_unique(&self) -> impl Iterator<Item = &'ctype CardType> + '_ {
        self.cards.keys().map(|key| key.0)
    }

    /// Returns an iterator over (`CardType`, count) pairs.
    /// The order is not defined.
    pub fn iter(&self) -> impl Iterator<Item = (&'ctype CardType, usize)> + '_ {
        self.cards.iter().map(|(key, count)| (key.0, *count))
    }
}

impl<CardType: ?Sized> Clone for Cards<'_, CardType> {
    fn clone(&self) -> Self {
        Self { cards: self.cards.clone() }
    }
}

impl<CardType: ?Sized> Default for Cards<'_, CardType> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'iter, 'ctype: 'iter, CardType: ?Sized> FromIterator<&'iter &'ctype CardType>
    for Cards<'ctype, CardType>
{
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

impl<'ctype, CardType: ?Sized> FromIterator<&'ctype CardType> for Cards<'ctype, CardType> {
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
