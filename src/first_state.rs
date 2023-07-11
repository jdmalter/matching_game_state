use crate::{tiles, Bag, Color, Hands, MaxMatches, Shape, PLAYER_CAPACITY, TILES_LEN, TILE_LIMIT};
use itertools::{Chunk, Itertools};
use map_macro::hash_set;
use rand::seq::SliceRandom;
use rand::Rng;
use std::cmp;
use std::collections::{BTreeSet, HashSet};

pub use first_play::*;
pub use first_view::*;

mod first_play;
mod first_view;
#[cfg(test)]
mod test_setup;

/// Owns game state on the first turn and implements methods. Created from [FirstState::new].
#[derive(Debug)]
pub struct FirstState {
    /// This is a bag of all the [tiles](crate::Tile) that haven't been removed yet.
    bag: Bag,
    /// A vector of hands for each player, where each hand is
    /// a vector of [tiles](crate::Tile).
    hands: Hands,
    /// A vector of the maximum number of matching [tiles](crate::Tile)
    /// in each player's hand.
    max_matches: MaxMatches,
    /// The index of the player whose turn it is.
    current_player: usize,
}

/// Describes the reason why [FirstState] could not be created.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NewError {
    /// Attempting [to start](FirstState::new) with empty players.
    EmptyPlayers,
    /// Attempting [to start](FirstState::new) with an empty bag.
    EmptyBag,
    /// Attempting [to start](FirstState::new) with empty hands.
    EmptyHands,
    /// Attempting to request more [tiles](crate::Tile) than [tiles](crate::Tile) in the bag.
    NotEnoughTiles {
        /// The number of [tiles](crate::Tile) requested for hands.
        requested_tiles: usize,
        /// The number of available [tiles](crate::Tile) in the bag.
        tiles_in_bag: usize,
    },
    /// Attempting to create more than [tiles](crate::Tile) in the bag
    /// than the [tile limit](TILE_LIMIT).
    TooManyTiles {
        /// The number of [tiles](crate::Tile) being created in the bag.
        tiles_in_bag: usize,
    },
    /// Attempt to select some current player not in `max_matching_players`.
    CurrentPlayerNotMaxMatchingPlayers {
        /// The index of the player whose turn it is.
        current_player: usize,
        /// An ordered set of indexes of players who all hold
        /// the maximum number of matching [tiles](crate::Tile).
        max_matching_players: BTreeSet<usize>,
    },
}

/// The default of number of times a unique [tile](crate::Tile) is copied in the game.
/// `3` copies of each [tile](crate::Tile).
///
/// # See Also
///
/// * [FirstState::new]
pub const DEFAULT_UNIQUE_TILE_COPIED_COUNT: usize = 3;
/// The default maximum number of [tiles](crate::Tile) in a hand.
/// `6` [tiles](crate::Tile) per hand.
///
/// # See Also
///
/// * [FirstState::new]
pub const DEFAULT_HAND_LEN: usize = 6;

impl FirstState {
    /// Checks that `players_len`, `unique_tile_copied_count`, and `hand_len` are all
    /// non-zero, that the number of [tiles](crate::Tile) requested (`player_len * hand_len`) is
    /// less than the number of [tiles](crate::Tile) in the bag
    /// ([TILES_LEN] ` * unique_tile_copied_count`), and that the number of [tiles](crate::Tile)
    /// in the bag is less than or equal to the [tile limit](TILE_LIMIT).
    ///
    /// Creates a bag of [tiles](crate::Tile), and then draws [tiles](crate::Tile)
    /// from the bag to create a hand for each player.
    ///
    /// Finds the maximum number of matching [tiles](crate::Tile) in each hand,
    /// and then finds the maximum of those maximums. Then, finds all players with that
    /// maximum number of matches, and then selects one of those players at random.
    ///
    /// When `unique_tile_copied_count` and/or `hand_len` are [None], default values
    /// [DEFAULT_UNIQUE_TILE_COPIED_COUNT] and [DEFAULT_HAND_LEN] are used respectively.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each [tile](crate::Tile)
    /// in the bag.
    /// * `hand_len`: The number of [tiles](crate::Tile) each player will have
    /// in their hand.
    ///
    /// # Errors
    ///
    /// * [NewError::EmptyPlayers] Attempting [to start](FirstState::new_random_first_player)
    /// with empty players.
    /// * [NewError::EmptyBag] Attempting [to start](FirstState::new_random_first_player)
    /// with an empty bag.
    /// * [NewError::EmptyHands] Attempting [to start](FirstState::new_random_first_player)
    /// with empty hands.
    /// * [NewError::NotEnoughTiles] Attempting to request more [tiles](crate::Tile)
    /// than [tiles](crate::Tile) in the bag.
    /// * [NewError::TooManyTiles] Attempting to create more than [tiles](crate::Tile)
    /// in the bag than the [tile limit](TILE_LIMIT).
    ///
    /// # See Also
    ///
    /// * [FirstState::new]
    pub fn new_random_first_player(
        players_len: usize,
        unique_tile_copied_count: Option<usize>,
        hand_len: Option<usize>,
    ) -> Result<FirstState, HashSet<NewError>> {
        fn first_player_selector(max_matching_players: &BTreeSet<usize>) -> usize {
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
    /// non-zero, that the number of [tiles](crate::Tile) requested (`player_len * hand_len`) is
    /// less than the number of [tiles](crate::Tile) in the bag
    /// ([TILES_LEN] ` * unique_tile_copied_count`), and that the number of [tiles](crate::Tile)
    /// in the bag is not large enough to cause overflow.
    ///
    /// Creates a bag of [tiles](crate::Tile), and then draws [tiles](crate::Tile)
    /// from the bag to create a hand for each player.
    ///
    /// Finds the maximum number of matching [tiles](crate::Tile) in each hand,
    /// and then finds the maximum of those maximums. Then, finds all players with that
    /// maximum number of matches, and then selects one of those players
    /// with `first_player_selector`.
    ///
    /// When `unique_tile_copied_count` and/or `hand_len` are [None], default values
    /// [DEFAULT_UNIQUE_TILE_COPIED_COUNT] and [DEFAULT_HAND_LEN] are used respectively.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each [tile](crate::Tile)
    /// in the bag.
    /// * `hand_len`: The number of [tiles](crate::Tile) each player will have
    /// in their hand.
    /// * `first_player_selector`: Selects the first player from a set of possible first players.
    ///
    /// # Errors
    ///
    /// * [NewError::EmptyPlayers] Attempting [to start](FirstState::new) with empty players.
    /// * [NewError::EmptyBag] Attempting [to start](FirstState::new) with an empty bag.
    /// * [NewError::EmptyHands] Attempting [to start](FirstState::new) with empty hands.
    /// * [NewError::NotEnoughTiles] Attempting to request more [tiles](crate::Tile)
    /// than [tiles](crate::Tile) in the bag.
    /// * [NewError::TooManyTiles] Attempting to create more than [tiles](crate::Tile)
    /// in the bag than the [tile limit](TILE_LIMIT).
    /// * [NewError::CurrentPlayerNotMaxMatchingPlayers] Attempt to select some current player
    /// not in `max_matching_players`.
    ///
    /// # See Also
    ///
    /// * [FirstState::new_random_first_player]
    pub fn new(
        players_len: usize,
        unique_tile_copied_count: Option<usize>,
        hand_len: Option<usize>,
        first_player_selector: impl FnOnce(&BTreeSet<usize>) -> usize,
    ) -> Result<FirstState, HashSet<NewError>> {
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
            return Err(hash_set! { NewError::CurrentPlayerNotMaxMatchingPlayers {
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

    /// The index of the player whose turn it is.
    pub fn current_player(&self) -> usize {
        self.current_player
    }

    /// Checks that `players_len`, `unique_tile_copied_count`, and `hand_len` are all
    /// non-zero, that the number of [tiles](crate::Tile) requested (`player_len * hand_len`) is
    /// less than the number of [tiles](crate::Tile) in the bag
    /// ([TILES_LEN] ` * unique_tile_copied_count`), and that the number of [tiles](crate::Tile)
    /// in the bag is less than or equal to the [tile limit](TILE_LIMIT).
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each [tile](crate::Tile)
    /// in the bag.
    /// * `hand_len`: The number of [tiles](crate::Tile) each player will have
    /// in their hand.
    ///
    /// # Errors
    ///
    /// * [NewError::EmptyPlayers] Attempting [to start](FirstState::new) with empty players.
    /// * [NewError::EmptyBag] Attempting [to start](FirstState::new) with an empty bag.
    /// * [NewError::EmptyHands] Attempting [to start](FirstState::new) with empty hands.
    /// * [NewError::NotEnoughTiles] Attempting to request more [tiles](crate::Tile)
    /// than [tiles](crate::Tile) in the bag.
    /// * [NewError::TooManyTiles] Attempting to create more than [tiles](crate::Tile)
    /// in the bag than the [tile limit](TILE_LIMIT).
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

        if tiles_in_bag > TILE_LIMIT {
            errors.insert(NewError::TooManyTiles { tiles_in_bag });
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }

    /// Creates a bag of [tiles](crate::Tile), and then draws [tiles](crate::Tile)
    /// from the bag to create a hand for each player.
    ///
    /// # Arguments
    ///
    /// * `players_len`: The number of players in the game.
    /// * `unique_tile_copied_count`: The number of copies of each [tile](crate::Tile)
    /// in the bag.
    /// * `hand_len`: The number of [tiles](crate::Tile) each player will have
    /// in their hand.
    ///
    /// # Returns
    ///
    /// A bag of all the [tiles](crate::Tile) that haven't been removed yet and
    /// a vector of hands, where each hand
    /// is a vector of [tiles](crate::Tile) randomly taken from the bag.
    fn new_bag_and_hands(
        players_len: usize,
        unique_tile_copied_count: usize,
        hand_len: usize,
    ) -> (Bag, Hands) {
        let mut bag = tiles()
            .into_iter()
            .flat_map(|tile| vec![tile; unique_tile_copied_count])
            .collect_vec();
        bag.shuffle(&mut rand::thread_rng());
        let hands = bag
            .drain(bag.len() - (players_len * hand_len)..)
            .chunks(hand_len)
            .into_iter()
            .map(Chunk::collect)
            .collect();
        (bag, hands)
    }

    /// Finds the maximum number of matching [tiles](crate::Tile) in each hand,
    /// and then finds the maximum of those maximums.
    ///
    /// # Arguments
    ///
    /// * `hands`: A vector of hands, where each hand is
    /// a vector of [tiles](crate::Tile).
    ///
    /// # Returns
    ///
    /// A vector of the maximum number of matching [tiles](crate::Tile)
    /// in each player's hand on the first turn and
    /// the indexes of players with the maximum of those maximums.
    fn new_max_matches_and_max_matching_players(hands: &Hands) -> (MaxMatches, BTreeSet<usize>) {
        // Produce the maximum number of matches in each hand
        let mut max_matches = MaxMatches::with_capacity(PLAYER_CAPACITY);
        // and maximum of those maximums
        let mut max_max_match: usize = 0;

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
        let max_matching_players = max_matches
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
    use map_macro::hash_set;
    use rand::Rng;

    #[test]
    fn empty_players() {
        test_new_one_error(0, None, None, NewError::EmptyPlayers);
    }

    #[test]
    fn empty_bag_not_enough_tiles() {
        test_new_errors(
            1,
            Some(0),
            Some(1),
            hash_set! {
              NewError::EmptyBag,
              NewError::NotEnoughTiles {
                requested_tiles: 1,
                tiles_in_bag: 0,
              },
            },
        );
    }

    #[test]
    fn empty_hands() {
        test_new_one_error(1, None, Some(0), NewError::EmptyHands);
    }

    #[test]
    fn empty_players_empty_bag_empty_hands() {
        test_new_errors(
            0,
            Some(0),
            Some(0),
            hash_set! { NewError::EmptyPlayers, NewError::EmptyBag, NewError::EmptyHands },
        );
    }

    #[test]
    fn not_enough_tiles() {
        let players_len = 10;
        let unique_tile_copied_count = 2;
        let hand_len = 20;

        test_new_one_error(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
            NewError::NotEnoughTiles {
                requested_tiles: players_len * hand_len,
                tiles_in_bag: unique_tile_copied_count * TILES_LEN,
            },
        );
    }

    #[test]
    fn too_many_tiles() {
        let players_len = 10;
        let unique_tile_copied_count = TILE_LIMIT;
        let hand_len = 20;

        test_new_one_error(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
            NewError::TooManyTiles {
                tiles_in_bag: unique_tile_copied_count * TILES_LEN,
            },
        );
    }

    #[test]
    fn current_player_not_max_matching_players() {
        let players_len = 4;
        let unique_tile_copied_count = 3;
        let hand_len = 6;
        fn first_player_selector(max_matching_players: &BTreeSet<usize>) -> usize {
            max_matching_players
                .iter()
                .rev()
                .next()
                .copied()
                .expect("new should always provide non-empty max_matching_players")
                + 1
        }

        let actual_error = FirstState::new(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
            first_player_selector,
        )
        .expect_err("new should return Err");

        assert_eq!(1, actual_error.len());
        let actual_error = actual_error.into_iter().next().expect(
            "actual_error should contain at least 1 item since \
                  assert_eq!(1, actual_error.len()) must be true to reach this point in code",
        );
        assert!(matches!(
            actual_error,
            NewError::CurrentPlayerNotMaxMatchingPlayers { .. }
        ));
    }

    #[test]
    fn new_none() {
        let players_len = rand::thread_rng().gen_range(2..=PLAYER_CAPACITY);

        let none = FirstState::new_random_first_player(players_len, None, None)
            .expect("new should return Ok");
        let some = FirstState::new_random_first_player(
            players_len,
            Some(DEFAULT_UNIQUE_TILE_COPIED_COUNT),
            Some(DEFAULT_HAND_LEN),
        )
        .expect("new should return Ok");

        assert_eq!(none.bag.len(), some.bag.len());
        assert_eq!(players_len, none.hands.len());
        assert_eq!(players_len, some.hands.len());
        for index in 0..players_len {
            assert_eq!(none.hands[index].len(), some.hands[index].len());
        }
    }

    #[test]
    fn new_some() {
        let players_len = 4;
        let unique_tile_copied_count = 3;
        let hand_len = 6;

        let first_state = FirstState::new_random_first_player(
            players_len,
            Some(unique_tile_copied_count),
            Some(hand_len),
        )
        .expect("new should return Ok");

        assert!(first_state.max_matches.len() > 0);
        let max_max_match = first_state.max_matches.clone().into_iter().max().expect(
            "max_max_match should exist since first_state.max_matches.len() > 0 must be true",
        );
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

    fn test_new_one_error(
        players_len: usize,
        unique_tile_copied_count: Option<usize>,
        hand_len: Option<usize>,
        expected_error: NewError,
    ) {
        test_new_errors(
            players_len,
            unique_tile_copied_count,
            hand_len,
            hash_set! { expected_error },
        );
    }

    fn test_new_errors(
        players_len: usize,
        unique_tile_copied_count: Option<usize>,
        hand_len: Option<usize>,
        expected_error: HashSet<NewError>,
    ) {
        let actual_error =
            FirstState::new_random_first_player(players_len, unique_tile_copied_count, hand_len)
                .expect_err("new_random_first_player_selector should only return Err");

        assert_eq!(expected_error, actual_error);
    }
}
