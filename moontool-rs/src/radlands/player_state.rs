use crate::cards::Cards;

use super::people::SpecialType;
use super::*;

/// Represents the state of a player's board and hand.
#[derive(Clone)]
pub struct PlayerState<'ctype> {
    /// The cards in the player's hand, not including Water Silo.
    pub hand: Cards<PersonOrEventType<'ctype>>,

    /// When it is not this player's turn, whether this player has Water Silo
    /// in their hand. (They are assumed to not have it in their hand when it
    /// *is* this player's turn.)
    pub has_water_silo: bool,

    /// The three columns of the player's board.
    pub columns: [CardColumn<'ctype>; 3],

    /// The three event slots of the player's board.
    pub events: [Option<&'ctype (dyn EventType + 'ctype)>; 3],
}

impl<'v, 'g: 'v, 'ctype: 'g> PlayerState<'ctype> {
    /// Creates a new `PlayerState` with the given camps, drawing an initial
    /// hand from the given deck.
    pub fn new(camps: &[&'ctype CampType], deck: &mut Vec<PersonOrEventType<'ctype>>) -> Self {
        // determine the number of starting cards from the set of camps
        assert_eq!(camps.len(), 3);
        let hand_size: usize = camps.iter().map(|c| c.num_initial_cards as usize).sum();

        // draw the top hand_size cards from the deck
        let deck_cut_index = deck.len() - hand_size;
        let hand = Cards::from_iter(deck.drain(deck_cut_index..));

        PlayerState {
            hand,
            has_water_silo: false,
            columns: [
                CardColumn::new(camps[0]),
                CardColumn::new(camps[1]),
                CardColumn::new(camps[2]),
            ],
            events: [None, None, None],
        }
    }

    /// Returns the column at the given index.
    pub fn column(&self, index: ColumnIndex) -> &CardColumn<'ctype> {
        &self.columns[index.as_usize()]
    }

    /// Returns the column at the given index as mutable.
    pub fn column_mut(&mut self, index: ColumnIndex) -> &mut CardColumn<'ctype> {
        &mut self.columns[index.as_usize()]
    }

    /// Returns the person slot at the given location.
    pub fn person_slot(&self, loc: PlayLocation) -> Option<&Person<'ctype>> {
        self.column(loc.column()).person_slot(loc.row())
    }

    /// Returns the person at the given location as mutable.
    pub fn person_mut_slot(&mut self, loc: PlayLocation) -> Option<&mut Person<'ctype>> {
        self.column_mut(loc.column()).person_mut_slot(loc.row())
    }

    /// Returns the person slot at the given location as mutable.
    pub fn person_slot_mut(&mut self, loc: PlayLocation) -> &mut Option<Person<'ctype>> {
        self.column_mut(loc.column()).person_slot_mut(loc.row())
    }

    /// Returns whether this player can use the raid effect to play or advance
    /// their Raiders event.
    pub fn can_raid(&self) -> bool {
        // search for the Raiders event in the event queue
        for i in 0..self.events.len() {
            if matches!(self.events[i], Some(event) if event.as_raiders().is_some()) {
                // found the raiders event
                if i == 0 {
                    // it's the first event, so the raid effect would resolve it
                    return true;
                } else {
                    // it's not the first event; the raid effect can only advance it if there is
                    // not an event directly in front of it
                    return self.events[i - 1].is_none();
                }
            }
        }

        // if we get here, the raiders event was not found in the event queue;
        // the raid effect can only be used if there is a free event slot for it
        self.can_play_event(RaidersEvent.resolve_turns())
    }

    /// Returns whether this player can play an event that resolves in the given number of turns.
    pub fn can_play_event(&self, resolve_turns: u8) -> bool {
        if resolve_turns == 0 {
            // immediately-resolving events are always allowed
            true
        } else {
            // other events can only be played if there is a free event slot on or after their
            // initial slot
            let initial_slot = resolve_turns - 1;
            self.events[initial_slot as usize..]
                .iter()
                .any(|slot| slot.is_none())
        }
    }

    /// Returns whether this player has at least one event in their queue.
    pub fn has_event(&self) -> bool {
        self.events.iter().any(|slot| slot.is_some())
    }

    /// Moves all events in the player's queue back one slot (if possible for each event).
    pub fn move_events_back(&mut self) {
        for i in (0..self.events.len() - 1).rev() {
            if self.events[i].is_some() && self.events[i + 1].is_none() {
                // move the event from the current slot to the next slot
                self.events[i + 1] = self.events[i].take();
            }
        }
    }

    /// Damages or destroys the camp in the given column.
    /// If `destroy` is true, the camp is always destroyed; otherwise, it is damaged.
    /// Returns true if this player has no camps remaining.
    #[must_use = "if this returns true, the game must immediately end with this player losing"]
    pub fn damage_camp_at(&mut self, column_index: ColumnIndex, destroy: bool) -> bool {
        self.column_mut(column_index).camp.damage(destroy);
        self.columns.iter().all(|c| c.camp.is_destroyed())
    }

    /// Restores the card at the given location.
    /// Panics if there is no card there.
    pub fn restore_card_at(&mut self, loc: PlayerCardLocation) {
        let column = self.column_mut(loc.column());
        match loc.row().to_person_index() {
            Ok(person_row_index) => column.person_slots[person_row_index.as_usize()]
                .as_mut()
                .expect("Tried to restore a location that does not contain a card")
                .restore(),
            Err(()) => column.camp.restore(),
        }
    }

    /// Removes and returns the Person at the given location, shifting any person in front of it back.
    /// Panics if there is no person at the location.
    pub fn remove_person_at(&mut self, loc: PlayLocation) -> Person<'ctype> {
        // remove the person from its slot
        let person = self
            .person_slot_mut(loc)
            .take()
            .expect("Tried to remove a person at an empty location");

        // shift any person in front of it back
        if loc.row() == 0.into() {
            let column = self.column_mut(loc.column());
            column.person_slots[0] = column.person_slots[1].take();
        }

        // return the removed person
        person
    }

    /// Returns whether this player has an empty person slot.
    pub fn has_empty_person_slot(&self) -> bool {
        self.columns
            .iter()
            .flat_map(|col| &col.person_slots)
            .any(|slot| slot.is_none())
    }

    /// Returns whether this player has an empty person slot in a column where
    /// `column.camp.is_destroyed() == camp_destroyed`. This is used to determine
    /// valid locations to play the person "Holdout" for different costs.
    pub fn has_empty_holdout_slot(&self, camp_destroyed: bool) -> bool {
        self.columns
            .iter()
            .filter(|col| col.camp.is_destroyed() == camp_destroyed)
            .flat_map(|col| &col.person_slots)
            .any(|slot| slot.is_none())
    }

    /// Returns whether this player has a punk on their board.
    pub fn has_punk(&self) -> bool {
        self.people()
            .any(|person| matches!(person, Person::Punk { .. }))
    }

    /// Returns whether this player has any damaged cards that they can restore.
    pub fn has_restorable_card(&self) -> bool {
        self.columns.iter().any(|col| col.has_restorable_card())
    }

    /// Returns an iterator over the locations of any damaged cards that this player can restore.
    pub fn restorable_card_locs(&self) -> impl Iterator<Item = PlayerCardLocation> + '_ {
        self.enumerate_columns().flat_map(|(col_index, col)| {
            col.restorable_card_rows()
                .map(move |row_index| PlayerCardLocation::new(col_index, row_index))
        })
    }

    /// Returns an iterator over the people on this player's board.
    pub fn people(&self) -> impl Iterator<Item = &Person<'ctype>> {
        self.columns.iter().flat_map(|col| col.people())
    }

    /// Returns an iterator over the locations of this player's people.
    pub fn person_locs(&self) -> impl Iterator<Item = PlayLocation> + '_ {
        self.enumerate_people().map(|(loc, _)| loc)
    }

    /// Returns whether this player has a person of the given SpecialType that is uninjured (i.e.
    /// whose trait is active).
    pub fn has_special_person(&self, special_type: SpecialType) -> bool {
        self.people().any(|person| {
            matches!(person,
                Person::NonPunk { person_type, .. } if person_type.special_type == special_type
            )
        })
    }

    /// Returns an iterator over the locations of this player's cards (people
    /// and non-destroyed camps).
    pub fn card_locs(&self) -> impl Iterator<Item = PlayerCardLocation> + '_ {
        self.enumerate_columns().flat_map(|(col_index, col)| {
            col.card_rows()
                .map(move |row_index| PlayerCardLocation::new(col_index, row_index))
        })
    }

    /// Returns an iterator over the locations of this player's unprotected cards.
    pub fn unprotected_card_locs(&self) -> impl Iterator<Item = PlayerCardLocation> + '_ {
        self.enumerate_columns().filter_map(|(col_index, col)| {
            col.frontmost_card_row()
                .map(move |row_index| PlayerCardLocation::new(col_index, row_index))
        })
    }

    /// Returns an iterator over the locations of this player's unprotected people.
    pub fn unprotected_person_locs(&self) -> impl Iterator<Item = PlayerCardLocation> + '_ {
        self.enumerate_columns().filter_map(|(col_index, col)| {
            col.frontmost_person_row()
                .map(move |row_index| PlayerCardLocation::new(col_index, row_index.into()))
        })
    }

    /// Returns an iterator that enumerates the columns of this player's board with strongly-typed
    /// column indices.
    pub fn enumerate_columns(
        &self,
    ) -> impl DoubleEndedIterator<Item = (ColumnIndex, &CardColumn<'ctype>)> + '_ {
        self.columns
            .iter()
            .enumerate()
            .map(|(col_index, col)| (col_index.into(), col))
    }

    /// Returns an iterator that enumerates the camps of this player's board (both destroyed
    /// and non-destroyed) with strongly-typed locations.
    pub fn enumerate_camps(
        &self,
    ) -> impl DoubleEndedIterator<Item = (PlayerCardLocation, &Camp<'ctype>)> + '_ {
        self.enumerate_columns().map(|(col_index, col)| {
            (
                PlayerCardLocation::new(col_index, CardRowIndex::camp()),
                &col.camp,
            )
        })
    }

    /// Returns an iterator that enumerates the people of this player's board with strongly-typed
    /// locations.
    pub fn enumerate_people(
        &self,
    ) -> impl DoubleEndedIterator<Item = (PlayLocation, &Person<'ctype>)> + '_ {
        self.enumerate_columns().flat_map(|(col_index, col)| {
            col.enumerate_people()
                .map(move |(row_index, person)| (PlayLocation::new(col_index, row_index), person))
        })
    }

    /// Returns the nth person (and its location) on this player's board, by some arbitrary
    /// but stable ordering.
    ///
    /// Panics if `n` is greater or equal to the number of people this player has.
    pub fn nth_person(&self, n: usize) -> (PlayLocation, &Person<'ctype>) {
        self.enumerate_people()
            .nth(n)
            .expect("nth_person: n is too large")
    }

    /// Returns the actions that this player can take given a view for them.
    pub fn actions(&self, game_view: &'v GameView<'g, 'ctype>) -> Vec<Action<'ctype>> {
        // this is a hot function, so pre-reserve enough capacity for most cases
        let mut actions = Vec::with_capacity(16);

        // actions to play or junk a card
        let can_play_person = self.has_empty_person_slot();
        for card_type in self.hand.iter_unique() {
            let can_afford = game_view.game_state.cur_player_water >= card_type.cost();
            match card_type {
                PersonOrEventType::Person(person_type)
                    if person_type.special_type == SpecialType::Holdout =>
                {
                    // PlayPerson/PlayHoldout actions for "Holdout"
                    if can_afford && self.has_empty_holdout_slot(false) {
                        // there's an empty slot in a column with a non-destroyed camp
                        // (and the player can afford Holdout's normal cost)
                        actions.push(Action::PlayPerson(person_type));
                    }
                    if self.has_empty_holdout_slot(true) {
                        // there's an empty slot in a column with a destroyed camp
                        actions.push(Action::PlayHoldout(person_type));
                    }
                }
                PersonOrEventType::Person(person_type) => {
                    // PlayPerson actions for all other people
                    if can_afford && can_play_person {
                        actions.push(Action::PlayPerson(person_type));
                    }
                }
                PersonOrEventType::Event(event_type) => {
                    // PlayEvent actions
                    if can_afford && self.can_play_event(event_type.resolve_turns()) {
                        actions.push(Action::PlayEvent(event_type));
                    }
                }
            }

            // JunkCard actions
            if card_type.junk_effect().can_perform(game_view) {
                actions.push(Action::JunkCard(card_type));
            }
        }

        // action to pay 2 water to draw a card
        // (limited to 1 use per turn)
        if game_view.game_state.cur_player_water >= 2 && !game_view.game_state.has_paid_to_draw {
            actions.push(Action::DrawCard);
        }

        // actions to use a person's ability

        let argo_yesky_ability = self
            .people()
            .filter_map(|person| match person {
                Person::NonPunk {
                    person_type,
                    status,
                } if person_type.special_type == SpecialType::ArgoYesky
                    && *status != NonPunkStatus::Injured =>
                {
                    Some(person_type.abilities[0].as_ref())
                }
                _ => None,
            })
            .next()
            .filter(|ability| ability.can_afford_and_perform(game_view));

        // TODO: make this a little more DRY
        for (loc, person) in self.enumerate_people() {
            match person {
                Person::Punk { is_ready, .. } => {
                    // punks don't have abilities, unless they're given one by another card
                    if *is_ready {
                        if let Some(argo_yesky_ability) = argo_yesky_ability {
                            actions.push(Action::UsePersonAbility(argo_yesky_ability, loc));
                        }
                    }
                }
                Person::NonPunk {
                    person_type,
                    status,
                } => {
                    if *status == NonPunkStatus::Ready {
                        // the person's own abilities
                        for ability in &person_type.abilities {
                            if ability.can_afford_and_perform(game_view) {
                                actions.push(Action::UsePersonAbility(ability.as_ref(), loc));
                            }
                        }

                        // Argo Yesky's ability (if in effect)
                        if person_type.special_type != SpecialType::ArgoYesky {
                            // TODO: somehow dedup abilities better?
                            if let Some(argo_yesky_ability) = argo_yesky_ability {
                                actions.push(Action::UsePersonAbility(argo_yesky_ability, loc));
                            }
                        }

                        // mimic gets its abilities from other people
                        if person_type.special_type == SpecialType::Mimic {
                            // debug_assert that enemy people will always be either Ready or Injured
                            #[cfg(debug_assertions)]
                            for other_person in game_view.other_state().people() {
                                #[rustfmt::skip]
                                debug_assert!(matches!(other_person,
                                    Person::Punk { is_ready: true, .. }
                                    | Person::NonPunk {
                                        status: NonPunkStatus::Ready | NonPunkStatus::Injured, ..
                                    }
                                ), "An enemy person was neither Ready nor Injured");
                            }

                            // either one of your own ready people, or any undamaged enemy (person)
                            // TODO: deduplicate these abilities?
                            let all_people = self.people().chain(game_view.other_state().people());
                            for other_person in all_people {
                                if let Person::NonPunk {
                                    person_type: other_person_type,
                                    status: NonPunkStatus::Ready,
                                } = other_person
                                {
                                    for ability in &other_person_type.abilities {
                                        if ability.can_afford_and_perform(game_view) {
                                            actions.push(Action::UsePersonAbility(
                                                ability.as_ref(),
                                                loc,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // actions to use a camp's ability
        for (loc, camp) in self.enumerate_camps() {
            if camp.is_ready() {
                for ability in &camp.camp_type.abilities {
                    if ability.can_afford_and_perform(game_view) {
                        actions.push(Action::UseCampAbility(ability.as_ref(), loc.column()));
                    }
                }
            }
        }

        // action to end turn (and take Water Silo if possible)
        actions.push(Action::EndTurn);

        actions
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct CardColumn<'ctype> {
    /// The column's camp.
    pub camp: Camp<'ctype>,

    /// The people slots in the column.
    /// The first slot (index 0) is the one in the back.
    pub person_slots: [Option<Person<'ctype>>; 2],
}

impl<'ctype> CardColumn<'ctype> {
    /// Creates a new column with the given camp.
    pub fn new(camp_type: &'ctype CampType) -> Self {
        CardColumn {
            camp: Camp {
                camp_type,
                status: CampStatus::Undamaged,
                is_ready: true,
            },
            person_slots: [None, None],
        }
    }

    /// Returns the person slot at the given location.
    pub fn person_slot(&self, loc: PersonRowIndex) -> Option<&Person<'ctype>> {
        self.person_slots[loc.as_usize()].as_ref()
    }

    /// Returns the person at the given location as mutable.
    pub fn person_mut_slot(&mut self, loc: PersonRowIndex) -> Option<&mut Person<'ctype>> {
        self.person_slots[loc.as_usize()].as_mut()
    }

    /// Returns the person slot at the given location as mutable.
    pub fn person_slot_mut(&mut self, loc: PersonRowIndex) -> &mut Option<Person<'ctype>> {
        &mut self.person_slots[loc.as_usize()]
    }

    /// Returns an iterator over the people in the column.
    pub fn people(&self) -> impl Iterator<Item = &Person<'ctype>> {
        self.person_slots.iter().filter_map(|slot| slot.as_ref())
    }

    /// Returns an iterator over the people in the column as mutable references.
    pub fn people_mut(&mut self) -> impl Iterator<Item = &mut Person<'ctype>> {
        self.person_slots
            .iter_mut()
            .filter_map(|slot| slot.as_mut())
    }

    /// Returns an iterator that enumerates the people in the column.
    pub fn enumerate_people(
        &self,
    ) -> impl DoubleEndedIterator<Item = (PersonRowIndex, &Person<'ctype>)> {
        self.person_slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                slot.as_ref()
                    .map(|person| (PersonRowIndex::from(i), person))
            })
    }

    /// Returns whether this column has any damaged cards that can be restored.
    pub fn has_restorable_card(&self) -> bool {
        self.camp.is_restorable() || self.people().any(|person| person.is_injured())
    }

    /// Returns an iterator over the locations of any damaged and restorable cards in this column.
    pub fn restorable_card_rows(&self) -> impl Iterator<Item = CardRowIndex> + '_ {
        let restorable_camp_row =
            if self.camp.is_restorable() { Some(CardRowIndex::camp()) } else { None };
        let restorable_person_rows =
            self.person_slots
                .iter()
                .enumerate()
                .filter_map(|(row, slot)| {
                    if matches!(slot, Some(person) if person.is_injured()) {
                        let row: PersonRowIndex = row.into();
                        Some(row.into())
                    } else {
                        None
                    }
                });
        restorable_camp_row
            .into_iter()
            .chain(restorable_person_rows)
    }

    /// Returns the row index (0 or 1) of the frontmost person in the column, or None if there are
    /// no people in the column.
    pub fn frontmost_person_row(&self) -> Option<PersonRowIndex> {
        self.person_slots
            .iter()
            .rposition(|person| person.is_some())
            .map(|row| row.into())
    }

    /// Returns the row index (0 (camp), 1, or 2) of the frontmost card in the column, or None if
    /// there are no people in the column and the camp is destroyed.
    pub fn frontmost_card_row(&self) -> Option<CardRowIndex> {
        if let Some(front_person_row) = self.frontmost_person_row() {
            // there's a person in the column; return the row of the person
            Some(front_person_row.into())
        } else if let CampStatus::Destroyed = self.camp.status {
            // there are no people and the camp is destroyed, so there are no unprotected cards in
            // this column
            None
        } else {
            // the (non-destroyed) camp is the only thing in the column
            Some(CardRowIndex::camp())
        }
    }

    /// Returns an iterator over the row indices of the cards in the column (people or non-destroyed
    /// camp).
    pub fn card_rows(&self) -> impl Iterator<Item = CardRowIndex> + '_ {
        let camp_row = if self.camp.is_destroyed() { None } else { Some(CardRowIndex::camp()) };
        let person_rows = self
            .person_slots
            .iter()
            .enumerate()
            .filter_map(|(row, slot)| {
                if slot.is_some() {
                    let row: PersonRowIndex = row.into();
                    Some(row.into())
                } else {
                    None
                }
            });
        camp_row.into_iter().chain(person_rows)
    }

    /// Returns whether the column is empty (no people and camp is destroyed).
    pub fn is_empty(&self) -> bool {
        self.camp.is_destroyed() && self.person_slots.iter().all(|slot| slot.is_none())
    }
}

/// A camp on the board.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Camp<'ctype> {
    /// The camp type.
    pub camp_type: &'ctype CampType,

    /// The damage status of the camp.
    pub status: CampStatus,

    /// Whether the camp is ready.
    is_ready: bool,
}

impl Camp<'_> {
    /// Damages or destroys the camp.
    /// If `destroy` is true, the camp is always destroyed; otherwise, it is damaged.
    /// Does not check for win conditions; that must be done separately.
    /// Panics if the camp is already destroyed.
    pub fn damage(&mut self, destroy: bool) {
        match self.status {
            CampStatus::Undamaged => {
                self.status = if destroy { CampStatus::Destroyed } else { CampStatus::Damaged };
            }
            CampStatus::Damaged => self.status = CampStatus::Destroyed,
            CampStatus::Destroyed => panic!("Tried to damage a destroyed camp"),
        }
    }

    /// Restores the camp.
    /// Panics if the camp is destroyed or undamaged.
    pub fn restore(&mut self) {
        assert!(
            self.status == CampStatus::Damaged,
            "Tried to restore a destroyed or undamaged camp"
        );
        self.status = CampStatus::Undamaged;
    }

    /// Returns whether the camp is destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.status == CampStatus::Destroyed
    }

    /// Returns whether the camp is damaged and can be restored.
    pub fn is_restorable(&self) -> bool {
        self.status == CampStatus::Damaged
    }

    /// Returns whether the camp is ready to use an ability.
    pub fn is_ready(&self) -> bool {
        self.is_ready && self.status != CampStatus::Destroyed
    }

    /// Sets whether the camp is ready. Has no effect if the camp is destroyed.
    pub fn set_ready(&mut self, is_ready: bool) {
        self.is_ready = is_ready;
    }
}

impl StyledName for Camp<'_> {
    /// Returns this camps's name, styled for display.
    fn styled_name(&self) -> Span<'static> {
        match self.status {
            CampStatus::Undamaged => Span::styled(self.camp_type.name, *CAMP),
            CampStatus::Damaged => Span::styled(self.camp_type.name, *CAMP_DAMAGED),
            CampStatus::Destroyed => Span::styled("<destroyed>", *CAMP_DESTROYED),
        }
    }
}

/// Enum representing the damage status of a camp.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CampStatus {
    Undamaged,
    Damaged,
    Destroyed,
}

/// A person played on the board (a punk or face-up person).
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum Person<'ctype> {
    Punk {
        /// Whether the punk is ready.
        is_ready: bool,
    },
    NonPunk {
        /// The identity of the person card.
        person_type: &'ctype PersonType,

        /// The damage/readiness status of the person.
        status: NonPunkStatus,
    },
}

/// Enum representing the damage/readiness of a non-punk person.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NonPunkStatus {
    /// Not injured and ready.
    Ready,
    /// Not injured but not ready.
    NotReady,
    /// Injured.
    Injured,
}

impl<'ctype> Person<'ctype> {
    /// Creates a non-ready punk.
    pub(super) fn new_punk() -> Self {
        Person::Punk { is_ready: false }
    }

    /// Creates a Person from a person type to be played onto the board.
    /// The supplied view must be for the player playing the person.
    /// The person will be ready if person_type.enters_play_ready is true or if
    /// Karli Blaze's trait is active; otherwise, it will be not ready and uninjured.
    pub(super) fn new_non_punk(person_type: &'ctype PersonType, game_view: &GameView) -> Self {
        let force_ready = game_view
            .my_state()
            .has_special_person(SpecialType::KarliBlaze);

        Person::NonPunk {
            person_type,
            status: if force_ready || person_type.enters_play_ready {
                NonPunkStatus::Ready
            } else {
                NonPunkStatus::NotReady
            },
        }
    }

    /// Returns whether this person is injured (and therefore can be restored).
    pub fn is_injured(&self) -> bool {
        matches!(self, Person::NonPunk { status, .. } if *status == NonPunkStatus::Injured)
    }

    /// Restores this person.
    /// Panics if the person is not injured.
    pub fn restore(&mut self) {
        match self {
            Person::Punk { .. } => panic!("Tried to restore a punk"),
            Person::NonPunk { status, .. } => {
                assert!(
                    *status == NonPunkStatus::Injured,
                    "Tried to restore an undamaged person"
                );
                *status = NonPunkStatus::NotReady;
            }
        }
    }

    /// Sets this person to be ready. Has no effect if the person is injured or already ready.
    pub fn set_ready(&mut self) {
        match self {
            Person::Punk { is_ready, .. } => {
                *is_ready = true;
            }
            Person::NonPunk { status, .. } => {
                if *status == NonPunkStatus::NotReady {
                    *status = NonPunkStatus::Ready;
                }
            }
        }
    }

    /// Sets this person to be not ready. Has no effect if the person is injured or already not
    /// ready.
    pub fn set_not_ready(&mut self) {
        match self {
            Person::Punk { is_ready, .. } => {
                *is_ready = false;
            }
            Person::NonPunk { status, .. } => {
                if *status == NonPunkStatus::Ready {
                    *status = NonPunkStatus::NotReady;
                }
            }
        }
    }
}

impl StyledName for Person<'_> {
    /// Returns the name of the person, styled for display.
    fn styled_name(&self) -> Span<'static> {
        match self {
            Person::Punk { .. } => Span::styled("Punk", *PUNK),
            Person::NonPunk {
                person_type,
                status,
            } => Span::styled(
                person_type.name,
                match status {
                    NonPunkStatus::Ready => *PERSON_READY,
                    NonPunkStatus::NotReady => *PERSON_NOT_READY,
                    NonPunkStatus::Injured => *PERSON_INJURED,
                },
            ),
        }
    }
}

impl StyledName for Option<Person<'_>> {
    /// Returns the name of the person slot, styled for display.
    fn styled_name(&self) -> Span<'static> {
        match self {
            Some(person) => person.styled_name(),
            None => Span::styled("<none>", *EMPTY),
        }
    }
}
