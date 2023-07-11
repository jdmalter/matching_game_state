use crate::{
    random_board, random_players, Board, Hands, LastState, Points, PLAYER_CAPACITY, TILES_LEN,
};
use rand::Rng;

impl LastState {
    /// Generates an empty [LastState] with no players.
    ///
    /// # Returns
    ///
    /// A [LastState] struct with the properties set to the following:
    /// * `board`: An empty board.
    /// * `points`: An empty points vector.
    /// * `hands`: An empty hands vector.
    pub fn empty_last_state() -> LastState {
        // capacity hardcoded to highest expected demand during test cases
        LastState {
            board: Board::with_capacity(TILES_LEN),
            points: Points::with_capacity(PLAYER_CAPACITY),
            hands: Hands::with_capacity(PLAYER_CAPACITY),
        }
    }

    /// Generates an last state with a random, small, non-zero number of players.
    /// The board contains one [tile](Tile) for every other x in a random, small, non-zero
    /// horizontal range at a random, small y. All points are a random, medium, non-zero number.
    /// All hands contain the same random, small, non-zero number of random [tiles](Tile).
    ///
    /// # Returns
    ///
    /// A [LastState] struct with the properties set to the following:
    /// * `board`: A map with one [tile](Tile) for every other x at a random, small y.
    /// * `points`: A vector of length players of random, non-zero points.
    /// * `hands`: A vector of length players of random hands.
    pub fn random_last_state<R: Rng + ?Sized>(rng: &mut R) -> LastState {
        let mut last_state = LastState::empty_last_state();
        random_board(rng, &mut last_state.board);
        random_players(rng, &mut last_state.points, &mut last_state.hands);

        last_state
    }

    /// A mutable reference to `self.board`.
    pub fn mut_board(&mut self) -> &mut Board {
        &mut self.board
    }

    /// A mutable reference to `self.points`.
    pub fn mut_points(&mut self) -> &mut Points {
        &mut self.points
    }

    /// A mutable reference to `self.hands`.
    pub fn mut_hands(&mut self) -> &mut Hands {
        &mut self.hands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_last_state() {
        let last_state: LastState = LastState::empty_last_state();

        assert_eq!(Board::new(), last_state.board);
        assert_eq!(Points::new(), last_state.points);
        assert_eq!(Hands::new(), last_state.hands);
    }
}
