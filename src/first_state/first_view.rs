use crate::{FirstState, Hand, HandLens, MaxMatches};
use smallvec::SmallVec;

/// Immutably borrows properties from [FirstState].
#[derive(Debug)]
pub struct FirstView<'a> {
    /// This is the number of [tiles](crate::Tile) that haven't been removed yet.
    pub bag_len: usize,
    /// A vector of hand lengths.
    pub hand_lens: HandLens,
    /// A vector of the maximum number of matching [tiles](crate::Tile)
    /// in each player's hand.
    pub max_matches: &'a MaxMatches,
    /// The index of the player whose turn it is.
    pub current_player: usize,
}

impl<'a> FirstState {
    /// A new [FirstView] struct, which immutably borrows properties from [FirstState], but
    /// with bag replaced by `bag.len()` and hands replaced by
    /// the number of [tiles](crate::Tile) in each hand.
    pub fn first_view(&'a self) -> FirstView<'a> {
        FirstView {
            bag_len: self.bag.len(),
            hand_lens: self.hands.iter().map(SmallVec::len).collect(),
            max_matches: &self.max_matches,
            current_player: self.current_player,
        }
    }

    /// A vector of [tiles](crate::Tile) held by the requesting player or [None] if out of bounds.
    pub fn get_hand(&self, index: usize) -> Option<&Hand> {
        self.hands.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_view() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let bag_len = first_state.random_bag(&mut rng);
        first_state.random_hands(&mut rng);
        first_state.max_matches_to_hand_len();
        first_state.random_current_player(&mut rng);

        let first_view = first_state.first_view();

        let hand_lens: HandLens = first_state.hands.iter().map(SmallVec::len).collect();
        assert_eq!(bag_len, first_view.bag_len);
        assert_eq!(hand_lens, first_view.hand_lens);
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
            let hand = first_state
                .get_hand(player)
                .cloned()
                .expect("get_hand should be safe in range 0..players");
            assert_eq!(first_state.hands[player], hand);
        }
    }

    #[test]
    fn get_hand_none() {
        let mut first_state = FirstState::empty_first_state();
        let players = first_state.random_players(&mut rand::thread_rng());

        assert!(first_state.get_hand(players).is_none());
    }
}
