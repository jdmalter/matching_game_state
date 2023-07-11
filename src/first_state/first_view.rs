use crate::state::{FirstState, Hand, HandLens, MaxMatches};
use smallvec::SmallVec;

/// Immutably borrows properties from [`FirstState`].
#[derive(Debug)]
pub struct FirstView<'a> {
    /// This is a bag of all the tiles that haven't been removed yet.
    pub bag_len: usize,
    /// A vector of hand lengths.
    pub hand_lens: HandLens,
    /// A vector of the maximum number of matching tiles in each player's hand.
    pub max_matches: &'a MaxMatches,
    /// The index of the player whose turn it is.
    pub current_player: usize,
}

impl<'a> FirstState {
    /// # Returns
    ///
    /// A new [`FirstView`] struct, which immutably borrows properties from [`FirstState`], but
    /// with `bag` replaced by `bag.len()` and `hands` replaced by the number
    /// of tiles in each hand.
    pub fn first_view(&'a self) -> FirstView<'a> {
        FirstView {
            bag_len: self.bag.len(),
            hand_lens: self.hands.iter().map(SmallVec::len).collect(),
            max_matches: &self.max_matches,
            current_player: self.current_player,
        }
    }

    /// # Returns
    ///
    /// A vector of tiles held by the requesting player or `None` if out of bounds.
    pub fn get_hand(&self, index: usize) -> Option<&Hand> {
        self.hands.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn first_view() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        let players = first_state.random_players(&mut rng);
        let bag_len = first_state.random_bag(&mut rng);
        first_state.random_hands(&mut rng);
        first_state.max_matches_to_hand_len();
        first_state.current_player = rng.gen_range(0..players);

        let first_view = first_state.first_view();

        let hands: HandLens = first_state.hands.iter().map(SmallVec::len).collect();
        assert_eq!(bag_len, first_view.bag_len);
        assert_eq!(hands, first_view.hand_lens);
        assert_eq!(first_state.max_matches, *first_view.max_matches);
        assert_eq!(first_state.current_player, first_view.current_player);
    }

    #[test]
    fn get_hand_some() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        let players = first_state.random_players(&mut rng);
        first_state.random_hands(&mut rng);

        for player in 0..players {
            let hand = first_state.get_hand(player).cloned().unwrap();
            assert_eq!(first_state.hands[player], hand);
        }
    }

    #[test]
    fn get_hand_none() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        let players = first_state.random_players(&mut rng);

        assert!(first_state.get_hand(players).is_none());
    }
}
