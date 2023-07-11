use crate::{Exchanges, NextState};
use itertools::Itertools;
use rand::distributions::{Distribution, Uniform};
use std::collections::{BTreeSet, HashSet};

/// Describes the reason why the [next exchange](NextState::next_exchange) could not be executed.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NextExchangeError {
    /// Attempting [to exchange](NextState::next_exchange) after the game has ended.
    HasEnded,
    /// Attempting [to exchange](NextState::next_exchange) no [tiles](crate::Tile).
    EmptyTiles,
    /// Attempting [to exchange](NextState::next_exchange) [tiles](crate::Tile) not
    /// in the player's hand.
    IndexesOutOfBounds {
        /// [Exchanges](Exchanges) where the index is greater than or equal to `hand_len`.
        illegal_exchanges: Exchanges,
        /// The number of [tiles](crate::Tile) in the current player's hand,
        /// or the minimum illegal [exchange](Exchanges) index.
        hand_len: usize,
    },
    /// Attempting [to exchange](NextState::next_exchange) only illegal [tiles](crate::Tile).
    NoLegalTiles,
    /// Attempting [to exchange](NextState::next_exchange) more legal [tiles](crate::Tile)
    /// than [tiles](crate::Tile) in the bag.
    NotEnoughTiles {
        /// The number of legal [tiles](crate::Tile) being exchanged.
        legal_exchanges: usize,
        /// The number of available [tiles](crate::Tile) in the bag.
        bag_len: usize,
    },
}

impl NextState {
    /// Checks whether [exchanges](Exchanges) matches various error conditions and
    /// returns all found errors. Otherwise, [exchanges](Exchanges) [tiles](crate::Tile)
    /// from the current player's hand with [tiles](crate::Tile)
    /// from the bag, ignores points,
    /// and advances to the next player.
    ///
    /// # Arguments
    ///
    /// * `exchanges`: An ordered set of indexes of [tiles](crate::Tile) to be exchanged.
    ///
    /// # Errors
    ///
    /// * [NextExchangeError::HasEnded] Attempting [to exchange](NextState::next_exchange)
    /// after the game has ended.
    /// * [NextExchangeError::EmptyTiles] Attempting [to exchange](NextState::next_exchange)
    /// no [tiles](crate::Tile).
    /// * [NextExchangeError::IndexesOutOfBounds] Attempting [to exchange](NextState::next_exchange)
    /// [tiles](crate::Tile) not in the player's hand.
    /// * [NextExchangeError::NoLegalTiles] Attempting [to exchange](NextState::next_exchange) only
    /// illegal [tiles](crate::Tile).
    /// * [NextExchangeError::NotEnoughTiles] Attempting [to exchange](NextState::next_exchange)
    /// more legal [tiles](crate::Tile) than [tiles](crate::Tile) in the bag.
    pub fn next_exchange(
        &mut self,
        exchanges: &Exchanges,
    ) -> Result<(), HashSet<NextExchangeError>> {
        self.check_exchanges(&exchanges)?;

        // Cannot filter or drain by tile since exchanges might request
        // a subset of duplicate tiles
        let hand = &mut self.hands[self.current_player];
        let tiles_from_hand = exchanges
            .iter()
            .rev()
            .map(|&index| hand.remove(index))
            .collect_vec();

        // Drain the bag before adding tiles from the hand
        // so that tiles do not return into the hand
        hand.extend(self.bag.drain(self.bag.len() - exchanges.len()..));

        // shuffle tiles in the bag, but in place
        // and without O(n log n) shuffle operation
        let mut rng = rand::thread_rng();
        let start = self.bag.len();
        self.bag.extend(tiles_from_hand);
        let end = self.bag.len();
        let possible_indexes = Uniform::from(0..end);
        for index in start..end {
            self.bag.swap(index, possible_indexes.sample(&mut rng));
        }

        self.current_player = (self.current_player + 1) % self.hands.len();
        Ok(())
    }

    /// Checks whether [exchanges](Exchanges) matches various error conditions
    /// and returns all found errors.
    ///
    /// # Arguments
    ///
    /// * `exchanges`: An ordered set of indexes of [tiles](crate::Tile) to be exchanged.
    ///
    /// # Errors
    ///
    /// * [NextExchangeError::HasEnded] Attempting [to exchange](NextState::next_exchange)
    /// after the game has ended.
    /// * [NextExchangeError::EmptyTiles] Attempting [to exchange](NextState::next_exchange)
    /// no [tiles](crate::Tile).
    /// * [NextExchangeError::IndexesOutOfBounds] Attempting [to exchange](NextState::next_exchange)
    /// [tiles](crate::Tile) not in the player's hand.
    /// * [NextExchangeError::NoLegalTiles] Attempting [to exchange](NextState::next_exchange) only
    /// illegal [tiles](crate::Tile).
    /// * [NextExchangeError::NotEnoughTiles] Attempting [to exchange](NextState::next_exchange)
    /// more legal [tiles](crate::Tile) than [tiles](crate::Tile) in the bag.
    fn check_exchanges(&self, exchanges: &Exchanges) -> Result<(), HashSet<NextExchangeError>> {
        let mut errors = HashSet::with_capacity(4);
        if self.has_ended() {
            errors.insert(NextExchangeError::HasEnded);
        }

        if exchanges.is_empty() {
            errors.insert(NextExchangeError::EmptyTiles);
            return Err(errors);
        }

        let hand_len = self.hands[self.current_player].len();
        let illegal_exchanges: BTreeSet<usize> = exchanges.range(hand_len..).copied().collect();
        if !illegal_exchanges.is_empty() {
            errors.insert(NextExchangeError::IndexesOutOfBounds {
                illegal_exchanges,
                hand_len,
            });
        }

        let legal_exchanges = exchanges.range(..hand_len).count();
        let bag_len = self.bag.len();
        if legal_exchanges == 0 {
            errors.insert(NextExchangeError::NoLegalTiles);
        } else if legal_exchanges > bag_len {
            errors.insert(NextExchangeError::NotEnoughTiles {
                legal_exchanges,
                bag_len,
            });
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, NextState};
    use map_macro::{btree_set, hash_set};
    use rand::Rng;
    use tap::Tap;

    impl NextState {
        fn test_next_exchange_one_error(
            self,
            exchanges: impl IntoIterator<Item = usize>,
            expected_error: NextExchangeError,
        ) {
            self.test_next_exchange_errors(exchanges, hash_set! { expected_error });
        }

        fn test_next_exchange_errors(
            mut self,
            exchanges: impl IntoIterator<Item = usize>,
            expected_error: HashSet<NextExchangeError>,
        ) {
            let exchanges = exchanges.into_iter().collect();
            let actual_error = self
                .next_exchange(&exchanges)
                .expect_err("next_exchange should return Err");

            assert_eq!(expected_error, actual_error);
        }
    }

    #[test]
    fn has_ended() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_bag(&mut rng);
        next_state.random_players(&mut rng);
        next_state.deadlocked_board();
        next_state.random_hands(&mut rng);

        next_state.test_next_exchange_one_error([0], NextExchangeError::HasEnded);
    }

    #[test]
    fn empty_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        next_state.random_hands(&mut rng);

        next_state.test_next_exchange_one_error([], NextExchangeError::EmptyTiles);
    }

    #[test]
    fn indexes_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        next_state.random_board(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let possible_illegal_indexes = Uniform::from(hand_len..=usize::MAX);
        let illegal_exchanges: Exchanges = (1..hand_len)
            .map(|_| possible_illegal_indexes.sample(&mut rng))
            .collect();

        next_state.test_next_exchange_one_error(
            illegal_exchanges.clone().tap_mut(|exchanges| {
                exchanges.insert(0);
            }),
            NextExchangeError::IndexesOutOfBounds {
                illegal_exchanges,
                hand_len,
            },
        );
    }

    #[test]
    fn indexes_out_of_bounds_no_legal_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let possible_illegal_indexes = Uniform::from(hand_len..=usize::MAX);
        let illegal_exchanges: Exchanges = (0..hand_len)
            .map(|_| possible_illegal_indexes.sample(&mut rng))
            .collect();

        next_state.test_next_exchange_errors(
            illegal_exchanges.clone(),
            hash_set! {
              NextExchangeError::IndexesOutOfBounds {
                illegal_exchanges,
                hand_len,
              },
              NextExchangeError::NoLegalTiles,
            },
        );
    }

    #[test]
    fn not_enough_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        next_state.random_hands(&mut rng);

        next_state.test_next_exchange_one_error(
            [0],
            NextExchangeError::NotEnoughTiles {
                legal_exchanges: 1,
                bag_len: 0,
            },
        );
    }

    #[test]
    fn indexes_out_of_bounds_not_enough_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let possible_illegal_indexes = Uniform::from(hand_len..=usize::MAX);
        let illegal_exchanges: Exchanges = (1..hand_len)
            .map(|_| possible_illegal_indexes.sample(&mut rng))
            .collect();

        next_state.test_next_exchange_errors(
            illegal_exchanges.clone().tap_mut(|exchanges| {
                exchanges.insert(0);
            }),
            hash_set! {
              NextExchangeError::IndexesOutOfBounds {
                illegal_exchanges,
                hand_len,
              },
              NextExchangeError::NotEnoughTiles {
                legal_exchanges: 1,
                bag_len: 0,
              }
            },
        );
    }

    #[test]
    fn exchange_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        let hand_color = Color::Blue;
        let current_player = next_state.current_player;
        let hand = &mut next_state.hands[current_player];

        let first = (hand_color, rng.gen());
        let second = (hand_color, rng.gen());
        let third = (hand_color, rng.gen());
        hand.extend([first, second, third]);

        let bag_tile = (Color::Red, rng.gen());
        let bag_len = rng.gen_range(hand.len()..10);
        next_state.bag.extend((0..bag_len).map(|_| bag_tile));

        let exchanges = btree_set! {1};
        let exchanges_len = exchanges.len();

        next_state
            .next_exchange(&exchanges)
            .expect("next_exchange should return Ok");

        let hand = &next_state.hands[current_player];
        assert_eq!(first, hand[0]);
        assert_eq!(third, hand[1]);
        assert_eq!(bag_tile, hand[2]);

        let counts = next_state.bag.iter().counts();
        assert_eq!(bag_len - exchanges_len, counts[&bag_tile]);
        assert_eq!(1, counts[&second]);
    }

    #[test]
    fn exchange_no_points() {
        let (mut next_state, exchanges) = set_up_next_exchange();

        next_state
            .next_exchange(&exchanges)
            .expect("next_exchange should return Ok");

        assert_eq!(0, next_state.points[0]);
    }

    #[test]
    fn exchange_some_points() {
        let (mut next_state, exchanges) = set_up_next_exchange();
        next_state.random_points(&mut rand::thread_rng());
        let points = next_state.points[0];

        next_state
            .next_exchange(&exchanges)
            .expect("next_exchange should return Ok");

        assert_eq!(points, next_state.points[0]);
    }

    #[test]
    fn exchange_increment_current_player() {
        let (mut next_state, exchanges) = set_up_next_exchange();

        next_state
            .next_exchange(&exchanges)
            .expect("next_exchange should return Ok");

        assert_eq!(1, next_state.current_player);
    }

    #[test]
    fn exchange_wrap_current_player() {
        let (mut next_state, exchanges) = set_up_next_exchange();
        next_state.current_player = next_state.points.len() - 1;

        next_state
            .next_exchange(&exchanges)
            .expect("next_exchange should return Ok");

        assert_eq!(0, next_state.current_player);
    }

    fn set_up_next_exchange() -> (NextState, Exchanges) {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        next_state.random_board(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let exchanges = (0..hand_len).collect();

        (next_state, exchanges)
    }
}
