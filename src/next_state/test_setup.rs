use crate::{
    random_bag, random_board, random_current_player, random_hands, random_players, random_points,
    Bag, Board, Color, Hands, NextState, Points, Shape, PLAYER_CAPACITY, TILES_LEN,
};
use rand::Rng;

impl NextState {
    /// Generates an empty [NextState] with no players.
    ///
    /// # Returns
    ///
    /// A [NextState] struct with the properties set to the following:
    /// * `bag`: An empty bag.
    /// * `board`: An empty board.
    /// * `points`: An empty points vector.
    /// * `hands`: An empty hands vector.
    /// * `current_player`: `0`.
    pub fn empty_next_state() -> NextState {
        // capacity hardcoded to highest expected demand during test cases
        NextState {
            bag: Bag::with_capacity(TILES_LEN),
            board: Board::with_capacity(TILES_LEN),
            points: Points::with_capacity(PLAYER_CAPACITY),
            hands: Hands::with_capacity(PLAYER_CAPACITY),
            current_player: 0,
        }
    }

    /// A mutable reference to `self.bag`.
    pub fn mut_bag(&mut self) -> &mut Bag {
        &mut self.bag
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

    /// A mutable reference to `self.current_player`.
    pub fn mut_current_player(&mut self) -> &mut usize {
        &mut self.current_player
    }

    /// It inserts a random, small, non-zero number of empty hands into hands and
    /// `0`s into points.
    ///
    /// # Returns
    ///
    /// The number of additional points/hands.
    pub fn random_players<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_players(rng, &mut self.points, &mut self.hands)
    }

    /// It inserts a random, small, non-zero number of [tiles](Tile) into the bag.
    ///
    /// # Returns
    ///
    /// The number of additional [tiles](Tile) in the bag.
    pub fn random_bag<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_bag(rng, &mut self.bag)
    }

    /// It inserts one [tile](Tile) for every other x in a random, small, non-zero horizontal range
    /// at a random, small y into the board.
    ///
    /// # Returns
    ///
    /// The number of additional [tiles](Tile) on the board.
    pub fn random_board<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_board(rng, &mut self.board)
    }

    /// Clears board and then inserts all the possible [tiles](Tile) in
    /// a compact grid into the board. The board should contain no possible legal plays.
    pub fn deadlocked_board(&mut self) {
        self.board.clear();
        self.board.extend(
            Color::colors()
                .into_iter()
                .enumerate()
                .flat_map(|(row, color)| {
                    Shape::shapes()
                        .into_iter()
                        .map(move |shape| (color, shape))
                        .enumerate()
                        .map(move |(col, tile)| ((row as isize, col as isize), tile))
                }),
        );
    }

    /// Sets each player's points to a random, medium, non-zero number.
    pub fn random_points<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        random_points(rng, &mut self.points)
    }

    /// Pushes the same random, small, non-zero number of [tiles](Tile)
    /// into each player's hand.
    ///
    /// # Returns
    ///   
    /// The number of additional [tiles](Tile) in each player's hand.
    pub fn random_hands<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_hands(rng, &mut self.hands)
    }

    /// Sets the current player to a random number between `0` inclusive and `players` exclusive.
    ///
    /// # Panics
    ///
    /// If `players` is `0`
    ///
    /// # Returns
    ///   
    /// The index of the player whose turn it is.
    pub fn random_current_player<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_current_player(rng, &mut self.current_player, self.hands.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_next_state() {
        let next_state = NextState::empty_next_state();

        assert_eq!(Bag::new(), next_state.bag);
        assert_eq!(Board::new(), next_state.board);
        assert_eq!(Points::new(), next_state.points);
        assert_eq!(Hands::new(), next_state.hands);
        assert_eq!(0, next_state.current_player);
    }

    #[test]
    fn deadlocked_board() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_hands(&mut rng);

        assert!(!next_state.has_ended());

        next_state.deadlocked_board();

        assert!(next_state.has_ended());
    }
}
