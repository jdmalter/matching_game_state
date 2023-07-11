use crate::{
    random_bag, random_current_player, random_hands, Bag, FirstState, Hand, Hands, MaxMatches,
    Points, HAND_CAPACITY, PLAYER_CAPACITY, TILES_LEN,
};
use rand::Rng;

impl FirstState {
    /// Generates an empty [FirstState] with no players.
    ///
    /// # Returns
    ///
    /// A [FirstState] struct with the properties set to the following:
    /// * `bag`: An empty bag.
    /// * `hands`: An empty hands vector.
    /// * `max_matches`: An empty max matches vector.
    /// * `current_player`: `0`.
    pub fn empty_first_state() -> FirstState {
        // capacity hardcoded to highest expected demand during test cases
        FirstState {
            bag: Bag::with_capacity(TILES_LEN),
            hands: Hands::with_capacity(PLAYER_CAPACITY),
            max_matches: MaxMatches::with_capacity(PLAYER_CAPACITY),
            current_player: 0,
        }
    }

    /// A mutable reference to `self.bag`.
    pub fn mut_bag(&mut self) -> &mut Bag {
        &mut self.bag
    }

    /// A mutable reference to `self.hands`.
    pub fn mut_hands(&mut self) -> &mut Hands {
        &mut self.hands
    }

    /// A mutable reference to `self.hands`.
    pub fn mut_max_matches(&mut self) -> &mut Points {
        &mut self.max_matches
    }

    /// A mutable reference to `self.current_player`.
    pub fn mut_current_player(&mut self) -> &mut usize {
        &mut self.current_player
    }

    /// It inserts a random, small, non-zero number of empty hands into hands
    /// and `0`s into max matches.
    ///
    /// # Returns
    ///
    /// The number of additional hands/max matches.
    pub fn random_players<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        let players = rng.gen_range(2..=PLAYER_CAPACITY);
        for _ in 0..players {
            self.hands.push(Hand::with_capacity(HAND_CAPACITY));
            self.max_matches.push(0);
        }

        players
    }

    /// It inserts a random, small, non-zero number of [tiles](Tile) into the bag.
    ///
    /// # Returns
    ///
    /// The number of additional [tiles](Tile) in the bag.
    pub fn random_bag<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_bag(rng, &mut self.bag)
    }

    /// Pushes the same random, small, non-zero number of [tiles](Tile) into
    /// each player's hand.
    ///
    /// # Returns
    ///
    /// The number of additional [tiles](Tile) in each player's hand.
    pub fn random_hands<R: Rng + ?Sized>(&mut self, rng: &mut R) -> usize {
        random_hands(rng, &mut self.hands)
    }

    /// Sets each index in max matches to the number of [tiles](Tile) in
    /// each hand at the same index in hands.
    pub fn max_matches_to_hand_len(&mut self) {
        assert_eq!(self.max_matches.len(), self.hands.len());

        for index in 0..self.max_matches.len() {
            self.max_matches[index] = self.hands[index].len();
        }
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
    use crate::MaxMatches;

    #[test]
    fn empty_first_state() {
        let first_state = FirstState::empty_first_state();

        assert_eq!(Bag::new(), first_state.bag);
        assert_eq!(Hands::new(), first_state.hands);
        assert_eq!(Points::new(), first_state.max_matches);
        assert_eq!(0, first_state.current_player);
    }

    #[test]
    fn random_players() {
        let mut first_state = FirstState::empty_first_state();

        let players = first_state.random_players(&mut rand::thread_rng());

        let mut hands = Hands::with_capacity(PLAYER_CAPACITY);
        let mut max_matches = MaxMatches::with_capacity(PLAYER_CAPACITY);

        for _ in 0..players {
            hands.push(Hand::new());
            max_matches.push(0);
        }

        assert_eq!(hands, first_state.hands);
        assert_eq!(max_matches, first_state.max_matches);
    }

    #[test]
    fn max_matches_to_hand_len() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand_len = first_state.random_hands(&mut rng);

        for &max_match in &first_state.max_matches {
            assert_eq!(0, max_match);
        }

        first_state.max_matches_to_hand_len();

        for max_match in first_state.max_matches {
            assert_eq!(hand_len, max_match);
        }
    }
}
