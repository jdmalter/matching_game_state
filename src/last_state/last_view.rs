use crate::state::{Board, Hands, LastState, Points};

/// Immutably borrows properties from [`LastState`].
#[derive(Debug)]
pub struct LastView<'a> {
    /// This is a map of coordinates to tiles that have been played.
    pub board: &'a Board,
    /// A vector of points for each player.
    pub points: &'a Points,
    /// A vector of hands, where each hand is a vector of tiles.
    pub hands: &'a Hands,
}

impl<'a> LastState {
    /// # Returns
    ///
    /// A new [`LastView`] struct, which immutably borrows properties from [`LastState`].
    #[inline]
    pub fn last_view(&'a self) -> LastView<'a> {
        LastView {
            board: &self.board,
            points: &self.points,
            hands: &self.hands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_view() {
        let mut rng = rand::thread_rng();
        let last_state: LastState = LastState::random_last_state(&mut rng);

        let last_view = last_state.last_view();

        assert_eq!(last_state.board, *last_view.board);
        assert_eq!(last_state.points, *last_view.points);
        assert_eq!(last_state.hands, *last_view.hands);
    }
}
