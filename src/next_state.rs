use crate::{
    find_component_minimums_and_maximums, Bag, Board, Color, Hands, Points, Shape, TILES_LEN,
};

pub use next_exchange::*;
pub use next_play::*;
pub use next_view::*;

mod next_exchange;
mod next_play;
mod next_view;
#[cfg(test)]
mod test_setup;

/// Owns game state after the first turn but before the last turn and implements methods.
/// Created from [FirstState::first_play](crate::FirstState::first_play).
#[derive(Debug)]
pub struct NextState {
    /// This is a bag of all the [tiles](crate::Tile) that haven't been removed yet.
    bag: Bag,
    /// This is a map of [coordinates](crate::Coordinate) to [tiles](crate::Tile) that
    /// have been played.
    board: Board,
    /// A vector of points for each player.
    points: Points,
    /// A vector of hands for each player, where each hand is
    /// a vector of [tiles](crate::Tile).
    hands: Hands,
    /// The index of the player whose turn it is.
    current_player: usize,
}

impl NextState {
    /// # Arguments
    ///
    /// * `bag`: This is a bag of all the [tiles](crate::Tile) that haven't been removed yet.
    /// * `board`: This is a map of [coordinates](crate::Coordinate) to [tiles](crate::Tile) that
    /// have been played.
    /// * `points`: A vector of points for each player.
    /// * `hands`: A vector of hands for each player, where each hand is
    /// a vector of [tiles](crate::Tile).
    /// * `current_player`: The index of the player whose turn it is.
    ///
    /// # Returns
    ///
    /// A [NextState] struct with properties owned from arguments.
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

    /// The index of the player whose turn it is.
    pub fn current_player(&self) -> usize {
        self.current_player
    }

    /// Whether the current player's hand is empty or the board is
    /// a filled rectangle of [every color](Color::colors) and [every shape](Shape::shapes).
    pub(super) fn has_ended(&self) -> bool {
        if self.hands[self.current_player].is_empty() {
            return true;
        }
        if self.board.len() != TILES_LEN {
            return false;
        }

        // More expensive (and still relatively cheap) check should be rarely reached
        let Some((min_x, min_y, max_x, max_y)) =
            find_component_minimums_and_maximums(self.board.keys().copied()) else {
            return false;
        };

        let (x_diff, y_diff) = (max_x.abs_diff(min_x), max_y.abs_diff(min_y));

        (x_diff == Color::COLORS_LEN - 1 && y_diff == Shape::SHAPES_LEN - 1)
            || (y_diff == Color::COLORS_LEN - 1 && x_diff == Shape::SHAPES_LEN - 1)
    }
}
