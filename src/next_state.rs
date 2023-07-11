use crate::state::{Bag, Board, Color, Hands, Points, Shape, TILES_LEN};
pub use next_exchange::*;
pub use next_play::*;
pub use next_view::*;
use std::cmp;

mod next_exchange;
mod next_play;
mod next_view;
#[cfg(test)]
mod test_utils;

/// Owns game state after the first turn and implements methods. Created from
/// [`FirstState::first_play`](crate::state::FirstState::first_play).
#[derive(Debug)]
pub struct NextState {
    /// This is a bag of all the tiles that haven't been removed yet.
    bag: Bag,
    /// This is a map of coordinates to tiles that have been played.
    board: Board,
    /// A vector of points for each player.
    points: Points,
    /// A vector of hands, where each hand is a vector of tiles.
    hands: Hands,
    /// The index of the player whose turn it is.
    current_player: usize,
}

impl NextState {
    /// # Arguments
    ///
    /// * `bag`: This is a bag of all the tiles that haven't been removed yet.
    /// * `board`: This is a map of coordinates to tiles that have been played.
    /// * `points`: A vector of points for each player.
    /// * `hands`: A vector of hands, where each hand is a vector of tiles.
    /// * `current_player`: The index of the player whose turn it is.
    ///
    /// # Returns
    ///
    /// A [`NextState`] struct with properties owned from arguments.
    pub(super) fn new(
        bag: Bag,
        board: Board,
        points: Points,
        hands: Hands,
        current_player: usize,
    ) -> NextState {
        NextState {
            bag,
            board,
            points,
            hands,
            current_player,
        }
    }

    /// # Returns
    ///
    /// The index of the player whose turn it is.
    #[inline]
    pub fn current_player(&self) -> usize {
        self.current_player
    }

    /// # Returns
    ///
    /// Whether `current_player`'s hand is empty or `board` is a filled rectangle
    /// of [`Color::COLORS_LEN`] by [`Shape::SHAPES_LEN`].
    pub(super) fn has_ended(&self) -> bool {
        if self.hands[self.current_player].is_empty() {
            return true;
        }
        if self.board.len() != TILES_LEN {
            return false;
        }

        // More expensive (and still relatively cheap) check should be reached rarely
        let mut min_x = isize::MAX;
        let mut max_x = isize::MIN;
        let mut min_y = isize::MAX;
        let mut max_y = isize::MIN;

        for &(x, y) in self.board.keys() {
            min_x = cmp::min(min_x, x);
            max_x = cmp::max(max_x, x);
            min_y = cmp::min(min_y, y);
            max_y = cmp::max(max_y, y);
        }

        let x_diff = max_x.abs_diff(min_x);
        let y_diff = max_y.abs_diff(min_y);

        (x_diff == Color::COLORS_LEN - 1 && y_diff == Shape::SHAPES_LEN - 1)
            || (y_diff == Color::COLORS_LEN - 1 && x_diff == Shape::SHAPES_LEN - 1)
    }
}
