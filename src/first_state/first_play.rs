use crate::{
    batch_continuous_decreasing_range, batch_continuous_increasing_range, check_line,
    find_component_minimums_and_maximums, find_coordinate_by_minimum_distance,
    partition_by_coordinates, possible_plays, Coordinate, FirstState, NextState, Plays, Points,
    HOLES_LIMIT,
};
use itertools::Itertools;
use std::collections::{BTreeSet, HashSet};

/// Describes the reason why the [first play](FirstState::first_play) could not be executed.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FirstPlayError {
    /// Attempting [to play](FirstState::first_play) no [tiles](crate::Tile).
    EmptyPlays,
    /// Attempting [to play](FirstState::first_play) [tiles](crate::Tile) not
    /// in the current player's hand.
    IndexesOutOfBounds {
        /// [Plays](Plays) where the index is greater than or equal to `hand_len`.
        indexes_out_of_bounds: Plays,
    },
    /// Attempting [to play](FirstState::first_play) [tiles](crate::Tile) too far away from
    /// the center of the board.
    CoordinatesOutOfBounds {
        /// [Plays](Plays) where the absolute value of a component in a [coordinate](Coordinate) is
        /// greater than or equal to the [coordinate limit](crate::COORDINATE_LIMIT).
        coordinates_out_of_bounds: Plays,
    },
    /// Not attempting [to play](FirstState::first_play) some [tile](crate::Tile) at the origin.
    OriginNotIncluded,
    /// Not attempting [to play](FirstState::first_play) max match of legal [plays](Plays).
    NotMaxMatching {
        /// Alternative acceptable [plays](Plays) of whose length is the maximum possible
        /// legal length from the current player's hand.
        max_matching_plays: BTreeSet<BTreeSet<usize>>,
    },
    /// Attempting to only [play](FirstState::first_play) illegal [plays](Plays).
    NoLegalPlays,
    /// Not attempting [to play](FirstState::first_play) a single [tile](crate::Tile)
    /// or [tiles](crate::Tile) in a line.
    NoLegalLines,
    /// Attempting [to play](FirstState::first_play) [tiles](crate::Tile) in a line but not
    /// the same connected line.
    Holes {
        /// Ranges of [coordinates](Coordinate) in line that are not in [plays](Plays).
        holes: BTreeSet<(Coordinate, Coordinate)>,
    },
    /// Attempting [to play](FirstState::first_play) duplicate [tiles](crate::Tile) in a line.
    Duplicates {
        /// Groups of [coordinates](Coordinate) where [tiles](crate::Tile) are the same.
        duplicates: BTreeSet<BTreeSet<Coordinate>>,
    },
    /// Attempting [to play](FirstState::first_play) a line where [tiles](crate::Tile) are
    /// not either the same [shape](crate::Shape) or the same [color](crate::Color).
    MultipleMatching {
        /// Groups of [coordinates](Coordinate) where [tiles](crate::Tile) match each other
        /// but not other groups.
        multiple_matching: BTreeSet<BTreeSet<Coordinate>>,
    },
}

impl FirstState {
    /// Checks if the [plays](Plays) are valid, then removes the [tiles](crate::Tile)
    /// from the current player's hand, inserts those [tiles](crate::Tile)
    /// into the board at their [coordinate](Coordinate) minus
    /// the average [coordinate](Coordinate) (centers the [plays](Plays) in
    /// the board around the origin), attempts to fill the current player's
    /// hand up to its previous length, adds points earned by the [play](Plays)
    /// to the current player, and advances to the next player if the game has not ended.
    ///
    /// # Points Calculation
    ///
    /// The number of points from a line is the number of [tiles](crate::Tile) in that line. If the
    /// line creates a full match on the board where a line contains either
    /// [every color](crate::Color::colors) or [every shape](crate::Shape::shapes), an extra
    /// [full match bonus](crate::FULL_MATCH_BONUS) is earned.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of [tiles](crate::Tile) to be played
    /// to [coordinates](Coordinate) on the board.
    ///
    /// # Errors
    ///
    /// * [FirstPlayError::EmptyPlays] Attempting [to play](FirstState::first_play) no
    /// [tiles](crate::Tile).
    /// * [FirstPlayError::IndexesOutOfBounds] Attempting [to play](FirstState::first_play)
    /// [tiles](crate::Tile) not in the current player's hand.
    /// * [FirstPlayError::CoordinatesOutOfBounds] Attempting [to play](FirstState::first_play)
    /// [tiles](crate::Tile) too far away from the center of the board.
    /// * [FirstPlayError::OriginNotIncluded] Not attempting [to play](FirstState::first_play)
    /// some [tile](crate::Tile) at the origin.
    /// * [FirstPlayError::NotMaxMatching] Not attempting [to play](FirstState::first_play)
    /// max match of legal [plays](Plays).
    /// * [FirstPlayError::NoLegalPlays] Attempting to only [play](FirstState::first_play)
    /// illegal [plays](Plays).
    /// * [FirstPlayError::NoLegalLines] Not attempting [to play](FirstState::first_play)
    /// [tiles](crate::Tile) in a point or a line.
    /// * [FirstPlayError::Holes] Attempting [to play](FirstState::first_play) [tiles](crate::Tile)
    /// in a line but not the same connected line.
    /// * [FirstPlayError::Duplicates] Attempting [to play](FirstState::first_play)
    /// duplicate [tiles](crate::Tile) in a line.
    /// * [FirstPlayError::MultipleMatching] Attempting [to play](FirstState::first_play) a line
    /// where [tiles](crate::Tile) are not either the same [shape](crate::Shape)
    /// or the same [color](crate::Color).
    ///
    /// # Returns
    ///
    /// The [next state](NextState) of the game after the [play](Plays).
    pub fn first_play(
        mut self,
        plays: &Plays,
    ) -> Result<NextState, (Self, HashSet<FirstPlayError>)> {
        let first_play_points = match self.check_plays(plays) {
            Ok(points) => points,
            Err(errors) => return Err((self, errors)),
        };

        let hand = &mut self.hands[self.current_player];
        let board = plays
            .iter()
            .rev()
            .map(|(&index, &coordinate)| (coordinate, hand.remove(index)))
            .collect();
        // when the bag is empty, no more tiles will be drained
        hand.extend(self.bag.drain(self.bag.len().saturating_sub(plays.len())..));

        let mut points: Points = self.hands.iter().map(|_| 0).collect();
        points[self.current_player] = first_play_points;
        self.current_player = (self.current_player + 1) % self.hands.len();

        Ok(NextState::new(
            self.bag,
            board,
            points,
            self.hands,
            self.current_player,
        ))
    }

    /// Takes a bimap of indexes of [tiles](crate::Tile) to be played to [coordinates](Coordinate)
    /// on the board and returns earned points
    /// if the bimap is a legal line. Otherwise, it returns the errors.
    ///
    /// # Points Calculation
    ///
    /// The number of points from a line is the number of [tiles](crate::Tile) in that line.
    /// If the line creates a full match on the board where a line contains
    /// either [every color](crate::Color::colors) or [every shape](crate::Shape::shapes),
    /// an extra [FULL_MATCH_BONUS](FULL_MATCH_BONUS) points are earned.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of [tiles](crate::Tile) to be played
    /// to [coordinates](Coordinate) on the board.
    ///
    /// # Errors
    ///
    /// * [FirstPlayError::EmptyPlays] Attempting [to play](FirstState::first_play) no
    /// [tiles](crate::Tile).
    /// * [FirstPlayError::IndexesOutOfBounds] Attempting [to play](FirstState::first_play)
    /// [tiles](crate::Tile) not in the current player's hand.
    /// * [FirstPlayError::CoordinatesOutOfBounds] Attempting [to play](FirstState::first_play)
    /// [tiles](Tile) too far away from the center of the board.
    /// * [FirstPlayError::OriginNotIncluded] Not attempting [to play](FirstState::first_play)
    /// some [tile](crate::Tile) at the origin.
    /// * [FirstPlayError::NotMaxMatching] Not attempting [to play](FirstState::first_play)
    /// max match of legal [plays](Plays).
    /// * [FirstPlayError::NoLegalPlays] Attempting to only [play](FirstState::first_play)
    /// illegal [plays](Plays).
    /// * [FirstPlayError::NoLegalLines] Not attempting [to play](FirstState::first_play)
    /// [tiles](crate::Tile) in a point or a line.
    /// * [FirstPlayError::Holes] Attempting [to play](FirstState::first_play) [tiles](crate::Tile)
    /// in a line but not the same connected line.
    /// * [FirstPlayError::Duplicates] Attempting [to play](FirstState::first_play)
    /// duplicate [tiles](crate::Tile) in a line.
    /// * [FirstPlayError::MultipleMatching] Attempting [to play](FirstState::first_play) a line
    /// where [tiles](crate::Tile) are not either the same [shape](crate::Shape)
    /// or the same [color](crate::Color).
    ///
    /// # Returns
    ///
    /// The earned points from [plays](Plays).
    fn check_plays(&self, plays: &Plays) -> Result<usize, HashSet<FirstPlayError>> {
        let mut errors = HashSet::with_capacity(9);

        if plays.is_empty() {
            errors.insert(FirstPlayError::EmptyPlays);
            return Err(errors);
        }

        let hand = &self.hands[self.current_player];
        let hand_len = hand.len();
        let indexes_out_of_bounds: Plays = plays
            .left_range(hand_len..)
            .map(|(&index, &coordinate)| (index, coordinate))
            .collect();
        if !indexes_out_of_bounds.is_empty() {
            errors.insert(FirstPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds,
            });
        }

        let (coordinates_in_bounds, coordinates_out_of_bounds) = partition_by_coordinates(plays);
        if !coordinates_out_of_bounds.is_empty() {
            errors.insert(FirstPlayError::CoordinatesOutOfBounds {
                coordinates_out_of_bounds,
            });
        }

        if !coordinates_in_bounds.contains_right(&(0, 0)) {
            errors.insert(FirstPlayError::OriginNotIncluded);
        }

        let legal_plays: Plays = coordinates_in_bounds
            .left_range(..hand_len)
            .map(|(&index, &coordinate)| (index, coordinate))
            .collect();

        let max_match = self.max_matches[self.current_player];
        if legal_plays.len() != max_match {
            let max_matching_plays = possible_plays(hand.clone(), max_match);
            errors.insert(FirstPlayError::NotMaxMatching { max_matching_plays });
        }

        let Some(component_minimums_and_maximums) =
            find_component_minimums_and_maximums(legal_plays.right_values().copied()) else {
            errors.insert(FirstPlayError::NoLegalPlays);
            return Err(errors);
        };

        let Some(mid_coordinate)
            = find_coordinate_by_minimum_distance(legal_plays.right_values().copied()) else {
            errors.insert(FirstPlayError::NoLegalPlays);
            return Err(errors);
        };

        let Some(holes) =
            FirstState::find_holes(&legal_plays, component_minimums_and_maximums,
                                   mid_coordinate) else {
            errors.insert(FirstPlayError::NoLegalLines);
            return Err(errors);
        };

        if !holes.is_empty() {
            errors.insert(FirstPlayError::Holes { holes });
        }

        // line is the same regardless of whether legal_plays is horizontal or vertical
        let line = legal_plays
            .into_iter()
            .map(|(index, coordinate)| (coordinate, hand[index]))
            .collect();

        let mut total_points = 0;

        match check_line(&line) {
            Err((duplicates, multiple_matching)) => {
                if !duplicates.is_empty() {
                    errors.insert(FirstPlayError::Duplicates { duplicates });
                }
                if !multiple_matching.is_empty() {
                    errors.insert(FirstPlayError::MultipleMatching { multiple_matching });
                }
            }
            Ok(points) => {
                total_points += points;
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(total_points)
    }

    /// Finds holes in the line being played. If there are no holes, [None] is returned.
    ///
    /// # Arguments
    ///
    /// * `legal_plays`: A bimap of indexes of [tiles](crate::Tile) to be played
    /// to [coordinates](Coordinate) on the board where each play violates no rules.
    /// * `min_x`: The minimum x component in a potential line
    /// * `min_y`: The minimum y component in a potential line
    /// * `max_x`: The maximum x component in a potential line
    /// * `max_y`: The maximum y component in a potential line    
    /// * `mid_x`: The x component in a potential line of the [coordinate](Coordinate)
    /// with the minimum distance from the origin
    /// * `mid_y`: The y component in a potential line of the [coordinate](Coordinate)
    /// with the minimum distance from the origin
    ///
    /// # Returns
    ///
    /// Ranges of [coordinates](Coordinate) in line that are not in [legal_plays](Plays).
    fn find_holes(
        legal_plays: &Plays,
        (min_x, min_y, max_x, max_y): (isize, isize, isize, isize),
        (mid_x, mid_y): Coordinate,
    ) -> Option<BTreeSet<(Coordinate, Coordinate)>> {
        // If the range of coordinates is large, hole will be large
        // and slow performance down. Limiting the range of coordinates
        // is necessary to limiting time and memory cost.
        // Also, COORDINATE_LIMIT and HOLES_LIMIT should prevent overflow.
        if min_x == max_x {
            let increasing = (mid_y + 1..=max_y)
                .filter(|&y| !legal_plays.contains_right(&(min_x, y)))
                .take((HOLES_LIMIT + 1) / 2)
                .peekable()
                .batching(batch_continuous_increasing_range)
                .map(|(first, last)| ((min_x, first), (min_x, last)));
            let decreasing = (min_y..=mid_y - 1)
                .rev()
                .filter(|&y| !legal_plays.contains_right(&(min_x, y)))
                .take((HOLES_LIMIT + HOLES_LIMIT % 2) / 2)
                .peekable()
                .batching(batch_continuous_decreasing_range)
                .map(|(first, last)| ((min_x, first), (min_x, last)));
            Some(increasing.chain(decreasing).collect())
        } else if min_y == max_y {
            let increasing = (mid_x + 1..=max_x)
                .filter(|&x| !legal_plays.contains_right(&(x, min_y)))
                .take((HOLES_LIMIT + 1) / 2)
                .peekable()
                .batching(batch_continuous_increasing_range)
                .map(|(first, last)| ((first, min_y), (last, min_y)));
            let decreasing = (min_x..=mid_x - 1)
                .rev()
                .filter(|&x| !legal_plays.contains_right(&(x, min_y)))
                .take((HOLES_LIMIT + HOLES_LIMIT % 2) / 2)
                .peekable()
                .batching(batch_continuous_decreasing_range)
                .map(|(first, last)| ((first, min_y), (last, min_y)));
            Some(increasing.chain(decreasing).collect())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        random_illegal_coordinates, Color, Shape, Tile, COORDINATE_LIMIT, FULL_MATCH_BONUS,
    };
    use bimap::BiBTreeMap;
    use map_macro::{btree_set, hash_set};
    use rand::Rng;
    use tap::Tap;

    impl FirstState {
        fn test_first_play_one_error(
            self,
            plays: impl IntoIterator<Item = (usize, (isize, isize))>,
            expected_error: FirstPlayError,
        ) {
            self.test_first_play_errors(plays, hash_set! { expected_error });
        }

        fn test_first_play_errors(
            self,
            plays: impl IntoIterator<Item = (usize, (isize, isize))>,
            expected_error: HashSet<FirstPlayError>,
        ) {
            let plays = plays.into_iter().collect();
            let (_, actual_error) = self
                .first_play(&plays)
                .expect_err("first_play should return Err");

            assert_eq!(expected_error, actual_error);
        }
    }

    #[test]
    fn empty_plays() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        first_state.random_hands(&mut rng);

        first_state.test_first_play_one_error([], FirstPlayError::EmptyPlays);
    }

    #[test]
    fn indexes_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand_len = first_state.random_hands(&mut rng);
        first_state.max_matches[0] = 1;

        let indexes_out_of_bounds: Plays = (1..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (index as isize, 0)))
            .collect();

        first_state.test_first_play_one_error(
            indexes_out_of_bounds.clone().tap_mut(|plays| {
                plays.insert(0, (0, 0));
            }),
            FirstPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds,
            },
        );
    }

    #[test]
    fn coordinates_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        first_state.hands[0].extend((0..=4).map(|_| rng.gen::<Tile>()));
        first_state.max_matches[0] = 1;

        let illegal_plays: Plays = (1..).zip(random_illegal_coordinates(&mut rng)).collect();

        first_state.test_first_play_one_error(
            illegal_plays.clone().tap_mut(|plays| {
                plays.insert(0, (0, 0));
            }),
            FirstPlayError::CoordinatesOutOfBounds {
                coordinates_out_of_bounds: illegal_plays.clone(),
            },
        );
    }

    #[test]
    fn origin_not_included() {
        let (first_state, plays) = set_up_first_play();
        let plays: Plays = plays
            .left_values()
            .map(|&index| (index, (1, index as isize)))
            .collect();

        first_state.test_first_play_one_error(plays, FirstPlayError::OriginNotIncluded);
    }

    #[test]
    fn not_max_matching() {
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rand::thread_rng());
        first_state.hands[0].extend([
            (Color::Orange, Shape::Starburst),
            (Color::Orange, Shape::X),
            (Color::Purple, Shape::X),
        ]);
        first_state.max_matches[0] = 2;

        first_state.test_first_play_one_error(
            [(0, (0, 0))],
            FirstPlayError::NotMaxMatching {
                max_matching_plays: btree_set! { btree_set! { 0, 1 }, btree_set! { 1, 2 } },
            },
        );
    }

    #[test]
    fn indexes_out_of_bounds_coordinates_out_of_bounds_not_max_matching() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand = &mut first_state.hands[0];
        hand.extend([
            (Color::Orange, Shape::Starburst),
            (Color::Orange, Shape::X),
            (Color::Purple, Shape::X),
            (Color::Red, Shape::Square),
            (Color::Red, Shape::Square),
        ]);
        let hand_len = hand.len();
        first_state.max_matches[0] = 2;

        let illegal_plays: Plays = (hand_len + 1..)
            .zip(random_illegal_coordinates(&mut rng))
            .collect();

        first_state.test_first_play_errors(
            illegal_plays.clone().tap_mut(|plays| {
                plays.insert(0, (0, 0));
            }),
            hash_set! {
                FirstPlayError::IndexesOutOfBounds {
                    indexes_out_of_bounds: illegal_plays.clone(),
                },
                FirstPlayError::CoordinatesOutOfBounds {
                    coordinates_out_of_bounds: illegal_plays.clone(),
                },
                FirstPlayError::NotMaxMatching {
                    max_matching_plays: btree_set! { btree_set! { 0, 1 }, btree_set! { 1, 2 } },
                },
            },
        );
    }

    #[test]
    fn indexes_out_of_bounds_no_legal_plays() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand_len = first_state.random_hands(&mut rng);

        let indexes_out_of_bounds: Plays = (0..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (index as isize, 0)))
            .collect();

        first_state.test_first_play_errors(
            indexes_out_of_bounds.clone(),
            hash_set! {
              FirstPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds,
              },
              FirstPlayError::NoLegalPlays,
            },
        );
    }

    #[test]
    fn no_legal_lines() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let color = rng.gen();
        first_state.hands[0].extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);
        first_state.max_matches_to_hand_len();

        first_state.test_first_play_one_error(
            [(0, (0, 0)), (1, (1, 1)), (2, (2, 2))],
            FirstPlayError::NoLegalLines,
        );
    }

    #[test]
    fn holes() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let color = rng.gen();
        first_state.hands[0].extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);
        first_state.max_matches_to_hand_len();

        first_state.test_first_play_one_error(
            [(0, (0, -3)), (1, (0, 0)), (2, (0, 4))],
            FirstPlayError::Holes {
                holes: btree_set! { ((0, -2), (0, -1)), ((0, 1), (0, 3)) },
            },
        );
    }

    #[test]
    fn holes_limit() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let color = rng.gen();
        first_state.hands[0].extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);
        first_state.max_matches_to_hand_len();

        let limit = ((HOLES_LIMIT + 1) / 2) as isize;
        first_state.test_first_play_one_error(
            [
                (0, (0, -COORDINATE_LIMIT + 1)),
                (1, (0, 0)),
                (2, (0, COORDINATE_LIMIT - 1)),
            ],
            FirstPlayError::Holes {
                holes: btree_set! { ((0, -limit), (0, -1)), ((0, 1), (0, limit)) },
            },
        );
    }

    #[test]
    fn duplicates() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let tile: Tile = rng.gen();
        first_state.hands[0].extend([tile, tile]);
        first_state.max_matches_to_hand_len();

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, 0)), (1, (0, 1))]);

        first_state.test_first_play_one_error(
            [(0, (0, 0)), (1, (0, 1))],
            FirstPlayError::Duplicates {
                duplicates: btree_set! { btree_set! {(0, 0), (0, 1)} },
            },
        );
    }

    #[test]
    fn multiple_matching() {
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rand::thread_rng());

        first_state.hands[0].extend([
            (Color::Yellow, Shape::Square),
            (Color::Yellow, Shape::Starburst),
            (Color::Yellow, Shape::Circle),
            (Color::Blue, Shape::Circle),
            (Color::Red, Shape::Circle),
            (Color::Green, Shape::X),
        ]);
        first_state.max_matches_to_hand_len();

        first_state.test_first_play_one_error(
            [
                (0, (0, 0)),
                (1, (0, 2)),
                (2, (0, 4)),
                (3, (0, 1)),
                (4, (0, 3)),
                (5, (0, 5)),
            ],
            FirstPlayError::MultipleMatching {
                multiple_matching: btree_set! {
                  btree_set! {(0, 0), (0, 2), (0, 4)},
                  btree_set! {(0, 1), (0, 3), (0, 4)},
                  btree_set! {(0, 5)}
                },
            },
        );
    }

    #[test]
    fn first_play_tiles() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let hand = &mut first_state.hands[0];
        let first = (Color::Green, Shape::X);
        let second = (Color::Green, Shape::Clover);
        let third = (Color::Purple, Shape::Diamond);
        let fourth = (Color::Green, Shape::Square);
        hand.extend([first, second, third, fourth]);

        let bag_tile = (Color::Orange, rng.gen());
        let bag_len = rng.gen_range(hand.len() + 1..10);
        first_state.bag.extend((0..bag_len).map(|_| bag_tile));
        first_state.hands[1].push(rng.gen());

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, -1)), (1, (0, 0)), (3, (0, 1))]);
        let plays_len = plays.len();
        first_state.max_matches[0] = plays_len;

        let mut next_state = first_state
            .first_play(&plays)
            .expect("first_play should return Ok");

        let hand = &next_state.mut_hands()[0];
        assert_eq!(third, hand[0]);
        assert_eq!(bag_tile, hand[1]);
        assert_eq!(bag_tile, hand[2]);
        assert_eq!(bag_tile, hand[3]);

        assert_eq!(bag_len - plays_len, next_state.mut_bag().len());
        assert_eq!(first, next_state.mut_board()[&(0, -1)]);
        assert_eq!(second, next_state.mut_board()[&(0, 0)]);
        assert_eq!(fourth, next_state.mut_board()[&(0, 1)]);
    }

    #[test]
    fn first_play_some_points() {
        let (first_state, plays) = set_up_first_play();

        let mut next_state = first_state
            .first_play(&plays)
            .expect("first_play should return Ok");

        assert_eq!(plays.len(), next_state.mut_points()[0]);
    }

    #[test]
    fn first_play_full_match() {
        let (first_state, plays) = set_up_first_play_full_match();

        let mut next_state = first_state
            .first_play(&plays)
            .expect("first_play should return Ok");

        assert_eq!(plays.len() + FULL_MATCH_BONUS, next_state.mut_points()[0]);
    }

    #[test]
    fn first_play_increment_current_player() {
        let (first_state, plays) = set_up_first_play();

        let mut next_state = first_state
            .first_play(&plays)
            .expect("first_play should return Ok");

        assert_eq!(1, *next_state.mut_current_player());
    }

    #[test]
    fn first_play_wrap_current_player() {
        let (mut first_state, plays) = set_up_first_play();
        let last = first_state.hands.len() - 1;
        first_state.current_player = last;
        let first_hand = first_state.hands[0].clone();
        let last_hand = &mut first_state.hands[last];
        last_hand.clear();
        last_hand.extend(first_hand);
        first_state.max_matches_to_hand_len();

        let mut next_state = first_state
            .first_play(&plays)
            .expect("first_play should return Ok");

        assert_eq!(0, *next_state.mut_current_player());
    }

    fn set_up_first_play() -> (FirstState, Plays) {
        let mut rng = rand::thread_rng();
        // avoid playing full match
        let hand_len = rng.gen_range(2..Shape::SHAPES_LEN);

        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        first_state.random_bag(&mut rng);
        let color = rng.gen();
        let hand = &mut first_state.hands[0];
        hand.extend(
            Shape::shapes()
                .into_iter()
                .take(hand_len)
                .map(|shape| (color, shape)),
        );
        let mut plays = BiBTreeMap::new();
        plays.extend((0..hand.len()).map(|index| (index, (index as isize, 0))));
        first_state.hands[1].push(rng.gen());
        first_state.max_matches_to_hand_len();

        (first_state, plays)
    }

    fn set_up_first_play_full_match() -> (FirstState, Plays) {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        first_state.random_bag(&mut rng);
        let shape = rng.gen();
        first_state.hands[0].extend(Color::colors().into_iter().map(|color| (color, shape)));
        let plays = (0..Color::COLORS_LEN)
            .map(|index| (index, (index as isize, 0)))
            .collect();
        first_state.hands[1].push(rng.gen());
        first_state.max_matches_to_hand_len();

        (first_state, plays)
    }
}
