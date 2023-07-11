use crate::state::{
    batch_holes, check_line, Coordinate, CoordinateRange, Coordinates, FirstState, Indexes,
    NextState, Plays, Points, Tile,
};
use itertools::Itertools;
use map_macro::btree_set;
use std::cmp;
use std::collections::{BTreeSet, HashSet};

/// Describes the reason why [`FirstState::first_play`](FirstState::first_play)
/// could not be executed.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FirstPlayError {
    /// Attempting to play no tiles.
    EmptyPlays,
    /// Attempting to play tiles not in `current_player`'s hand.
    IndexesOutOfBounds {
        /// Plays where the index is greater than or equal to hand_len.
        indexes_out_of_bounds: Plays,
    },
    /// Not attempting to play max match of legal plays.
    NotMaxMatching {
        /// Alternative acceptable plays of length max_match from `current_player`'s hand.
        max_matching_plays: BTreeSet<Indexes>,
    },
    /// Attempting to only play illegal plays.
    NoLegalPlays,
    /// Not attempting to play tiles in a point or a line.
    NoLegalLines,
    /// Attempting to play tiles in a line but not the same connected line.
    Holes {
        /// Ranges of coordinates in line that are not in plays.
        holes: BTreeSet<CoordinateRange>,
    },
    /// Attempting to play duplicate tiles in a line.
    Duplicates {
        /// Groups of coordinates where tiles are the same.
        duplicates: BTreeSet<Coordinates>,
    },
    /// Attempting to play a line where tiles are not either the same shape or the same color.
    MultipleMatching {
        /// Groups of coordinates where tiles match each other but not other groups.
        multiple_matching: BTreeSet<Coordinates>,
    },
}

impl FirstState {
    /// Checks if the plays are valid, then removes the tiles from `current_player`'s hand,
    /// inserts those into `board` at their coordinate minus the average coordinate
    /// (centers the plays in `board` around the origin), attempts to fill
    /// `current_player`'s hand up to its previous length, adds `points` earned by the play
    /// to the current player, and advances to the next player if the game has not ended.
    ///
    /// # Points Calculation
    ///
    /// The number of `points` from a line is the number of tiles in that line. If the line creates
    /// a full match on `board` where a line contains either every color in
    /// [`Color::colors`](crate::state::Color::colors) or every shape in
    /// [`Shape::shapes`](crate::state::Shape::shapes),
    /// gives an extra [`FULL_MATCH_BONUS`](crate::state::FULL_MATCH_BONUS) `points`.
    ///
    /// If `current_player`'s hand is empty (and therefore no tiles are available) or `board`
    /// becomes deadlocked (a filled rectangle of
    /// [`Color::COLORS_LEN`](crate::state::Color::COLORS_LEN)
    /// by [`Shape::SHAPES_LEN`](crate::state::Shape::SHAPES_LEN)) where no additional plays
    /// are allowed despite players still holding tiles, gives an extra
    /// [`LAST_PLAY_BONUS`](crate::state::LAST_PLAY_BONUS) `points`.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    ///
    /// # Errors
    ///
    /// * [`FirstPlayError::EmptyPlays`] Attempting to play no tiles.
    /// * [`FirstPlayError::IndexesOutOfBounds`] Attempting to play tiles not
    /// in `current_player`'s hand.
    /// * [`FirstPlayError::NotMaxMatching`] Not attempting to play max match of legal plays.
    /// * [`FirstPlayError::NoLegalPlays`] Attempting to only play illegal plays.
    /// * [`FirstPlayError::NoLegalLines`] Not attempting to play tiles in a point or a line.
    /// * [`FirstPlayError::Holes`] Attempting to play tiles in a line but not
    /// the same connected line.
    /// * [`FirstPlayError::Duplicates`] Attempting to play duplicate tiles in a line.
    /// * [`FirstPlayError::MultipleMatching`] Attempting to play a line where tiles are not either
    /// the same shape or the same color.
    ///
    /// # Returns
    ///
    /// The [`NextState`] of the game after the first turn.
    pub fn first_play(
        mut self,
        plays: &Plays,
    ) -> Result<NextState, (Self, HashSet<FirstPlayError>)> {
        let first_play_points = match self.check_plays(plays) {
            Ok(points) => points,
            Err(errors) => return Err((self, errors)),
        };

        let len = plays.len() as isize;
        let (avg_x, avg_y): Coordinate = plays
            .right_values()
            .copied()
            .reduce(|(sum_x, sum_y), (x, y)| (sum_x + x, sum_y + y))
            .map(|(x, y)| (x / len, y / len))
            .unwrap_or_default();

        let hand = &mut self.hands[self.current_player];
        let board = plays
            .iter()
            .rev()
            .map(|(&index, &(x, y))| ((x - avg_x, y - avg_y), hand.remove(index)))
            .collect();
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

    /// Takes a bimap of indexes of tiles to be played to coordinates on `board` and returns
    /// earned points if the bimap is a legal line, otherwise it returns the errors.
    ///
    /// # Points Calculation
    ///
    /// The number of points from a line is the number of tiles in that line. If the line creates
    /// a full match on `board` where a line contains either every color in
    /// [`Color::colors`](state::Color::colors) or every shape in
    /// [`Shape::shapes`](state::Shape::shapes),
    /// gives an extra [`FULL_MATCH_BONUS`](state::FULL_MATCH_BONUS) points.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    ///
    /// # Errors
    ///
    /// * [`FirstPlayError::HasEnded`] Attempting to play after the game has ended.
    /// * [`FirstPlayError::EmptyPlays`] Attempting to play no tiles.
    /// * [`FirstPlayError::IndexesOutOfBounds`] Attempting to play tiles not
    /// in `current_player`'s hand.
    /// * [`FirstPlayError::CoordinatesOutOfBounds`] Attempting to play tiles outside of `board`.
    /// * [`FirstPlayError::NotMaxMatching`] Not attempting to play max match of legal plays.
    /// * [`FirstPlayError::NoLegalPlays`] Attempting to only play illegal plays.
    /// * [`FirstPlayError::NoLegalLines`] Not attempting to play tiles in a point or a line.
    /// * [`FirstPlayError::Holes`] Attempting to play tiles in a line but not
    /// the same connected line.
    /// * [`FirstPlayError::Duplicates`] Attempting to play duplicate tiles in a line.
    /// * [`FirstPlayError::MultipleMatching`] Attempting to play a line where tiles are not either
    /// the same shape or the same color.
    ///
    /// # Returns
    ///
    /// The earned points from plays.
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

        let legal_plays: Plays = plays
            .left_range(..hand_len)
            .map(|(&index, &coordinate)| (index, coordinate))
            .collect();

        if legal_plays.len() != self.max_matches[self.current_player] {
            let max_matching_plays = self.find_max_matching_plays();
            errors.insert(FirstPlayError::NotMaxMatching { max_matching_plays });
        }

        // filter out empty legal_plays and find coordinate to be used after max match check
        let Some(&(x, y)) = legal_plays.right_values().next() else {
            errors.insert(FirstPlayError::NoLegalPlays);
            return Err(errors);
        };

        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;

        for &(x, y) in legal_plays.right_values() {
            min_x = cmp::min(min_x, x);
            max_x = cmp::max(max_x, x);
            min_y = cmp::min(min_y, y);
            max_y = cmp::max(max_y, y);
        }

        // If range of coordinates is large, hole will be large and slow performance down.
        // Limiting the range of coordinates is necessary to limiting time and memory cost.
        let holes: BTreeSet<CoordinateRange> = if min_x == max_x {
            (min_y..=max_y)
                .filter(|&y| !plays.contains_right(&(min_x, y)))
                .peekable()
                .batching(batch_holes)
                .map(|(first, last)| ((min_x, first), (min_x, last)))
                .collect()
        } else if min_y == max_y {
            (min_x..=max_x)
                .filter(|&x| !plays.contains_right(&(x, min_y)))
                .peekable()
                .batching(batch_holes)
                .map(|(first, last)| ((first, min_y), (last, min_y)))
                .collect()
        } else {
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

    /// Finds all maximum length combinations of unique tiles in `current_player`'s hand
    /// that match exclusively in color or shape, and return them as a set of indexes.
    ///
    /// # Returns
    ///
    /// A set of indexes that are the maximum matching plays.
    #[inline]
    fn find_max_matching_plays(&self) -> BTreeSet<Indexes> {
        let max_match = self.max_matches[self.current_player];
        let hand = &self.hands[self.current_player];
        if max_match == 0 {
            return btree_set! {};
        }
        if max_match == 1 {
            return (0..hand.len()).map(|index| btree_set! {index}).collect();
        }

        hand.iter()
            // indexes collected after filter
            .enumerate()
            // combinations must use unique tiles to prevent replacements
            .unique_by(|(_, &tile)| tile)
            .combinations(max_match)
            // is combination matching?
            .filter(|combination: &Vec<(usize, &Tile)>| {
                let mut iter = combination.iter();
                let Some((_,&(first_color, first_shape))) = iter.next() else {
                    // zero items is impossible after check
                    dbg!(&self, max_match);
                    unreachable!("max_match ({:?}) should not equal 0", max_match);
                };
                let Some((_,&(second_color, second_shape))) = iter.next() else {
                    // one item is impossible after check
                    dbg!(&self, max_match);
                    unreachable!("max_match ({:?}) should not equal 1", max_match);
                };

                if first_color == second_color {
                    iter.all(|(_, &(other_color, _))| first_color == other_color)
                } else if first_shape == second_shape {
                    iter.all(|(_, &(_, other_shape))| first_shape == other_shape)
                } else {
                    // tiles do not match since there are no duplicates
                    false
                }
            })
            // map vec to btreeset
            .map(|combination: Vec<(usize, &Tile)>| {
                combination.into_iter().map(|(index, _)| index).collect()
            })
            // collect into set
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Color, Shape, FULL_MATCH_BONUS};
    use bimap::BiBTreeMap;
    use map_macro::{btree_set, set};
    use rand::distributions::{Distribution, Uniform};
    use rand::Rng;

    #[test]
    fn empty_plays() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        first_state.random_hands(&mut rng);
        let plays = BiBTreeMap::new();

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! { FirstPlayError::EmptyPlays };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn indexes_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand_len = first_state.random_hands(&mut rng);
        first_state.max_matches[0] = 1;

        let indexes_out_of_bounds: Plays = (1..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (0, index as isize)))
            .collect();

        let mut plays = BiBTreeMap::new();
        plays.insert(0, (0, 0));
        plays.extend(indexes_out_of_bounds.clone());

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! { FirstPlayError::IndexesOutOfBounds {
            indexes_out_of_bounds
        }};
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn not_max_matching() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand = &mut first_state.hands[0];
        hand.extend([
            (Color::Orange, Shape::Starburst),
            (Color::Orange, Shape::X),
            (Color::Purple, Shape::X),
        ]);
        first_state.max_matches[0] = 2;

        let mut plays = BiBTreeMap::new();
        plays.insert(0, (0, 0));

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! { FirstPlayError::NotMaxMatching {
            max_matching_plays: btree_set! { btree_set! { 0, 1 }, btree_set! { 1, 2 } }
        }};
        assert_eq!(expected_error, actual_error);
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
        ]);
        hand.extend((0..rng.gen_range(3..=5)).map(|_| (Color::Red, Shape::Square)));
        let hand_len = hand.len();
        first_state.max_matches[0] = 2;
        let possible_illegal_coordinates = Uniform::from(40..isize::MAX);

        let mut illegal_plays = BiBTreeMap::new();
        for index in 1..(hand_len / 2) {
            let illegal_coordinate = possible_illegal_coordinates.sample(&mut rng);
            illegal_plays.insert(hand_len + index + 1, (0, illegal_coordinate));
            illegal_plays.insert(hand_len + index + 2, (illegal_coordinate, 0));
        }

        let mut plays = BiBTreeMap::new();
        plays.insert(0, (0, 0));
        plays.extend(illegal_plays.clone());

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! {
            FirstPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds: illegal_plays.clone(),
            },
            FirstPlayError::NotMaxMatching {
                max_matching_plays: btree_set! { btree_set! { 0, 1 }, btree_set! { 1, 2 } }
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn indexes_out_of_bounds_no_legal_plays() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand_len = first_state.random_hands(&mut rng);

        let indexes_out_of_bounds: Plays = (0..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (0, index as isize)))
            .collect();

        let mut plays = BiBTreeMap::new();
        plays.extend(indexes_out_of_bounds.clone());

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! {
            FirstPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds
            },
            FirstPlayError::NoLegalPlays
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn no_legal_lines() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let color = rng.gen();
        let hand = &mut first_state.hands[0];
        hand.extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);
        first_state.max_matches_to_hand_len();

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, 0)), (1, (1, 1)), (2, (2, 2))]);

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! { FirstPlayError::NoLegalLines };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn holes() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);

        let color = rng.gen();
        let hand = &mut first_state.hands[0];
        hand.extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);
        first_state.max_matches_to_hand_len();

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, -3)), (1, (0, 1)), (2, (0, 4))]);

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! {
            FirstPlayError::Holes {
                holes: btree_set! { ((0, -2), (0, 0)), ((0, 2), (0, 3)) }
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn duplicates() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand = &mut first_state.hands[0];
        let tile: Tile = rng.gen();
        hand.extend([tile, tile]);
        first_state.max_matches_to_hand_len();

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, 0)), (1, (0, 1))]);

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! {
            FirstPlayError::Duplicates {
                duplicates: btree_set! { btree_set! {(0, 0), (0, 1)} }
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn multiple_matching() {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        let hand = &mut first_state.hands[0];
        hand.extend([
            (Color::Yellow, Shape::Square),
            (Color::Yellow, Shape::Starburst),
            (Color::Yellow, Shape::Circle),
            (Color::Blue, Shape::Circle),
            (Color::Red, Shape::Circle),
            (Color::Green, Shape::X),
        ]);
        first_state.max_matches_to_hand_len();

        let mut plays = BiBTreeMap::new();
        plays.extend([
            (0, (0, 0)),
            (1, (0, 2)),
            (2, (0, 4)),
            (3, (0, 1)),
            (4, (0, 3)),
            (5, (0, 5)),
        ]);

        let (_, actual_error) = first_state.first_play(&plays).unwrap_err();

        let expected_error = set! {
            FirstPlayError::MultipleMatching {
                multiple_matching: btree_set! {
                    btree_set! {(0, 0), (0, 2), (0, 4)},
                    btree_set! {(0, 1), (0, 3), (0, 4)},
                    btree_set! {(0, 5)}
                }
            }
        };
        assert_eq!(expected_error, actual_error);
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
        plays.extend([(0, (0, 10)), (1, (0, 11)), (3, (0, 12))]);
        let plays_len = plays.len();
        first_state.max_matches[0] = plays_len;

        let mut next_state = first_state.first_play(&plays).unwrap();

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

        let mut next_state = first_state.first_play(&plays).unwrap();

        assert_eq!(plays.len(), next_state.mut_points()[0]);
    }

    #[test]
    fn first_play_full_match() {
        let (first_state, plays) = set_up_first_play_full_match();

        let mut next_state = first_state.first_play(&plays).unwrap();
        assert_eq!(plays.len() + FULL_MATCH_BONUS, next_state.mut_points()[0]);
    }

    #[test]
    fn first_play_increment_current_player() {
        let (first_state, plays) = set_up_first_play();

        let mut next_state = first_state.first_play(&plays).unwrap();

        assert_eq!(1, *next_state.mut_current_player());
    }

    #[test]
    fn first_play_wrap_current_player() {
        let (mut first_state, plays) = set_up_first_play();
        first_state.current_player = first_state.hands.len() - 1;
        let first_hand = first_state.hands[0].clone();
        first_state.hands[first_state.current_player].clear();
        first_state.hands[first_state.current_player].extend(first_hand);
        first_state.max_matches_to_hand_len();

        let mut next_state = first_state.first_play(&plays).unwrap();

        assert_eq!(0, *next_state.mut_current_player());
    }

    #[inline]
    fn set_up_first_play() -> (FirstState, Plays) {
        let mut rng = rand::thread_rng();
        let hand_len = rng.gen_range(2..=5);

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
        plays.extend((0..hand.len()).map(|index| (index, (0, index as isize))));
        first_state.hands[1].push(rng.gen());
        first_state.max_matches_to_hand_len();

        (first_state, plays)
    }

    #[inline]
    fn set_up_first_play_full_match() -> (FirstState, Plays) {
        let mut rng = rand::thread_rng();
        let mut first_state = FirstState::empty_first_state();
        first_state.random_players(&mut rng);
        first_state.random_bag(&mut rng);
        let hand = &mut first_state.hands[0];
        let mut plays = BiBTreeMap::new();
        let shape = rng.gen();
        hand.extend(Color::colors().into_iter().map(|color| (color, shape)));
        plays.extend((0..hand.len()).map(|index| (index, (0, index as isize))));
        first_state.hands[1].push(rng.gen());
        first_state.max_matches_to_hand_len();

        (first_state, plays)
    }
}
