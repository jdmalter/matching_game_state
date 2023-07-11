use crate::{
    adjacent_coordinates, batch_continuous_decreasing_range, batch_continuous_increasing_range,
    check_line, find_component_minimums_and_maximums, find_coordinate_by_minimum_distance,
    partition_by_coordinates, Board, Coordinate, LastState, NextState, Plays, Tile, HOLES_LIMIT,
    LAST_PLAY_BONUS,
};
use either::Either;
use itertools::Itertools;
use std::collections::{BTreeSet, HashSet};
use std::iter;
use std::ops::Index;

/// Describes the reason why the [next play](NextState::next_play) could not be executed.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NextPlayError {
    /// Attempting [to play](NextState::next_play) no [tiles](Tile).
    EmptyPlays,
    /// Attempting [to play](NextState::next_play) [tiles](Tile) not
    /// in the current player's hand.
    IndexesOutOfBounds {
        /// [Plays](Plays) where the index is greater than or equal to `hand_len`.
        indexes_out_of_bounds: Plays,
    },
    /// Attempting [to play](NextState::next_play) [tiles](Tile) too far away from
    /// the center of the board.
    CoordinatesOutOfBounds {
        /// [Plays](Plays) where the absolute value of a component in a [coordinate](Coordinate) is
        /// greater than or equal to the [coordinate limit](crate::COORDINATE_LIMIT).
        coordinates_out_of_bounds: Plays,
    },
    /// Attempting [to play](NextState::next_play) at already occupied [coordinates](Coordinate)
    /// on the board.
    CoordinatesOccupied {
        /// [Plays](Plays) where board already contains a [tile](Tile)
        /// at the [coordinate](Coordinate).
        coordinates_occupied: Plays,
    },
    /// Attempting [to play](NextState::next_play) [tiles](Tile) not connected
    /// to the board.
    NotConnected {
        /// [Plays](Plays) where there are no adjacent [tiles](Tile) on the board
        /// or no path through other connected [plays](Plays) to a [tile](Tile)
        /// on the board.
        not_connected: Plays,
    },
    /// Attempting to only [play](NextState::next_play) illegal [plays](Plays).
    NoLegalPlays,
    /// Not attempting [to play](NextState::next_play) a single [tile](Tile) or
    /// [tiles](Tile) in a line.
    NoLegalLines,
    /// Attempting [to play](NextState::next_play) [tiles](Tile) in a line but not
    /// the same connected line.
    Holes {
        /// Ranges of [coordinates](Coordinate) in line that are not in [plays](Plays).
        holes: BTreeSet<(Coordinate, Coordinate)>,
    },
    /// Attempting [to play](NextState::next_play) duplicate [tiles](Tile) in a line.
    Duplicates {
        /// Groups of [coordinates](Coordinate) where [tiles](Tile) are the same.
        duplicates: BTreeSet<BTreeSet<Coordinate>>,
    },
    /// Attempting [to play](NextState::next_play) a line where [tiles](Tile) are not either
    /// the same [shape](crate::Shape) or the same [color](crate::Color).
    MultipleMatching {
        /// Groups of [coordinates](Coordinate) where [tiles](Tile) match each other
        /// but not other groups.
        multiple_matching: BTreeSet<BTreeSet<Coordinate>>,
    },
}

impl NextState {
    /// Checks if the [plays](Plays) are valid, then removes the [tiles](Tile) from
    /// the current player's hand, inserts those [tiles](Tile)
    /// into the board at their respective [coordinates](Coordinate), attempts to fill
    /// the current player's hand up to its previous length,
    /// adds points earned by the [play](Plays) to the current player,
    /// and advances to the next player if the game has not ended.
    ///
    /// # Points Calculation
    ///
    /// The number of points earned by a [play](Plays) is the sum of points scored from each line
    /// that contains played [tiles](Tile). Each [tile](Tile) can be counted twice if
    /// the [tile](Tile) is a part of a vertical and horizontal line.
    ///
    /// The number of points from a line is the number of [tiles](Tile) in that line. If the line
    /// creates a full match on the board where a line contains either
    /// [every color](crate::Color::colors) or [every shape](crate::Shape::shapes), an extra
    /// [full match bonus](crate::FULL_MATCH_BONUS) is earned.
    ///
    /// If the current player's hand is empty (and therefore no [tiles](Tile) in
    /// the bag are available) or the board becomes deadlocked
    /// (a filled rectangle of [every color](crate::Color::colors) and every
    /// [shapes](crate::Shape::shapes)) where no additional [plays](Plays)
    /// are allowed despite players still holding some [tiles](Tile), an extra
    /// [last play bonus](LAST_PLAY_BONUS) is earned.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of [tiles](Tile) to be played to [coordinates](Coordinate)
    /// on the board.
    ///
    /// # Errors
    ///
    /// * [NextPlayError::EmptyPlays] Attempting [to play](NextState::next_play) no [tiles](Tile).
    /// * [NextPlayError::IndexesOutOfBounds] Attempting [to play](NextState::next_play)
    /// [tiles](Tile) not in the current player's hand.
    /// * [NextPlayError::CoordinatesOutOfBounds] Attempting [to play](NextState::next_play)
    /// [tiles](Tile) too far away from the center of the board.
    /// * [NextPlayError::CoordinatesOccupied] Attempting to
    /// [play](NextState::next_play) at already occupied [coordinates](Coordinate)
    /// on the board.
    /// * [NextPlayError::NotConnected] Attempting [to play](NextState::next_play) [tiles](Tile)
    /// not connected to the board.
    /// * [NextPlayError::NoLegalPlays] Attempting to only [play](NextState::next_play)
    /// illegal [plays](Plays).
    /// * [NextPlayError::NoLegalLines] Not attempting [to play](NextState::next_play)
    /// [tiles](Tile) in a point or a line.
    /// * [NextPlayError::Holes] Attempting [to play](NextState::next_play) [tiles](Tile) in a line
    /// but not the same connected line.
    /// * [NextPlayError::Duplicates] Attempting [to play](NextState::next_play)
    /// duplicate [tiles](Tile) in a line.
    /// * [NextPlayError::MultipleMatching] Attempting [to play](NextState::next_play) a line
    /// where [tiles](Tile) are not either the same [shape](crate::Shape)
    /// or the same [color](crate::Color).
    ///
    /// # Returns
    ///
    /// Either the [next state](NextState) or the [last state](LastState) of the game
    /// after the [play](Plays).
    pub fn next_play(
        mut self,
        plays: &Plays,
    ) -> Result<Either<NextState, LastState>, (Self, HashSet<NextPlayError>)> {
        let next_play_points = match self.check_plays(plays) {
            Ok(points) => points,
            Err(errors) => return Err((self, errors)),
        };

        let hand = &mut self.hands[self.current_player];
        self.board.extend(
            plays
                .iter()
                .rev()
                .map(|(&index, &coordinate)| (coordinate, hand.remove(index))),
        );
        // when the bag is empty, no more tiles will be drained
        hand.extend(self.bag.drain(self.bag.len().saturating_sub(plays.len())..));

        if self.has_ended() {
            self.points[self.current_player] += next_play_points + LAST_PLAY_BONUS;
            Ok(Either::Right(LastState::new(
                self.board,
                self.points,
                self.hands,
            )))
        } else {
            self.points[self.current_player] += next_play_points;
            self.current_player = (self.current_player + 1) % self.hands.len();
            Ok(Either::Left(self))
        }
    }

    /// Takes a bimap of indexes of [tiles](Tile) to be played to [coordinates](Coordinate)
    /// on the board and returns earned points if the bimap only creates legal lines.
    /// Otherwise, it returns the errors.
    ///
    /// # Points Calculation
    ///
    /// The number of points earned by a [play](Plays) is the sum of points scored from
    /// each line that contains played [tiles](Tile). Each [tile](Tile) can be counted twice
    /// if the [tile](Tile) is a part of a vertical and horizontal line.
    ///
    /// The number of points from a line is the number of [tiles](Tile) in that line.
    /// If the line creates a full match on the board where a line contains either
    /// [every color](crate::Color::colors) or [every shape](crate::Shape::shapes),
    /// an extra [FULL_MATCH_BONUS](crate::FULL_MATCH_BONUS) points are earned.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of [tiles](Tile) to be played to [coordinates](Coordinate)
    /// on the board.
    ///
    /// # Errors
    ///
    /// * [NextPlayError::EmptyPlays] Attempting [to play](NextState::next_play) no [tiles](Tile).
    /// * [NextPlayError::IndexesOutOfBounds] Attempting [to play](NextState::next_play)
    /// [tiles](Tile) not in the current player's hand.
    /// * [NextPlayError::CoordinatesOutOfBounds] Attempting [to play](NextState::next_play)
    /// [tiles](Tile) too far away from the center of the board.
    /// * [NextPlayError::CoordinatesOccupied] Attempting to
    /// [play](NextState::next_play) at already occupied [coordinates](Coordinate)
    /// on the board.
    /// * [NextPlayError::NotConnected] Attempting [to play](NextState::next_play) [tiles](Tile)
    /// not connected to the board.
    /// * [NextPlayError::NoLegalPlays] Attempting to only [play](NextState::next_play)
    /// illegal [plays](Plays).
    /// * [NextPlayError::NoLegalLines] Not attempting [to play](NextState::next_play)
    /// [tiles](Tile) in a point or a line.
    /// * [NextPlayError::Holes] Attempting [to play](NextState::next_play) [tiles](Tile) in a line
    /// but not the same connected line.
    /// * [NextPlayError::Duplicates] Attempting [to play](NextState::next_play)
    /// duplicate [tiles](Tile) in a line.
    /// * [NextPlayError::MultipleMatching] Attempting [to play](NextState::next_play) a line
    /// where [tiles](Tile) are not either the same [shape](crate::Shape)
    /// or the same [color](crate::Color).
    ///
    /// # Returns
    ///   
    /// The earned points from [plays](Plays).
    fn check_plays(&self, plays: &Plays) -> Result<usize, HashSet<NextPlayError>> {
        let mut errors = HashSet::with_capacity(10);

        if plays.is_empty() {
            errors.insert(NextPlayError::EmptyPlays);
            return Err(errors);
        }

        let hand = &self.hands[self.current_player];
        let hand_len = hand.len();
        let indexes_out_of_bounds: Plays = plays
            .left_range(hand_len..)
            .map(|(&index, &coordinate)| (index, coordinate))
            .collect();
        if !indexes_out_of_bounds.is_empty() {
            errors.insert(NextPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds,
            });
        }

        let (coordinates_in_bounds, coordinates_out_of_bounds) = partition_by_coordinates(plays);
        if !coordinates_out_of_bounds.is_empty() {
            errors.insert(NextPlayError::CoordinatesOutOfBounds {
                coordinates_out_of_bounds,
            });
        }

        let (coordinates_unoccupied, coordinates_occupied): (Plays, Plays) = coordinates_in_bounds
            .into_iter()
            .partition(|(_, coordinate)| !self.board.contains_key(coordinate));
        if !coordinates_occupied.is_empty() {
            errors.insert(NextPlayError::CoordinatesOccupied {
                coordinates_occupied,
            });
        }

        let (connected, not_connected) = self.partition_connected(coordinates_unoccupied);
        if !not_connected.is_empty() {
            errors.insert(NextPlayError::NotConnected { not_connected });
        }

        let legal_plays: Plays = connected
            .left_range(..hand_len)
            .map(|(&index, &coordinate)| (index, coordinate))
            .collect();

        let Some((min_x, min_y, max_x, max_y)) =
            find_component_minimums_and_maximums(legal_plays.right_values().copied()) else {
            errors.insert(NextPlayError::NoLegalPlays);
            return Err(errors);
        };

        let Some((mid_x, mid_y))
            = find_coordinate_by_minimum_distance(legal_plays.right_values().copied()) else {
            errors.insert(NextPlayError::NoLegalPlays);
            return Err(errors);
        };

        // If the range of coordinates is large, hole will be large
        // and slow performance down. Limiting the range of coordinates
        // is necessary to limiting time and memory cost.
        // Also, COORDINATE_LIMIT and HOLES_LIMIT should prevent overflow.
        let (holes, lines): (BTreeSet<(Coordinate, Coordinate)>, Vec<Board>) = if min_x == max_x {
            let increasing = (mid_y + 1..=max_y)
                .filter(|&y| {
                    let coordinate = (min_x, y);
                    !self.board.contains_key(&coordinate) && !plays.contains_right(&coordinate)
                })
                .take((HOLES_LIMIT + 1) / 2)
                .peekable()
                .batching(batch_continuous_increasing_range)
                .map(|(first, last)| ((min_x, first), (min_x, last)));
            let decreasing = (min_y..=mid_y - 1)
                .rev()
                .filter(|&y| {
                    let coordinate = (min_x, y);
                    !self.board.contains_key(&coordinate) && !plays.contains_right(&coordinate)
                })
                .take((HOLES_LIMIT + HOLES_LIMIT % 2) / 2)
                .peekable()
                .batching(batch_continuous_decreasing_range)
                .map(|(first, last)| ((min_x, first), (min_x, last)));
            let holes = increasing.chain(decreasing).collect();

            // horizontal lines perpendicular to the vertical line legal_plays
            let lines = self.find_lines(
                &legal_plays,
                (mid_y..).map(|next_y| (mid_x, next_y)),
                (1..)
                    .map(|offset| mid_y - offset)
                    .map(|next_y| (mid_x, next_y)),
                |(x, y)| (x + 1..).map(move |next_x| (next_x, y)),
                |(x, y)| {
                    (1..)
                        .map(move |offset| x - offset)
                        .map(move |next_x| (next_x, y))
                },
            );

            (holes, lines)
        } else if min_y == max_y {
            let increasing = (mid_x + 1..=max_x)
                .filter(|&x| {
                    let coordinate = (x, min_y);
                    !self.board.contains_key(&coordinate) && !plays.contains_right(&coordinate)
                })
                .take((HOLES_LIMIT + 1) / 2)
                .peekable()
                .batching(batch_continuous_increasing_range)
                .map(|(first, last)| ((first, min_y), (last, min_y)));
            let decreasing = (min_x..=mid_x - 1)
                .rev()
                .filter(|&x| {
                    let coordinate = (x, min_y);
                    !self.board.contains_key(&coordinate) && !plays.contains_right(&coordinate)
                })
                .take((HOLES_LIMIT + HOLES_LIMIT % 2) / 2)
                .peekable()
                .batching(batch_continuous_decreasing_range)
                .map(|(first, last)| ((first, min_y), (last, min_y)));
            let holes = increasing.chain(decreasing).collect();

            // vertical lines perpendicular to the horizontal line legal_plays
            let lines = self.find_lines(
                &legal_plays,
                (mid_x..).map(|next_x| (next_x, mid_y)),
                (1..)
                    .map(|offset| mid_x - offset)
                    .map(|next_x| (next_x, mid_y)),
                |(x, y)| (y + 1..).map(move |next_y| (x, next_y)),
                |(x, y)| {
                    (1..)
                        .map(move |offset| y - offset)
                        .map(move |next_y| (x, next_y))
                },
            );

            (holes, lines)
        } else {
            errors.insert(NextPlayError::NoLegalLines);
            return Err(errors);
        };

        if !holes.is_empty() {
            errors.insert(NextPlayError::Holes { holes });
        }

        let mut total_points = 0;

        for line in lines {
            match check_line(&line) {
                Err((duplicates, multiple_matching)) => {
                    if !duplicates.is_empty() {
                        errors.insert(NextPlayError::Duplicates { duplicates });
                    }
                    if !multiple_matching.is_empty() {
                        errors.insert(NextPlayError::MultipleMatching { multiple_matching });
                    }
                }
                Ok(points) => {
                    total_points += points;
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(total_points)
    }

    /// Partitions `possibly_connected` by whether the [coordinate](Coordinate) is connected
    /// to the board. A [coordinate](Coordinate) can be connected either directly
    /// by an adjacent [tile](Tile) in the board or indirectly by a path of adjacent,
    /// indirectly connected [coordinates](Coordinate) to a directly
    /// connected [coordinate](Coordinate).
    ///
    /// # Arguments
    ///
    /// * `possibly_connected`: [Plays] which may or may not be connected to the board.
    ///
    /// # Returns
    ///   
    /// A tuple of connected and not connected [plays](Plays).
    fn partition_connected(&self, possibly_connected: Plays) -> (Plays, Plays) {
        let capacity = possibly_connected.len();
        let mut connected = HashSet::with_capacity(capacity);

        for &stack_coordinate in possibly_connected.right_values() {
            if connected.contains(&stack_coordinate) {
                // Avoid duplicate searching
                continue;
            }

            // DFS
            let mut stack = Vec::with_capacity(capacity);
            stack.push(stack_coordinate);
            let mut visited = HashSet::with_capacity(capacity);

            while let Some(coordinate) = stack.pop() {
                if !visited.insert(coordinate) {
                    // Avoid duplicate searching
                    continue;
                }

                // overflow should not occur since coordinates in plays should
                // not be isize::MIN or isize::MAX
                for adjacent_coordinate in adjacent_coordinates(coordinate) {
                    if self.board.contains_key(&adjacent_coordinate)
                        || connected.contains(&adjacent_coordinate)
                    {
                        // connected directly or indirectly
                        connected.extend(visited.clone());
                        break;
                    } else if possibly_connected.contains_right(&adjacent_coordinate) {
                        // not yet connected
                        stack.push(adjacent_coordinate);
                    }
                }
            }
        }

        possibly_connected
            .into_iter()
            .partition(|(_, coordinate)| connected.contains(coordinate))
    }

    /// Each `increasing` and `decreasing` pair are stopped when `plays` or the board
    /// does not contain the next [coordinate](Coordinate) and chained together to create a line.
    /// Each [coordinate](Coordinate) in `plays` is converted `into_increasing`
    /// and `into_decreasing` to create lines perpendicular to `plays`.
    ///
    /// # Arguments
    ///
    /// * `board`: A map of [coordinates](Coordinate) to [tiles](Tile) that have been played.
    /// * `hand`: A vector of [tiles](Tile).
    /// * `plays`: A bimap of indexes of [tiles](Tile) to be played to [coordinates](Coordinate)
    /// on the board.
    /// * `increasing`: An iteration of [coordinates](Coordinate) in the line containing `plays`
    /// in the opposite direction of `decreasing`.
    /// * `decreasing`: An iteration of [coordinates](Coordinate) in the line containing `plays`
    /// in the opposite direction of `increasing`.
    /// * `into_increasing`: Produces an `increasing` iteration from some [coordinate](Coordinate)
    /// in `plays` in the opposite direction of `into_decreasing`
    /// * `into_decreasing`: Produces an `decreasing` iteration from some [coordinate](Coordinate)
    /// in `plays` in the opposite direction of `into_increasing`
    ///
    /// # Returns
    ///
    /// A vector of the line containing `plays` plus the board lines extending from
    /// each [tile](Tile)in `plays`.
    fn find_lines<I, D>(
        &self,
        legal_plays: &Plays,
        increasing: impl Iterator<Item = Coordinate>,
        decreasing: impl Iterator<Item = Coordinate>,
        into_increasing: impl Fn(Coordinate) -> I,
        into_decreasing: impl Fn(Coordinate) -> D,
    ) -> Vec<Board>
    where
        I: Iterator<Item = Coordinate>,
        D: Iterator<Item = Coordinate>,
    {
        let hand = &self.hands[self.current_player];
        let get_plays_or_board = |coordinate: Coordinate| -> Option<(Coordinate, Tile)> {
            legal_plays
                .get_by_right(&coordinate)
                .map(|&index| hand.index(index))
                .or_else(|| self.board.get(&coordinate))
                .map(|&tile| (coordinate, tile))
        };
        let increasing = increasing.map(get_plays_or_board).while_some();
        let decreasing = decreasing.map(get_plays_or_board).while_some();
        let plays_line: Board = increasing.chain(decreasing).collect();

        let board_lines = legal_plays
            .iter()
            .map(|(&index, &plays_coordinate)| {
                let increasing = into_increasing(plays_coordinate)
                    .map(|coordinate| self.board.get(&coordinate).map(|&tile| (coordinate, tile)))
                    .while_some();
                let decreasing = into_decreasing(plays_coordinate)
                    .map(|coordinate| self.board.get(&coordinate).map(|&tile| (coordinate, tile)))
                    .while_some();
                iter::once((plays_coordinate, hand[index]))
                    .chain(increasing)
                    .chain(decreasing)
                    .collect()
            })
            .filter(|line: &Board| line.len() > 1);

        iter::once(plays_line).chain(board_lines).collect_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        random_different_color_same_shape, random_different_shape_same_color,
        random_illegal_coordinates, Color, Hand, Shape, Tile, COORDINATE_LIMIT, FULL_MATCH_BONUS,
        HAND_CAPACITY,
    };
    use bimap::BiBTreeMap;
    use map_macro::{btree_set, hash_set};
    use rand::Rng;
    use tap::Tap;

    impl NextState {
        fn test_next_play_one_error(
            self,
            plays: impl IntoIterator<Item = (usize, (isize, isize))>,
            expected_error: NextPlayError,
        ) {
            self.test_next_play_errors(plays, hash_set! { expected_error });
        }

        fn test_next_play_errors(
            self,
            plays: impl IntoIterator<Item = (usize, (isize, isize))>,
            expected_error: HashSet<NextPlayError>,
        ) {
            let plays = plays.into_iter().collect();
            let (_, actual_error) = self
                .next_play(&plays)
                .expect_err("next_play should return Err");

            assert_eq!(expected_error, actual_error);
        }
    }

    #[test]
    fn empty_plays() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_hands(&mut rng);

        next_state.test_next_play_one_error([], NextPlayError::EmptyPlays);
    }

    #[test]
    fn indexes_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);
        let tile = random_different_shape_same_color(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((0, -1), tile);

        let indexes_out_of_bounds: Plays = (1..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (index as isize, 0)))
            .collect();

        next_state.test_next_play_one_error(
            indexes_out_of_bounds.clone().tap_mut(|plays| {
                plays.insert(0, (0, 0));
            }),
            NextPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds,
            },
        );
    }

    #[test]
    fn coordinates_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.hands[0].extend((0..=4).map(|_| rng.gen::<Tile>()));
        let tile = random_different_color_same_shape(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((0, -1), tile);

        let illegal_plays: Plays = (1..).zip(random_illegal_coordinates(&mut rng)).collect();

        next_state.test_next_play_one_error(
            illegal_plays.clone().tap_mut(|plays| {
                plays.insert(0, (0, 0));
            }),
            NextPlayError::CoordinatesOutOfBounds {
                coordinates_out_of_bounds: illegal_plays.clone(),
            },
        );
    }

    #[test]
    fn coordinates_occupied() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let coordinates_occupied: Plays = next_state
            .board
            .keys()
            .take(hand_len - 1)
            .enumerate()
            .map(|(index, &coordinate)| (index + 1, coordinate))
            .collect();

        let &(x, y) = next_state
            .board
            .keys()
            .next()
            .expect("random_board should not produce an empty board");
        let tile = random_different_shape_same_color(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((x, y), tile);

        next_state.test_next_play_one_error(
            coordinates_occupied.clone().tap_mut(|plays| {
                plays.insert(0, (x, y + 1));
            }),
            NextPlayError::CoordinatesOccupied {
                coordinates_occupied,
            },
        );
    }

    #[test]
    fn not_connected() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let not_connected: Plays = (0..hand_len - 1)
            .map(|index| (index + 1, (index as isize, 3)))
            .collect();

        let tile = random_different_color_same_shape(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((0, -1), tile);

        next_state.test_next_play_one_error(
            not_connected.clone().tap_mut(|plays| {
                plays.insert(0, (0, 0));
            }),
            NextPlayError::NotConnected { not_connected },
        );
    }

    #[test]
    fn indexes_out_of_bounds_coordinates_occupied() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_board(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let illegal_plays: Plays = next_state
            .board
            .keys()
            .enumerate()
            .map(|(index, &coordinate)| (hand_len + index + 1, coordinate))
            .collect();

        let &(x, y) = next_state
            .board
            .keys()
            .next()
            .expect("random_board should not produce an empty board");
        let tile = random_different_shape_same_color(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((x, y), tile);

        next_state.test_next_play_errors(
            illegal_plays.clone().tap_mut(|plays| {
                plays.insert(0, (x, y + 1));
            }),
            hash_set! {
                NextPlayError::IndexesOutOfBounds {
                    indexes_out_of_bounds: illegal_plays.clone(),
                },
                NextPlayError::CoordinatesOccupied {
                    coordinates_occupied: illegal_plays,
                },
            },
        );
    }

    #[test]
    fn indexes_out_of_bounds_not_connected_no_legal_plays() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let illegal_plays: Plays = (0..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (index as isize, 0)))
            .collect();

        next_state.test_next_play_errors(
            illegal_plays.clone(),
            hash_set! {
                NextPlayError::IndexesOutOfBounds {
                    indexes_out_of_bounds: illegal_plays.clone()
                },
                NextPlayError::NotConnected {
                    not_connected: illegal_plays
                },
                NextPlayError::NoLegalPlays
            },
        );
    }

    #[test]
    fn no_legal_lines() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let color = rng.gen();
        next_state.board.extend([
            ((0, 1), (color, Shape::Clover)),
            ((1, 2), (color, Shape::Starburst)),
        ]);

        next_state.hands[0].extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);

        next_state.test_next_play_one_error(
            [(0, (0, 0)), (1, (1, 1)), (2, (2, 2))],
            NextPlayError::NoLegalLines,
        );
    }

    #[test]
    fn holes() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let shape = rng.gen();
        next_state.board.extend(
            Color::colors()
                .into_iter()
                .map(|color| (color, shape))
                .enumerate()
                .map(|(index, tile)| ((-1, index as isize), tile)),
        );

        next_state.hands[0].extend([
            (Color::Purple, shape),
            (Color::Green, shape),
            (Color::Red, shape),
        ]);

        next_state.test_next_play_one_error(
            [(0, (0, 0)), (1, (0, 2)), (2, (0, 5))],
            NextPlayError::Holes {
                holes: btree_set! { ((0, 1), (0, 1)), ((0, 3), (0, 4)) },
            },
        );
    }

    #[test]
    fn holes_limit() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);

        let color = rng.gen();
        next_state.hands[0].extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);

        next_state.board.extend([
            (
                (1, -COORDINATE_LIMIT + 1),
                random_different_color_same_shape(&mut rng, next_state.hands[0][0]),
            ),
            (
                (1, 0),
                random_different_shape_same_color(&mut rng, next_state.hands[0][1]),
            ),
            (
                (1, COORDINATE_LIMIT - 1),
                random_different_color_same_shape(&mut rng, next_state.hands[0][2]),
            ),
        ]);

        let limit = ((HOLES_LIMIT + 1) / 2) as isize;
        next_state.test_next_play_one_error(
            [
                (0, (0, -COORDINATE_LIMIT + 1)),
                (1, (0, 0)),
                (2, (0, COORDINATE_LIMIT - 1)),
            ],
            NextPlayError::Holes {
                holes: btree_set! { ((0, -limit), (0, -1)), ((0, 1), (0, limit)) },
            },
        );
    }

    #[test]
    fn duplicates_vertical() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let tile = rng.gen();
        let matching_tile = random_different_shape_same_color(&mut rng, tile);
        next_state.board.insert((0, 0), tile);
        next_state.board.insert((0, 1), matching_tile);

        next_state.hands[0].extend([tile, tile]);

        next_state.test_next_play_errors(
            [(0, (1, 0)), (1, (1, 1))],
            hash_set! {
                NextPlayError::Duplicates {
                    duplicates: btree_set! { btree_set! {(1, 0), (1, 1)} },
                },
                NextPlayError::Duplicates {
                    duplicates: btree_set! { btree_set! {(0, 0), (1, 0)} },
                },
            },
        );
    }

    #[test]
    fn duplicates_horizontal() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let tile = rng.gen();
        let matching_tile = random_different_color_same_shape(&mut rng, tile);
        next_state.board.insert((0, 0), tile);
        next_state.board.insert((1, 0), matching_tile);

        next_state.hands[0].extend([tile, tile]);

        next_state.test_next_play_errors(
            [(0, (0, 1)), (1, (1, 1))],
            hash_set! {
                NextPlayError::Duplicates {
                    duplicates: btree_set! { btree_set! {(0, 1), (1, 1)} },
                },
                NextPlayError::Duplicates {
                    duplicates: btree_set! { btree_set! {(0, 0), (0, 1)} },
                },
            },
        );
    }

    #[test]
    fn multiple_matching_vertical() {
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rand::thread_rng());
        next_state.board.extend([
            ((-1, -1), (Color::Green, Shape::Diamond)),
            ((1, -1), (Color::Red, Shape::Square)),
            ((-1, 0), (Color::Green, Shape::X)),
            ((1, 0), (Color::Yellow, Shape::Circle)),
            ((-1, 1), (Color::Blue, Shape::Starburst)),
            ((1, 1), (Color::Purple, Shape::Circle)),
        ]);

        next_state.hands[0].extend([
            (Color::Green, Shape::Square),
            (Color::Green, Shape::Circle),
            (Color::Blue, Shape::Circle),
        ]);

        next_state.test_next_play_errors(
            [(0, (0, -1)), (1, (0, 0)), (2, (0, 1))],
            hash_set! {
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (0, -1), (0, 0) },
                        btree_set! { (0, 0), (0, 1) },
                    }
                },
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (-1, -1), (0, -1) },
                        btree_set! { (0, -1), (1, -1) },
                    }
                },
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (-1, 0), (0, 0) },
                        btree_set! { (0, 0), (1, 0) },
                    }
                },
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (-1, 1), (0, 1) },
                        btree_set! { (0, 1), (1, 1) },
                    }
                },
            },
        );
    }

    #[test]
    fn multiple_matching_horizontal() {
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rand::thread_rng());
        next_state.board.extend([
            ((-1, -1), (Color::Green, Shape::Diamond)),
            ((-1, 1), (Color::Red, Shape::Square)),
            ((0, -1), (Color::Green, Shape::X)),
            ((0, 1), (Color::Yellow, Shape::Circle)),
            ((1, -1), (Color::Blue, Shape::Starburst)),
            ((1, 1), (Color::Purple, Shape::Circle)),
        ]);

        next_state.hands[0].extend([
            (Color::Green, Shape::Square),
            (Color::Green, Shape::Circle),
            (Color::Blue, Shape::Circle),
        ]);

        next_state.test_next_play_errors(
            [(0, (-1, 0)), (1, (0, 0)), (2, (1, 0))],
            hash_set! {
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (-1, 0), (0, 0) },
                        btree_set! { (0, 0), (1, 0) },
                    }
                },
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (-1, -1), (-1, 0) },
                        btree_set! { (-1, 0), (-1, 1) },
                    }
                },
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (0, -1), (0, 0) },
                        btree_set! { (0, 0), (0, 1) },
                    }
                },
                NextPlayError::MultipleMatching {
                    multiple_matching: btree_set! {
                        btree_set! { (1, -1), (1, 0) },
                        btree_set! { (1, 0), (1, 1) },
                    }
                },
            },
        );
    }

    #[test]
    fn next_play_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);

        next_state.board.insert((0, 0), (Color::Yellow, Shape::X));

        let hand = &mut next_state.hands[0];
        let first = (Color::Green, Shape::X);
        let second = (Color::Green, Shape::Clover);
        let third = (Color::Purple, Shape::Diamond);
        let fourth = (Color::Green, Shape::Square);
        hand.extend([first, second, third, fourth]);

        let bag_tile = (Color::Orange, rng.gen());
        let bag_len = rng.gen_range(hand.len()..10);
        next_state.bag.extend((0..bag_len).map(|_| bag_tile));

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (1, 0)), (1, (1, 1)), (3, (1, 2))]);
        let plays_len = plays.len();

        let next_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_left("Ok should contain next_state");

        let hand = &next_state.hands[0];
        assert_eq!(third, hand[0]);
        assert_eq!(bag_tile, hand[1]);
        assert_eq!(bag_tile, hand[2]);
        assert_eq!(bag_tile, hand[3]);

        assert_eq!(bag_len - plays_len, next_state.bag.len());
        assert_eq!(first, next_state.board[&(1, 0)]);
        assert_eq!(second, next_state.board[&(1, 1)]);
        assert_eq!(fourth, next_state.board[&(1, 2)]);
    }

    #[test]
    fn next_play_some_points() {
        let (next_state, plays) = set_up_next_play();

        let next_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_left("Ok should contain next_state");

        assert_eq!(2 + plays.len(), next_state.points[0]);
    }

    #[test]
    fn next_play_some_points_last_play() {
        let (mut next_state, plays) = set_up_next_play();
        next_state.bag.clear();

        let mut last_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_right("Ok should contain last_state");

        assert_eq!(
            2 + plays.len() + LAST_PLAY_BONUS,
            last_state.mut_points()[0]
        );
    }

    #[test]
    fn next_play_full_match() {
        let (next_state, plays) = set_up_next_play_full_match();

        let next_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_left("Ok should contain next_state");

        assert_eq!(2 + plays.len() + FULL_MATCH_BONUS, next_state.points[0]);
    }

    #[test]
    fn next_play_double_full_match() {
        let (next_state, plays) = set_up_next_play_double_match();

        let next_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_left("Ok should contain next_state");

        assert_eq!(
            Color::COLORS_LEN + Shape::SHAPES_LEN + 2 * FULL_MATCH_BONUS,
            next_state.points[0]
        );
    }

    #[test]
    fn next_play_double_full_match_last_play() {
        let (mut next_state, plays) = set_up_next_play_double_match();
        next_state.bag.clear();

        let mut last_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_right("Ok should contain last_state");

        assert_eq!(
            Color::COLORS_LEN + Shape::SHAPES_LEN + 2 * FULL_MATCH_BONUS + LAST_PLAY_BONUS,
            last_state.mut_points()[0]
        );
    }

    #[test]
    fn next_play_full_match_last_play() {
        let (mut next_state, plays) = set_up_next_play_full_match();
        next_state.bag.clear();

        let mut last_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_right("Ok should contain last_state");

        assert_eq!(
            2 + plays.len() + FULL_MATCH_BONUS + LAST_PLAY_BONUS,
            last_state.mut_points()[0]
        );
    }

    #[test]
    fn next_play_increment_current_player() {
        let (next_state, plays) = set_up_next_play();

        let next_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_left("Ok should contain next_state");

        assert_eq!(1, next_state.current_player);
    }

    #[test]
    fn next_play_wrap_current_player() {
        let (mut next_state, plays) = set_up_next_play();
        let last = next_state.hands.len() - 1;
        next_state.current_player = last;
        let next_hand = next_state.hands[0].clone();
        next_state.hands[last].extend(next_hand);

        let next_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_left("Ok should contain next_state");

        assert_eq!(0, next_state.current_player);
    }

    #[test]
    fn next_play_deadlock() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();

        let bag_tile: Tile = rng.gen();
        next_state
            .bag
            .extend((0..Shape::SHAPES_LEN).map(|_| bag_tile));

        let color: Color = rng.gen();

        next_state.board.extend(
            Color::colors()
                .into_iter()
                .filter(|&other_color| color != other_color)
                .enumerate()
                .flat_map(|(row, color)| {
                    Shape::shapes()
                        .into_iter()
                        .map(move |shape| (color, shape))
                        .enumerate()
                        .map(move |(col, tile)| ((col as isize, (row + 1) as isize), tile))
                }),
        );

        next_state.points.push(0);
        next_state.hands.push(Hand::with_capacity(HAND_CAPACITY));
        let hand = &mut next_state.hands[0];
        let mut plays = BiBTreeMap::new();
        hand.extend(Shape::shapes().into_iter().map(|shape| (color, shape)));
        plays.extend((0..hand.len()).map(|index| (index, (index as isize, 0))));
        let hand_len = next_state.hands[0].len();

        let mut last_state = next_state
            .next_play(&plays)
            .expect("next_play should return Ok")
            .expect_right("Ok should contain last_state");

        assert_eq!(
            Color::COLORS_LEN * Shape::SHAPES_LEN,
            last_state.mut_board().len()
        );
        for (coordinate, tile) in Color::colors()
            .into_iter()
            .filter(|&other_color| color != other_color)
            .enumerate()
            .flat_map(|(row, color)| {
                Shape::shapes()
                    .into_iter()
                    .map(move |shape| (color, shape))
                    .enumerate()
                    .map(move |(col, tile)| ((col as isize, (row + 1) as isize), tile))
            })
        {
            assert_eq!(tile, last_state.mut_board()[&coordinate]);
        }

        for (coordinate, tile) in Shape::shapes()
            .into_iter()
            .map(|shape| (color, shape))
            .enumerate()
            .map(|(index, tile)| ((index as isize, 0), tile))
        {
            assert_eq!(tile, last_state.mut_board()[&coordinate]);
        }
        assert_eq!(
            // parallel line
            (Shape::SHAPES_LEN + FULL_MATCH_BONUS + LAST_PLAY_BONUS)
                // perpendicular lines
                + (Shape::SHAPES_LEN * (Color::COLORS_LEN + FULL_MATCH_BONUS)),
            last_state.mut_points()[0]
        );

        assert_eq!(hand_len, last_state.mut_hands()[0].len());
        for tile in &last_state.mut_hands()[0] {
            assert_eq!(bag_tile, *tile);
        }
    }

    fn set_up_next_play() -> (NextState, Plays) {
        let mut rng = rand::thread_rng();
        // avoid playing full match
        let hand_len = rng.gen_range(2..Shape::SHAPES_LEN);

        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        let color = rng.gen();
        next_state.hands[0].extend(
            Shape::shapes()
                .into_iter()
                .take(hand_len)
                .map(|shape| (color, shape)),
        );

        let tile = random_different_shape_same_color(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((0, 1), tile);

        let mut plays = BiBTreeMap::new();
        plays.extend((0..hand_len).map(|index| (index, (index as isize, 0))));

        (next_state, plays)
    }

    fn set_up_next_play_full_match() -> (NextState, Plays) {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        let color = rng.gen();
        next_state.hands[0].extend(Shape::shapes().into_iter().map(|shape| (color, shape)));

        let tile = random_different_color_same_shape(&mut rng, next_state.hands[0][0]);
        next_state.board.insert((0, 1), tile);

        let mut plays = BiBTreeMap::new();
        plays.extend((0..Shape::SHAPES_LEN).map(|index| (index, (index as isize, 0))));

        (next_state, plays)
    }

    fn set_up_next_play_double_match() -> (NextState, Plays) {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        let tile = rng.gen();
        let (color, shape) = tile;
        next_state.board.extend(
            Color::colors()
                .into_iter()
                .filter(|&other_color| color != other_color)
                .map(|other_color| (other_color, shape))
                .enumerate()
                .map(|(index, tile)| (((index + 1) as isize, 0), tile)),
        );
        next_state.board.extend(
            Shape::shapes()
                .into_iter()
                .filter(|&other_shape| shape != other_shape)
                .map(|other_shape| (color, other_shape))
                .enumerate()
                .map(|(index, tile)| ((0, (index + 1) as isize), tile)),
        );

        let hand = &mut next_state.hands[0];
        hand.push(tile);

        let mut plays = BiBTreeMap::new();
        plays.insert(0, (0, 0));

        (next_state, plays)
    }
}
