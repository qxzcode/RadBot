//! This module contains types representing locations of cards on the board.

use rand::distributions::{Distribution, Standard};
use rand::Rng;

/// A row index for a person (0 or 1) in a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersonRowIndex(u8);

impl PersonRowIndex {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl From<usize> for PersonRowIndex {
    fn from(row: usize) -> Self {
        assert!(row < 2);
        PersonRowIndex(row as u8)
    }
}

/// A row index for a card (0 (camp), 1, or 2) in a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardRowIndex(u8);

impl CardRowIndex {
    pub fn camp() -> Self {
        CardRowIndex(0)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub fn to_person_index(self) -> Result<PersonRowIndex, ()> {
        if self.0 != 0 {
            Ok(PersonRowIndex(self.0 - 1))
        } else {
            Err(())
        }
    }
}

impl From<PersonRowIndex> for CardRowIndex {
    fn from(person_row_index: PersonRowIndex) -> Self {
        CardRowIndex(person_row_index.0 + 1)
    }
}

/// A column index (0, 1, or 2) on a player's board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnIndex(u8);

impl ColumnIndex {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl From<usize> for ColumnIndex {
    fn from(column: usize) -> Self {
        assert!(column < 3);
        ColumnIndex(column as u8)
    }
}

/// A location at which to play a person onto the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayLocation {
    /// The column to play the person into (0, 1, or 2).
    column: ColumnIndex,

    /// The row to play the person into (0 or 1).
    row: PersonRowIndex,
}

impl PlayLocation {
    /// Creates a new PlayLocation.
    pub fn new(column: ColumnIndex, row: PersonRowIndex) -> Self {
        Self { column, row }
    }

    /// Returns the column.
    pub fn column(&self) -> ColumnIndex {
        self.column
    }

    /// Returns the row.
    pub fn row(&self) -> PersonRowIndex {
        self.row
    }
}

/// Enum for specifying a particular player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Player {
    /// Player 1.
    Player1,

    /// Player 2.
    Player2,
}

impl Player {
    /// Returns the other player.
    pub fn other(self) -> Self {
        match self {
            Self::Player1 => Self::Player2,
            Self::Player2 => Self::Player1,
        }
    }

    /// Returns the player's number.
    pub fn number(self) -> u8 {
        match self {
            Self::Player1 => 1,
            Self::Player2 => 2,
        }
    }
}

// allow random generation of Player
impl Distribution<Player> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Player {
        match rng.gen::<bool>() {
            true => Player::Player1,
            false => Player::Player2,
        }
    }
}

/// A location of a card (camp or person) on a specified player's board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardLocation {
    /// The column of the card (0, 1, or 2).
    column: ColumnIndex,

    /// The row of the card (0 (camp), 1, or 2).
    row: CardRowIndex,

    /// The player whose board the card is on.
    player: Player,
}

impl CardLocation {
    /// Creates a new CardLocation.
    pub fn new(column: ColumnIndex, row: CardRowIndex, player: Player) -> Self {
        Self {
            column,
            row,
            player,
        }
    }

    /// Returns the column.
    pub fn column(&self) -> ColumnIndex {
        self.column
    }

    /// Returns the row.
    pub fn row(&self) -> CardRowIndex {
        self.row
    }

    /// Returns the player whose board the card is on.
    pub fn player(&self) -> Player {
        self.player
    }
}

/// A location of a card (camp or person) within a player's board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerCardLocation {
    /// The column of the card (0, 1, or 2).
    column: ColumnIndex,

    /// The row of the card (0 (camp), 1, or 2).
    row: CardRowIndex,
}

impl PlayerCardLocation {
    /// Creates a new PlayerCardLocation.
    pub fn new(column: ColumnIndex, row: CardRowIndex) -> Self {
        Self { column, row }
    }

    /// Returns the column.
    pub fn column(&self) -> ColumnIndex {
        self.column
    }

    /// Returns the row.
    pub fn row(&self) -> CardRowIndex {
        self.row
    }

    /// Converts the location to a location on the specified player's board.
    pub fn for_player(&self, player: Player) -> CardLocation {
        CardLocation::new(self.column, self.row, player)
    }
}
