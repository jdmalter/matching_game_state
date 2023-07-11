use crate::{Coordinate, Tile, HAND_CAPACITY, PLAYER_CAPACITY};
use bimap::BiBTreeMap;
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashMap};

/// A bimap of indexes of [tiles](Tile) to be played to [coordinates](Coordinate) on
/// the board.
///
/// # See Also
///
/// * [Coordinate]
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
pub type Plays = BiBTreeMap<usize, Coordinate>;
/// An ordered set of indexes of [tiles](Tile) to be exchanged.
///
/// # See Also
///
/// * [NextState::next_exchange](crate::NextState::next_exchange)
pub type Exchanges = BTreeSet<usize>;

/// This is a bag of all the [tiles](Tile) that haven't been removed yet.
///
/// # See Also
///
/// * [Tile]
/// * [FirstState](crate::FirstState)
/// * [NextState](crate::NextState)
pub type Bag = Vec<Tile>;
/// This is a map of [coordinates](Coordinate) to [tiles](Tile) that have been played.
///
/// # See Also
///
/// * [Coordinate]
/// * [Tile]
/// * [NextState](crate::NextState)
/// * [NextView](crate::NextView)
/// * [LastState](crate::LastState)
/// * [LastView](crate::LastView)
/// * [check_line](crate::check_line)
pub type Board = HashMap<Coordinate, Tile>;
/// A vector of the maximum number of matching [tiles](Tile) in each player's hand.
///
/// # See Also
///
/// * [PLAYER_CAPACITY]
/// * [FirstState](crate::FirstState)
/// * [FirstView](crate::FirstView)
pub type MaxMatches = SmallVec<[usize; PLAYER_CAPACITY]>;
/// A vector of points for each player.
///
/// # See Also
///
/// * [PLAYER_CAPACITY]
/// * [NextState](crate::NextState)
/// * [NextView](crate::NextView)
/// * [LastState](crate::LastState)
/// * [LastView](crate::LastView)
pub type Points = SmallVec<[usize; PLAYER_CAPACITY]>;
/// A vector of [tiles](Tile) for one player.
///
/// # See Also
///
/// * [Tile]
/// * [HAND_CAPACITY]
/// * [Hands]
/// * [FirstState::get_hand](crate::FirstState::get_hand)
/// * [NextState::get_hand](crate::NextState::get_hand)
pub type Hand = SmallVec<[Tile; HAND_CAPACITY]>;
/// A vector of hands for each player, where each hand is
/// a vector of [tiles](Tile).
///
/// # See Also
///
/// * [Hand]
/// * [PLAYER_CAPACITY]
/// * [FirstState](crate::FirstState)
/// * [NextState](crate::NextState)
/// * [LastView](crate::LastView)
pub type Hands = SmallVec<[Hand; PLAYER_CAPACITY]>;
/// A vector of hand lengths.
///
/// # See Also
///
/// * [Hands]
/// * [PLAYER_CAPACITY]
/// * [FirstView](crate::FirstView)
/// * [NextView](crate::NextView)
pub type HandLens = SmallVec<[usize; PLAYER_CAPACITY]>;
