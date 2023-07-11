use crate::{Board, Hands, Points};

pub use last_view::*;

mod last_view;
#[cfg(test)]
mod test_setup;

/// Owns game state on the last turn and implements methods.
/// Created from [NextState::next_play](crate::NextState::next_play).
#[derive(Debug)]
pub struct LastState {
    /// This is a map of [coordinates](crate::Coordinate) to [tiles](crate::Tile) that
    /// have been played.
    board: Board,
    /// A vector of points for each player.
    points: Points,
    /// A vector of hands for each player, where each hand is
    /// a vector of [tiles](crate::Tile).
    hands: Hands,
}

impl LastState {
    /// # Arguments
    ///
    /// * `board`: This is a map of [coordinates](crate::Coordinate) to [tiles](crate::Tile) that
    /// have been played.
    /// * `points`: A vector of points for each player.
    /// * `hands`: A vector of hands for each player, where each hand is
    /// a vector of [tiles](crate::Tile).
    ///
    /// # Returns
    ///
    /// A [LastState] struct with properties owned from arguments.
    pub(super) fn new(board: Board, points: Points, hands: Hands) -> LastState {
        LastState {
            board,
            points,
            hands,
        }
    }
}
