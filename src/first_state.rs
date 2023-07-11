use crate::state::{
    tiles, Bag, Color, Hands, Indexes, MaxMatches, Shape, PLAYER_CAPACITY, TILES_LEN,
};
pub use first_play::*;
pub use first_view::*;
use itertools::Itertools;
use map_macro::set;
use rand::seq::SliceRandom;
use rand::Rng;
use std::cmp;
use std::collections::HashSet;
use tap::Tap;

mod first_play;
mod first_view;
#[cfg(test)]
mod test_utils;

/// Owns game state on the first turn and implements methods. Created from [`FirstState::new`].
#[derive(Debug)]
pub struct FirstState {
    /// This is a bag of all the tiles that haven't been removed yet.
    bag: Bag,
    /// A vector of hands, where each hand is a vector of tiles.
    hands: Hands,
    /// A vector of the maximum number of matching tiles in each player's hand.
    max_matches: MaxMatches,
    /// The index of the player whose turn it is.
    current_player: usize,
}

/// Describes the reason why [`FirstState`] could not be created.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NewError {
    /// Attempting to start with empty `players`.
    EmptyPlayers,
    /// Attempting to start with an empty `bag`.
    EmptyBag,
    /// Attempting to start with empty `hands`.
    EmptyHands,
    /// Attempting to request more tiles than tiles in `bag`.
    NotEnoughTiles {
        /// The number of tiles requested for `hands`.
        requested_tiles: usize,
        /// The number of available tiles in `bag`.
        tiles_in_bag: usize,
    },
    /// Attempting to add too many tiles into `bag`
    TooManyTiles {
        /// The number of available tiles in `bag`.
        tiles_in_bag: usize,
    },
    /// Attempt to select a current player not in `max_matching_players`.
    CurrentPlayerNotMaxMatchingPlayers {
        /// The index of the player whose turn it is.
        current_player: usize,
        /// An ordered set of indexes of players who all hold the maximum number of matching tiles.
        max_matching_players: Indexes,
    },
}

/// Default of number of times a unique tile is copied in a game. Used by [`FirstState::new`] when
/// `unique_tile_copied_count` is [`None`]. 3 copies of each tile.
pub const DEFAULT_UNIQUE_TILE_COPIED_COUNT: usize = 3;
/// Default maximum number of tiles in a hand. Used by [`FirstState::new`] when `hand_len` is
/// [`None`]. 6 tiles per hand.
pub const DEFAULT_HAND_LEN: usize = 6;
/// The maximum number of available tiles before addition overflows occur.
/// This const should not be public since its value should not be depended on.
/// floor(sqrt(isize::Max)) = 3_037_000_499 available tiles.
const MAXIMUM_BAG_TILES: usize = 3_037_000_499;

impl FirstState {
    /// Checks that `players_len`, `unique_tile_copied_count`, and `hand_len` are all
    /// non-zero, that the number of tiles requested (`player_len * hand_len`) is less than
    /// the number of tiles in `bag` ([`TILES_LEN`] ` * unique_tile_copied_count`), and that
    /// the number of tiles in `bag` is not large enough to cause overflow.
    ///
    /// Creates a `bag` of tiles, and then draws tiles from `bag` to create a hand for each player.
    ///
    /// Finds the maximum number of matching tiles in each hand, and then finds the maximum
    /// of those maximums. Then, finds all players with that maximum number of matches,
    /// and then selects one of those players at random.
    ///
    /// When `unique_tile_copied_count` and/or `hand_len` are [`None`], default values
    /// [`DEFAULT_UNIQUE_TILE_COPIED_COUNT`] and [`DEFAULT_HAND_LEN`] are used respectively.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each tile in `bag`.
    /// * `hand_len`: The number of tiles each player will have in their hand.
    ///
    /// # Errors
    ///
    /// * [`NewError::EmptyPlayers`] Attempting to start with empty `players`.
    /// * [`NewError::EmptyBag`] Attempting to start with an empty `bag`.
    /// * [`NewError::EmptyHands`] Attempting to start with empty `hands`.
    /// * [`NewError::NotEnoughTiles`] Attempting to request more tiles than tiles in `bag`.
    /// * [`NewError::TooManyTiles`] Attempting to add too many tiles into `bag`
    pub fn new_random_first_player_selector(
        players_len: usize,
        unique_tile_copied_count: Option<usize>,
        hand_len: Option<usize>,
    ) -> Result<FirstState, HashSet<NewError>> {
        #[inline]
        fn first_player_selector(max_matching_players: &Indexes) -> usize {
            let len = max_matching_players.len();
            if len == 0 {
                unreachable!("max_matching_players should not be empty.");
            }
            max_matching_players
                .into_iter()
                .nth(rand::thread_rng().gen_range(0..len))
                .copied()
                .unwrap_or_else(|| unreachable!("max_matching_players should not be empty."))
        }
        FirstState::new(
            players_len,
            unique_tile_copied_count,
            hand_len,
            first_player_selector,
        )
    }

    /// Checks that `players_len`, `unique_tile_copied_count`, and `hand_len` are all
    /// non-zero, that the number of tiles requested (`player_len * hand_len`) is less than
    /// the number of tiles in `bag` ([`TILES_LEN`] ` * unique_tile_copied_count`), and that
    /// the number of tiles in `bag` is not large enough to cause overflow.
    ///
    /// Creates a `bag` of tiles, and then draws tiles from `bag` to create a hand for each player.
    ///
    /// Finds the maximum number of matching tiles in each hand, and then finds the maximum
    /// of those maximums. Then, finds all players with that maximum number of matches,
    /// and then selects one of those players with `first_player_selector`.
    ///
    /// When `unique_tile_copied_count` and/or `hand_len` are [`None`], default values
    /// [`DEFAULT_UNIQUE_TILE_COPIED_COUNT`] and [`DEFAULT_HAND_LEN`] are used respectively.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each tile in `bag`.
    /// * `hand_len`: The number of tiles each player will have in their hand.
    /// * `first_player_selector`: Selects the first player from a set of possible first players.
    ///
    /// # Errors
    ///
    /// * [`NewError::EmptyPlayers`] Attempting to start with empty `players`.
    /// * [`NewError::EmptyBag`] Attempting to start with an empty `bag`.
    /// * [`NewError::EmptyHands`] Attempting to start with empty `hands`.
    /// * [`NewError::NotEnoughTiles`] Attempting to request more tiles than tiles in `bag`.
    /// * [`NewError::TooManyTiles`] Attempting to add too many tiles into `bag`
    /// * [`NewError::CurrentPlayerNotMaxMatchingPlayers`] Attempt to select a current player
    /// not in `max_matching_players`.
    pub fn new<F>(
        players_len: usize,
        unique_tile_copied_count: Option<usize>,
        hand_len: Option<usize>,
        first_player_selector: F,
    ) -> Result<FirstState, HashSet<NewError>>
    where
        F: FnOnce(&Indexes) -> usize,
    {
        let unique_tile_copied_count =
            unique_tile_copied_count.unwrap_or(DEFAULT_UNIQUE_TILE_COPIED_COUNT);
        let hand_len = hand_len.unwrap_or(DEFAULT_HAND_LEN);
        FirstState::check(players_len, unique_tile_copied_count, hand_len)?;

        let (bag, hands) =
            FirstState::new_bag_and_hands(players_len, unique_tile_copied_count, hand_len);
        let (max_matches, max_matching_players) =
            FirstState::new_max_matches_and_max_matching_players(&hands);

        let current_player = first_player_selector(&max_matching_players);

        // check whether selected current_player is in max_matching_players
        if !max_matching_players.contains(&current_player) {
            return Err(set! { NewError::CurrentPlayerNotMaxMatchingPlayers {
                current_player,
                max_matching_players
            }});
        }

        Ok(FirstState {
            bag,
            hands,
            max_matches,
            current_player,
        })
    }

    /// # Returns
    ///
    /// The index of the player whose turn it is.
    #[inline]
    pub fn current_player(&self) -> usize {
        self.current_player
    }

    /// Checks that `players_len`, `unique_tile_copied_count`, and `hand_len` are all
    /// non-zero, that the number of tiles requested (`player_len * hand_len`) is less than
    /// the number of tiles in `bag` ([`Tile::TILES_LEN`] ` * unique_tile_copied_count`), and that
    /// the number of tiles in `bag` is not large enough to cause overflow.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of `players` in the game.
    /// * `unique_tile_copied_count`: The number of copies of each tile in `bag`.
    /// * `hand_len`: The number of tiles each player will have in their hand.
    ///
    /// # Errors
    ///
    /// * [`NewError::EmptyPlayers`] Attempting to start with empty `players`.
    /// * [`NewError::EmptyBag`] Attempting to start with an empty `bag`.
    /// * [`NewError::EmptyHands`] Attempting to start with empty `hands`.
    /// * [`NewError::NotEnoughTiles`] Attempting to request more tiles than tiles in `bag`.
    /// * [`NewError::TooManyTiles`] Attempting to add too many tiles into `bag`
    #[inline]
    fn check(
        players_len: usize,
        unique_tile_copied_count: usize,
        hand_len: usize,
    ) -> Result<(), HashSet<NewError>> {
        let mut errors = HashSet::with_capacity(5);
        if players_len == 0 {
            errors.insert(NewError::EmptyPlayers);
        }
        if unique_tile_copied_count == 0 {
            errors.insert(NewError::EmptyBag);
        }
        if hand_len == 0 {
            errors.insert(NewError::EmptyHands);
        }

        let requested_tiles = players_len * hand_len;
        let tiles_in_bag = TILES_LEN * unique_tile_copied_count;

        if requested_tiles > tiles_in_bag {
            errors.insert(NewError::NotEnoughTiles {
                requested_tiles,
                tiles_in_bag,
            });
        }

        // it is possible to cause an overflow in first_play above this limit
        // also, this program should slow down long before this limit is reached
        if tiles_in_bag > MAXIMUM_BAG_TILES {
            errors.insert(NewError::TooManyTiles { tiles_in_bag });
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }

    /// Creates a bag of tiles, and then draws tiles from `bag` to create a hand for each player.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each tile in `bag`.
    /// * `hand_len`: The number of tiles each player will have in their hand.
    ///
    /// # Returns
    ///
    /// A bag of all the tiles that haven't been removed yet and a vector of hands, where each hand
    /// is a vector of tiles randomly taken from `bag`.
    #[inline]
    fn new_bag_and_hands(
        players_len: usize,
        unique_tile_copied_count: usize,
        hand_len: usize,
    ) -> (Bag, Hands) {
        let mut bag = tiles()
            .into_iter()
            .flat_map(|tile| vec![tile; unique_tile_copied_count])
            .collect_vec()
            .tap_mut(|bag| bag.shuffle(&mut rand::thread_rng()));
        let hands = bag
            .drain(bag.len() - (players_len * hand_len)..)
            .chunks(hand_len)
            .into_iter()
            .map(|chunk| chunk.collect())
            .collect();
        (bag, hands)
    }

    /// Finds the maximum number of matching tiles in each hand, and then finds the maximum
    /// of those maximums. Then, finds all players with that maximum number of matches,
    /// and then selects one of those players at random.
    ///
    /// # Arguments
    ///
    /// * `hands`: A vector of hands, where each hand is a vector of tiles.
    ///
    /// # Returns
    ///
    /// A vector of the maximum number of matching tiles in each player's hand on the first turn and
    /// the index of a random player with the maximum of those maximums.
    #[inline]
    fn new_max_matches_and_max_matching_players(hands: &Hands) -> (MaxMatches, Indexes) {
        // Produce maximum number of matches in each hand
        let mut max_matches = MaxMatches::with_capacity(PLAYER_CAPACITY);
        // and maximum of those maximums
        let mut max_max_match = 0;

        for hand in hands {
            // Find maximum count of colors or shapes
            let mut count_colors = [0; Color::COLORS_LEN];
            let mut count_shapes = [0; Shape::SHAPES_LEN];
            let mut max_match = 0;

            // Prevent tiles from matching with themselves
            for &(color, shape) in hand.iter().unique() {
                let index = color as usize;
                count_colors[index] += 1;
                max_match = cmp::max(max_match, count_colors[index]);

                let index = shape as usize;
                count_shapes[index] += 1;
                max_match = cmp::max(max_match, count_shapes[index]);
            }
            max_matches.push(max_match);

            // Find maximum of maximums between hands
            max_max_match = cmp::max(max_max_match, max_match);
        }

        // Filter players with maximum of maximum matches
        let max_matching_players: Indexes = max_matches
            .iter()
            .enumerate()
            .filter(|(_, &max_match)| max_max_match == max_match)
            .map(|(index, _)| index)
            .collect();

        (max_matches, max_matching_players)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_macro::set;
    use rand::Rng;

    #[test]
    fn empty_players() {
        let actual_error = FirstState::new_random_first_player_selector(0, None, None).unwrap_err();

        let expected_error = set! { NewError::EmptyPlayers };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn empty_bag_not_enough_tiles() {
        let actual_error =
            FirstState::new_random_first_player_selector(1, Some(0), Some(1)).unwrap_err();

        let expected_error = set! {
            NewError::EmptyBag,
            NewError::NotEnoughTiles {
                requested_tiles: 1,
                tiles_in_bag: 0,
            },
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn empty_hands() {
        let actual_error =
            FirstState::new_random_first_player_selector(1, None, Some(0)).unwrap_err();

        let expected_error = set! { NewError::EmptyHands };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn empty_players_empty_bag_empty_hands() {
        let actual_error =
            FirstState::new_random_first_player_selector(0, Some(0), Some(0)).unwrap_err();

        let expected_error = set! {
            NewError::EmptyPlayers,
            NewError::EmptyBag,
            NewError::EmptyHands
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn not_enough_tiles() {
        let players_len = 10;
        let unique_tile_copied_count = 2;
        let hand_len = 20;

        let actual_error = FirstState::new_random_first_player_selector(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
        )
        .unwrap_err();

        let expected_error = set! { NewError::NotEnoughTiles {
            requested_tiles: players_len * hand_len,
            tiles_in_bag: unique_tile_copied_count * TILES_LEN,
        }};
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn too_many_tiles() {
        let players_len = 10;
        let unique_tile_copied_count = MAXIMUM_BAG_TILES;
        let hand_len = 20;

        let actual_error = FirstState::new_random_first_player_selector(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
        )
        .unwrap_err();

        let expected_error = set! { NewError::TooManyTiles {
            tiles_in_bag: unique_tile_copied_count * TILES_LEN,
        }};
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn current_player_not_max_matching_players() {
        let players_len = 4;
        let unique_tile_copied_count = 3;
        let hand_len = 6;
        fn first_player_selector(max_matching_players: &Indexes) -> usize {
            max_matching_players.iter().rev().next().copied().unwrap() + 1
        }

        let actual_error = FirstState::new(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
            first_player_selector,
        )
        .unwrap_err();

        assert_eq!(1, actual_error.len());
        let actual_error = actual_error.into_iter().next().unwrap();
        assert!(matches!(
            actual_error,
            NewError::CurrentPlayerNotMaxMatchingPlayers { .. }
        ));
    }

    #[test]
    fn new_none() {
        let mut rng = rand::thread_rng();
        let players_len = rng.gen_range(2..=4);

        let default =
            FirstState::new_random_first_player_selector(players_len, None, None).unwrap();
        let new = FirstState::new_random_first_player_selector(
            players_len,
            Some(DEFAULT_UNIQUE_TILE_COPIED_COUNT),
            Some(DEFAULT_HAND_LEN),
        )
        .unwrap();

        assert_eq!(default.bag.len(), new.bag.len());
        assert_eq!(players_len, default.hands.len());
        assert_eq!(players_len, new.hands.len());
        for index in 0..players_len {
            assert_eq!(default.hands[index].len(), new.hands[index].len());
        }
    }

    #[test]
    fn new_some() {
        let players_len = 4;
        let unique_tile_copied_count = 3;
        let hand_len = 6;

        let first_state = FirstState::new_random_first_player_selector(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
        )
        .unwrap();

        let max_max_match = first_state.max_matches.clone().into_iter().max().unwrap();
        let max_matching_players: HashSet<usize> = first_state
            .max_matches
            .iter()
            .enumerate()
            .filter(|(_, &max_match)| max_max_match == max_match)
            .map(|(index, _)| index)
            .collect();

        assert_eq!(
            TILES_LEN * unique_tile_copied_count - (hand_len * players_len),
            first_state.bag.len()
        );
        assert_eq!(players_len, first_state.hands.len());
        for player in 0..players_len {
            assert_eq!(hand_len, first_state.hands[player].len());
        }
        assert!(max_matching_players.contains(&first_state.current_player));
    }
}
