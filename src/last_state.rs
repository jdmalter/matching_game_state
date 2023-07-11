use crate::state::{Board, Hands, Points};
pub use last_view::*;

mod last_view;
#[cfg(test)]
mod test_utils;

/// Owns game state after the last turn and implements methods. Created from either
/// [`FirstState::first_play`](crate::state::FirstState::first_play) or
/// [`NextState::next_play`](crate::state::NextState::next_play).
#[derive(Debug)]
pub struct LastState {
    /// This is a map of coordinates to tiles that have been played.
    board: Board,
    /// A vector of points for each player.
    points: Points,
    /// A vector of hands, where each hand is a vector of tiles.
    hands: Hands,
}

impl LastState {
    /// # Arguments
    ///
    /// * `board`: This is a map of coordinates to tiles that have been played.
    /// * `points`: A vector of points for each player.
    /// * `hands`: A vector of hands, where each hand is a vector of tiles.
    ///
    /// # Returns
    ///
    /// A [`LastState`] struct with properties owned from arguments.
    pub(super) fn new(board: Board, points: Points, hands: Hands) -> LastState {
        LastState {
            board,
            points,
            hands,
        }
    }
}
