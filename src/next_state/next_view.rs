use crate::state::{Board, Hand, HandLens, NextState, Points};
use smallvec::SmallVec;

/// Immutably borrows properties from [`NextState`].
#[derive(Debug)]
pub struct NextView<'a> {
    /// This is a bag of all the tiles that haven't been removed yet.
    pub bag_len: usize,
    /// This is a map of coordinates to tiles that have been played.
    pub board: &'a Board,
    /// A vector of points for each player.
    pub points: &'a Points,
    /// A vector of hand lengths.
    pub hand_lens: HandLens,
    /// The index of the player whose turn it is.
    pub current_player: usize,
}

impl<'a> NextState {
    /// # Returns
    ///
    /// A new [`NextView`] struct, which immutably borrows properties from [`NextState`], but
    /// with `bag` replaced by `bag.len()` and `hands` replaced by the number
    /// of tiles in each hand.
    pub fn next_view(&'a self) -> NextView<'a> {
        NextView {
            bag_len: self.bag.len(),
            board: &self.board,
            points: &self.points,
            hand_lens: self.hands.iter().map(SmallVec::len).collect(),
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
    fn next_view() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        let players = next_state.random_players(&mut rng);
        let bag_len = next_state.random_bag(&mut rng);
        next_state.random_board(&mut rng);
        next_state.random_hands(&mut rng);
        next_state.current_player = rng.gen_range(0..players);
        next_state.random_points(&mut rng);

        let next_view = next_state.next_view();

        let hands: HandLens = next_state.hands.iter().map(SmallVec::len).collect();
        assert_eq!(bag_len, next_view.bag_len);
        assert_eq!(next_state.board, *next_view.board);
        assert_eq!(next_state.points, *next_view.points);
        assert_eq!(hands, next_view.hand_lens);
        assert_eq!(next_state.current_player, next_view.current_player);
    }

    #[test]
    fn get_hand_some() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        let players = next_state.random_players(&mut rng);
        next_state.random_hands(&mut rng);

        for player in 0..players {
            let hand = next_state.get_hand(player).cloned().unwrap();
            assert_eq!(next_state.hands[player], hand);
        }
    }

    #[test]
    fn get_hand_none() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        let players = next_state.random_players(&mut rng);

        assert!(next_state.get_hand(players).is_none());
    }
}
