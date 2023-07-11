use crate::state::{
    batch_holes, check_line, Board, Coordinate, CoordinateRange, Coordinates, LastState,
    NextOrLastState, NextState, Plays, Tile, LAST_PLAY_BONUS,
};
use bimap::BiBTreeMap;
use itertools::Itertools;
use std::collections::{BTreeSet, HashSet};
use std::ops::Index;
use std::{cmp, iter};

/// Describes the reason why [`NextState::next_play`] could not be executed.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NextPlayError {
    /// Attempting to play no tiles.
    EmptyPlays,
    /// Attempting to play tiles not in `current_player`'s hand.
    IndexesOutOfBounds {
        /// Plays where the index is greater than or equal to hand_len.
        indexes_out_of_bounds: Plays,
    },
    /// Attempting to play at already occupied coordinates on `board`.
    CoordinatesOccupied {
        /// Plays where `board` already contains a tile at the coordinate.
        coordinates_occupied: Plays,
    },
    /// Attempting to play tiles not connected to `board`.
    NotConnected {
        /// Plays where there are no adjacent tiles on `board` or no path through other
        /// connected plays to a tile on `board`.
        not_connected: Plays,
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

impl NextState {
    /// Checks if the plays are valid, then removes the tiles from `current_player`'s hand,
    /// inserts those into `board` at the coordinates, attempts to fill `current_player`'s hand
    /// up to its previous length, adds `points` earned by the play to the current player, and
    /// advances to the next player if the game has not ended.
    ///
    /// # Points Calculation
    ///
    /// The number of `points` earned by a play is the sum of `points` scored from each line that
    /// contains played tiles. Each tile can be counted twice if the tile is a part of
    /// a vertical and horizontal line.
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
    /// [`LAST_PLAY_BONUS`](LAST_PLAY_BONUS) `points`.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    ///
    /// # Errors
    ///
    /// * [`NextPlayError::EmptyPlays`] Attempting to play no tiles.
    /// * [`NextPlayError::IndexesOutOfBounds`] Attempting to play tiles not in the current
    /// player's hand.
    /// * [`NextPlayError::CoordinatesOccupied`] Attempting to play at already occupied coordinates
    /// on `board`.
    /// * [`NextPlayError::NotConnected`] Attempting to play tiles not connected to `board`.
    /// * [`NextPlayError::NoLegalPlays`] Attempting to only play illegal plays.
    /// * [`NextPlayError::NoLegalLines`] Not attempting to play tiles in a point or a line.
    /// * [`NextPlayError::Holes`] Attempting to play tiles in a line but not the same
    /// connected line.
    /// * [`NextPlayError::Duplicates`] Attempting to play duplicate tiles in a line.
    /// * [`NextPlayError::MultipleMatching`] Attempting to play a line where tiles are not either
    /// the same shape or the same color.
    ///
    /// # Returns
    ///
    /// The [`NextOrLastState`] of the game after the first turn.
    pub fn next_play(
        mut self,
        plays: &Plays,
    ) -> Result<NextOrLastState, (Self, HashSet<NextPlayError>)> {
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
        hand.extend(self.bag.drain(self.bag.len().saturating_sub(plays.len())..));

        if self.has_ended() {
            self.points[self.current_player] += next_play_points + LAST_PLAY_BONUS;
            Ok(NextOrLastState::Last(LastState::new(
                self.board,
                self.points,
                self.hands,
            )))
        } else {
            self.points[self.current_player] += next_play_points;
            self.current_player = (self.current_player + 1) % self.hands.len();
            Ok(NextOrLastState::Next(self))
        }
    }

    /// Takes a bimap of indexes of tiles to be played to coordinates on `board` and returns
    /// earned points if the bimap only creates legal lines, otherwise it returns the errors.
    ///
    /// # Points Calculation
    ///
    /// The number of points earned by a play is the sum of points scored from each line that
    /// contains played tiles. Each tile can be counted twice if the tile is a part of
    /// a vertical and horizontal line.
    ///
    /// The number of points from a line is the number of tiles in that line. If the line creates
    /// a full match on `board` where a line contains either every color in
    /// [`Color::colors`](crate::state::Color::colors) or every shape in
    /// [`Shape::shapes`](crate::state::Shape::shapes),
    /// gives an extra [`FULL_MATCH_BONUS`](crate::state::FULL_MATCH_BONUS) points.
    ///
    /// # Arguments
    ///
    /// * `plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    ///
    /// # Errors
    ///
    /// * [`NextPlayError::EmptyPlays`] Attempting to play no tiles.
    /// * [`NextPlayError::IndexesOutOfBounds`] Attempting to play tiles not in the current
    /// player's hand.
    /// * [`NextPlayError::CoordinatesOutOfBounds`] Attempting to play tiles outside of `board`.
    /// * [`NextPlayError::CoordinatesOccupied`] Attempting to play at already occupied coordinates
    /// on `board`.
    /// * [`NextPlayError::NotConnected`] Attempting to play tiles not connected to `board`.
    /// * [`NextPlayError::NoLegalPlays`] Attempting to only play illegal plays.
    /// * [`NextPlayError::NoLegalLines`] Not attempting to play tiles in a point or a line.
    /// * [`NextPlayError::Holes`] Attempting to play tiles in a line but not the same
    /// connected line.
    /// * [`NextPlayError::Duplicates`] Attempting to play duplicate tiles in a line.
    /// * [`NextPlayError::MultipleMatching`] Attempting to play a line where tiles are not either
    /// the same shape or the same color.
    ///
    /// # Returns
    ///
    /// The earned points from plays.
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

        let (coordinates_unoccupied, coordinates_occupied): (Plays, Plays) = plays
            .into_iter()
            .map(|(&index, &coordinate)| (index, coordinate))
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

        // filter out empty legal_plays and find coordinate
        let Some(&(x,y)) = legal_plays.right_values().next() else {
            errors.insert(NextPlayError::NoLegalPlays);
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
        let (holes, lines): (BTreeSet<CoordinateRange>, Vec<Board>) = if min_x == max_x {
            let holes = (min_y..=max_y)
                .filter(|&y| {
                    let coordinate = (min_x, y);
                    !self.board.contains_key(&coordinate) && !plays.contains_right(&coordinate)
                })
                .peekable()
                .batching(batch_holes)
                .map(|(first, last)| ((min_x, first), (min_x, last)))
                .collect();
            // horizontal lines perpendicular to the vertical line legal_plays
            let lines = self.find_horizontal_lines(&legal_plays, min_x, min_y);

            (holes, lines)
        } else if min_y == max_y {
            let holes = (min_x..=max_x)
                .filter(|&x| {
                    let coordinate = (x, min_y);
                    !self.board.contains_key(&coordinate) && !plays.contains_right(&coordinate)
                })
                .peekable()
                .batching(batch_holes)
                .map(|(first, last)| ((first, min_y), (last, min_y)))
                .collect();
            // vertical lines perpendicular to the horizontal line legal_plays
            let lines = self.find_vertical_lines(&legal_plays, min_x, min_y);

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

    /// Partitions unoccupied coordinates by whether the coordinate is connected to `board`.
    /// A coordinate can be connected either directly by an adjacent tile in `board` or indirectly
    /// by a path of adjacent, indirectly connected coordinates to a directly connected coordinate.
    ///
    /// # Returns
    ///
    /// A tuple of connected and not connected plays.
    #[inline]
    fn partition_connected(&self, coordinates_unoccupied: Plays) -> (Plays, Plays) {
        let mut connected = BiBTreeMap::new();
        let mut not_connected = coordinates_unoccupied;
        let capacity = not_connected.len();

        for (&stack_index, &stack_coordinate) in not_connected.iter() {
            if connected.contains_right(&stack_coordinate) {
                // Avoid duplicate searching
                continue;
            }

            // DFS with visited set to prevent cycles
            let mut stack = Vec::with_capacity(capacity);
            stack.push((stack_index, stack_coordinate));
            let mut visited = BiBTreeMap::new();

            while let Some((index, coordinate)) = stack.pop() {
                if !visited.insert(index, coordinate).did_overwrite() {
                    let (x, y) = coordinate;
                    for adjacent_coordinate in [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)] {
                        if self.board.contains_key(&adjacent_coordinate)
                            || connected.contains_right(&adjacent_coordinate)
                        {
                            // connected directly or indirectly
                            connected.extend(visited.clone());
                            break;
                        } else if let Some(&index) =
                            not_connected.get_by_right(&adjacent_coordinate)
                        {
                            // not yet connected
                            stack.push((index, adjacent_coordinate));
                        }
                    }
                }
            }
        }

        // execute partition
        not_connected.retain(|index, _| !connected.contains_left(index));
        (connected, not_connected)
    }

    /// # Arguments
    ///
    /// * `legal_plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    /// * `x`: The horizontal part of a location on `board` or in `legal_plays`
    /// which might contain a tile.
    /// * `y`: The vertical part of a location on `board` or in `legal_plays`
    /// which might contain a tile.
    ///
    /// # Returns
    ///
    /// A vector of horizontal lines extending from each tile in `legal_plays` plus the vertical
    /// line of `legal_plays` itself.
    #[inline]
    fn find_horizontal_lines(&self, legal_plays: &Plays, x: isize, y: isize) -> Vec<Board> {
        let increasing = (y..)
            .map(|next_y| self.get_board_or_plays(legal_plays, x, next_y))
            .while_some();
        let decreasing = (1..)
            .map(|offset| y - offset)
            .map(|next_y| self.get_board_or_plays(legal_plays, x, next_y))
            .while_some();
        let vertical_line: Board = increasing.chain(decreasing).collect();

        legal_plays
            .iter()
            .map(|(&index, &(x, y))| {
                let from_line = iter::once(((x, y), self.hands[self.current_player][index]));
                let increasing = (x + 1..)
                    .map(|next_x| self.get_board(next_x, y))
                    .while_some();
                let decreasing = (1..)
                    .map(|offset| x - offset)
                    .map(|next_x| self.get_board(next_x, y))
                    .while_some();
                from_line.chain(increasing).chain(decreasing).collect()
            })
            .filter(|line: &Board| line.len() > 1)
            .chain(iter::once(vertical_line))
            .collect()
    }

    /// # Arguments
    ///
    /// * `legal_plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    /// * `x`: The horizontal part of a location on `board` or in `legal_plays`
    /// which might contain a tile.
    /// * `y`: The vertical part of a location on `board` or in `legal_plays`
    /// which might contain a tile.
    ///
    /// # Returns
    ///
    /// A vector of vertical lines extending from each tile in `legal_plays` plus the horizontal
    /// line of `legal_plays` itself.
    #[inline]
    fn find_vertical_lines(&self, legal_plays: &Plays, x: isize, y: isize) -> Vec<Board> {
        let increasing = (x..)
            .map(|next_x| self.get_board_or_plays(legal_plays, next_x, y))
            .while_some();
        let decreasing = (1..)
            .map(|offset| x - offset)
            .map(|next_x| self.get_board_or_plays(legal_plays, next_x, y))
            .while_some();
        let horizontal_line: Board = increasing.chain(decreasing).collect();

        legal_plays
            .iter()
            .map(|(&index, &(x, y))| {
                let from_line = iter::once(((x, y), self.hands[self.current_player][index]));
                let increasing = (y + 1..)
                    .map(|next_y| self.get_board(x, next_y))
                    .while_some();
                let decreasing = (1..)
                    .map(|offset| y - offset)
                    .map(|next_y| self.get_board(x, next_y))
                    .while_some();
                from_line.chain(increasing).chain(decreasing).collect()
            })
            .filter(|line: &Board| line.len() > 1)
            .chain(iter::once(horizontal_line))
            .collect_vec()
    }

    /// If there is a tile is in `board` at `(x, y)`, returns that tile.
    ///
    /// # Arguments
    ///
    /// * `x`: The horizontal part of a location on `board` which might contain a tile.
    /// * `y`: The vertical part of a location on `board` which might contain a tile.
    ///
    /// # Returns
    ///
    /// A tuple of the coordinate and the tile.
    #[inline]
    fn get_board(&self, x: isize, y: isize) -> Option<(Coordinate, Tile)> {
        let coordinate = (x, y);
        self.board.get(&coordinate).map(|&tile| (coordinate, tile))
    }

    /// If there is a tile in `legal_plays` at `(x, y)`, returns that tile.
    /// Otherwise if there is a tile is in `board` at `(x, y)`, returns that tile.
    ///
    /// # Arguments:
    ///
    /// * `legal_plays`: A bimap of indexes of tiles to be played to coordinates on `board`.
    /// * `x`: The horizontal part of a location on `board` which might contain a tile.
    /// * `y`: The vertical part of a location on `board` which might contain a tile.
    ///
    /// # Returns
    ///
    /// A tuple of the coordinate and the tile.
    #[inline]
    fn get_board_or_plays(
        &self,
        legal_plays: &Plays,
        x: isize,
        y: isize,
    ) -> Option<(Coordinate, Tile)> {
        let coordinate = (x, y);
        legal_plays
            .get_by_right(&coordinate)
            .map(|&index| self.hands[self.current_player].index(index))
            .or_else(|| self.board.get(&coordinate))
            .map(|&tile| (coordinate, tile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Color, Hand, Shape, Tile, FULL_MATCH_BONUS, HAND_CAPACITY};
    use bimap::BiBTreeMap;
    use map_macro::{btree_set, set};
    use rand::seq::SliceRandom;
    use rand::Rng;
    use tap::Tap;

    #[test]
    fn empty_plays() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_hands(&mut rng);
        let plays = BiBTreeMap::new();

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! { NextPlayError::EmptyPlays };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn indexes_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);
        let tile = random_matching_tile(&mut rng, &next_state.hands[0][0]);
        next_state.board.insert((0, -1), tile);

        let indexes_out_of_bounds: Plays = (1..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (0, index as isize)))
            .collect();

        let mut plays = BiBTreeMap::new();
        plays.insert(0, (0, 0));
        plays.extend(indexes_out_of_bounds.clone());

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! { NextPlayError::IndexesOutOfBounds {
            indexes_out_of_bounds
        }};
        assert_eq!(expected_error, actual_error);
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

        let mut plays = BiBTreeMap::new();
        if let Some(&(x, y)) = next_state.board.keys().next() {
            let tile = random_matching_tile(&mut rng, &next_state.hands[0][0]);
            next_state.board.insert((x, y), tile);
            plays.insert(0, (x, y + 1));
        }
        plays.extend(coordinates_occupied.clone());

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! { NextPlayError::CoordinatesOccupied {
            coordinates_occupied
        }};
        assert_eq!(expected_error, actual_error);
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

        let mut plays = BiBTreeMap::new();
        let tile = random_matching_tile(&mut rng, &next_state.hands[0][0]);
        next_state.board.insert((0, -1), tile);
        plays.insert(0, (0, 0));
        plays.extend(not_connected.clone());

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! { NextPlayError::NotConnected {
            not_connected
        }};
        assert_eq!(expected_error, actual_error);
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

        let mut plays = BiBTreeMap::new();
        if let Some(&(x, y)) = next_state.board.keys().next() {
            let tile = random_matching_tile(&mut rng, &next_state.hands[0][0]);
            next_state.board.insert((x, y), tile);
            plays.insert(0, (x, y + 1));
        }
        plays.extend(illegal_plays.clone());

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
            NextPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds: illegal_plays.clone()
            },
            NextPlayError::CoordinatesOccupied {
                coordinates_occupied: illegal_plays
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn indexes_out_of_bounds_not_connected_no_legal_plays() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let hand_len = next_state.random_hands(&mut rng);

        let illegal_plays: Plays = (0..rng.gen_range(3..=6))
            .map(|index| (hand_len + index, (0, index as isize)))
            .collect();

        let plays = illegal_plays.clone();

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
            NextPlayError::IndexesOutOfBounds {
                indexes_out_of_bounds: illegal_plays.clone()
            },
            NextPlayError::NotConnected {
                not_connected: illegal_plays
            },
            NextPlayError::NoLegalPlays
        };
        assert_eq!(expected_error, actual_error);
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

        let hand = &mut next_state.hands[0];
        hand.extend([
            (color, Shape::Starburst),
            (color, Shape::X),
            (color, Shape::Clover),
        ]);

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, 0)), (1, (1, 1)), (2, (2, 2))]);

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! { NextPlayError::NoLegalLines };
        assert_eq!(expected_error, actual_error);
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

        let hand = &mut next_state.hands[0];
        hand.extend([
            (Color::Purple, shape),
            (Color::Green, shape),
            (Color::Red, shape),
        ]);

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, 0)), (1, (0, 2)), (2, (0, 5))]);

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
            NextPlayError::Holes {
                holes: btree_set! { ((0, 1), (0, 1)), ((0, 3), (0, 4)) }
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn duplicates_vertical() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let tile = rng.gen();
        let matching_tile = random_matching_tile(&mut rng, &tile);
        next_state.board.insert((0, 0), tile);
        next_state.board.insert((0, 1), matching_tile);

        let hand = &mut next_state.hands[0];
        hand.extend([tile, tile]);

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (1, 0)), (1, (1, 1))]);

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
            NextPlayError::Duplicates {
                duplicates: btree_set! { btree_set! {(1, 0), (1, 1)} }
            },
            NextPlayError::Duplicates {
                duplicates: btree_set! { btree_set! {(0, 0), (1, 0)} }
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn duplicates_horizontal() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        let tile = rng.gen();
        let matching_tile = random_matching_tile(&mut rng, &tile);
        next_state.board.insert((0, 0), tile);
        next_state.board.insert((1, 0), matching_tile);

        let hand = &mut next_state.hands[0];
        hand.extend([tile, tile]);

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, 1)), (1, (1, 1))]);

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
            NextPlayError::Duplicates {
                duplicates: btree_set! { btree_set! {(0, 1), (1, 1)} }
            },
            NextPlayError::Duplicates {
                duplicates: btree_set! { btree_set! {(0, 0), (0, 1)} }
            }
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn multiple_matching_vertical() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.board.extend([
            ((-1, -1), (Color::Green, Shape::Diamond)),
            ((1, -1), (Color::Red, Shape::Square)),
            ((-1, 0), (Color::Green, Shape::X)),
            ((1, 0), (Color::Yellow, Shape::Circle)),
            ((-1, 1), (Color::Blue, Shape::Starburst)),
            ((1, 1), (Color::Purple, Shape::Circle)),
        ]);

        let hand = &mut next_state.hands[0];
        hand.extend([
            (Color::Green, Shape::Square),
            (Color::Green, Shape::Circle),
            (Color::Blue, Shape::Circle),
        ]);

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (0, -1)), (1, (0, 0)), (2, (0, 1))]);

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
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
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn multiple_matching_horizontal() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.board.extend([
            ((-1, -1), (Color::Green, Shape::Diamond)),
            ((-1, 1), (Color::Red, Shape::Square)),
            ((0, -1), (Color::Green, Shape::X)),
            ((0, 1), (Color::Yellow, Shape::Circle)),
            ((1, -1), (Color::Blue, Shape::Starburst)),
            ((1, 1), (Color::Purple, Shape::Circle)),
        ]);

        let hand = &mut next_state.hands[0];
        hand.extend([
            (Color::Green, Shape::Square),
            (Color::Green, Shape::Circle),
            (Color::Blue, Shape::Circle),
        ]);

        let mut plays = BiBTreeMap::new();
        plays.extend([(0, (-1, 0)), (1, (0, 0)), (2, (1, 0))]);

        let (_, actual_error) = next_state.next_play(&plays).unwrap_err();

        let expected_error = set! {
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
        };
        assert_eq!(expected_error, actual_error);
    }

    #[test]
    fn next_play_tiles() {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);

        next_state.board.insert((1, 10), (Color::Yellow, Shape::X));

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
        plays.extend([(0, (0, 10)), (1, (0, 11)), (3, (0, 12))]);
        let plays_len = plays.len();

        let next_state = next_state.next_play(&plays).unwrap().unwrap_next();

        let hand = &next_state.hands[0];
        assert_eq!(third, hand[0]);
        assert_eq!(bag_tile, hand[1]);
        assert_eq!(bag_tile, hand[2]);
        assert_eq!(bag_tile, hand[3]);

        assert_eq!(bag_len - plays_len, next_state.bag.len());
        assert_eq!(first, next_state.board[&(0, 10)]);
        assert_eq!(second, next_state.board[&(0, 11)]);
        assert_eq!(fourth, next_state.board[&(0, 12)]);
    }

    #[test]
    fn next_play_some_points() {
        let (next_state, plays) = set_up_next_play();

        let next_state = next_state.next_play(&plays).unwrap().unwrap_next();

        assert_eq!(2 + plays.len(), next_state.points[0]);
    }

    #[test]
    fn next_play_some_points_last_play() {
        let (mut next_state, plays) = set_up_next_play();
        next_state.bag.clear();

        let mut last_state = next_state.next_play(&plays).unwrap().unwrap_last();

        assert_eq!(
            2 + plays.len() + LAST_PLAY_BONUS,
            last_state.mut_points()[0]
        );
    }

    #[test]
    fn next_play_full_match() {
        let (next_state, plays) = set_up_next_play_full_match();

        let next_state = next_state.next_play(&plays).unwrap().unwrap_next();

        assert_eq!(2 + plays.len() + FULL_MATCH_BONUS, next_state.points[0]);
    }

    #[test]
    fn next_play_double_full_match() {
        let (next_state, plays) = set_up_next_play_double_match();

        let next_state = next_state.next_play(&plays).unwrap().unwrap_next();

        assert_eq!(
            Color::COLORS_LEN + Shape::SHAPES_LEN + 2 * FULL_MATCH_BONUS,
            next_state.points[0]
        );
    }

    #[test]
    fn next_play_double_full_match_last_play() {
        let (mut next_state, plays) = set_up_next_play_double_match();
        next_state.bag.clear();

        let mut last_state = next_state.next_play(&plays).unwrap().unwrap_last();

        assert_eq!(
            Color::COLORS_LEN + Shape::SHAPES_LEN + 2 * FULL_MATCH_BONUS + LAST_PLAY_BONUS,
            last_state.mut_points()[0]
        );
    }

    #[test]
    fn next_play_full_match_last_play() {
        let (mut next_state, plays) = set_up_next_play_full_match();
        next_state.bag.clear();

        let mut last_state = next_state.next_play(&plays).unwrap().unwrap_last();

        assert_eq!(
            2 + plays.len() + FULL_MATCH_BONUS + LAST_PLAY_BONUS,
            last_state.mut_points()[0]
        );
    }

    #[test]
    fn next_play_increment_current_player() {
        let (next_state, plays) = set_up_next_play();

        let next_state = next_state.next_play(&plays).unwrap().unwrap_next();

        assert_eq!(1, next_state.current_player);
    }

    #[test]
    fn next_play_wrap_current_player() {
        let (mut next_state, plays) = set_up_next_play();
        next_state.current_player = next_state.hands.len() - 1;
        let next_hand = next_state.hands[0].clone();
        next_state.hands[next_state.current_player].extend(next_hand);

        let next_state = next_state.next_play(&plays).unwrap().unwrap_next();

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
                        .map(move |(col, tile)| (((row + 1) as isize, col as isize), tile))
                }),
        );

        next_state.points.push(0);
        next_state.hands.push(Hand::with_capacity(HAND_CAPACITY));
        let hand = &mut next_state.hands[0];
        let mut plays = BiBTreeMap::new();
        hand.extend(Shape::shapes().into_iter().map(|shape| (color, shape)));
        plays.extend((0..hand.len()).map(|index| (index, (0, index as isize))));
        let hand_len = next_state.hands[0].len();

        let mut last_state = next_state.next_play(&plays).unwrap().unwrap_last();

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
                    .map(move |(col, tile)| (((row + 1) as isize, col as isize), tile))
            })
        {
            assert_eq!(tile, last_state.mut_board()[&coordinate]);
        }

        for (coordinate, tile) in Shape::shapes()
            .into_iter()
            .map(|shape| (color, shape))
            .enumerate()
            .map(|(index, tile)| ((0, index as isize), tile))
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

    #[inline]
    fn random_matching_tile<R: Rng + ?Sized>(rng: &mut R, tile: &Tile) -> Tile {
        let &(color, shape) = tile;
        Shape::shapes()
            .tap_mut(|shapes| shapes.shuffle(rng))
            .into_iter()
            .filter(|&other_shape| shape != other_shape)
            .map(|shape| (color, shape))
            .next()
            .unwrap_or_else(|| {
                dbg!(tile, Shape::shapes());
                unreachable!("Shape::shapes() should contain more than one shape");
            })
    }

    #[inline]
    fn set_up_next_play() -> (NextState, Plays) {
        let mut rng = rand::thread_rng();
        let hand_len = rng.gen_range(2..=5);

        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        let color = rng.gen();
        let hand = &mut next_state.hands[0];
        hand.extend(
            Shape::shapes()
                .into_iter()
                .take(hand_len)
                .map(|shape| (color, shape)),
        );

        let tile = random_matching_tile(&mut rng, &next_state.hands[0][0]);
        next_state.board.insert((1, 0), tile);

        let mut plays = BiBTreeMap::new();
        plays.extend((0..hand_len).map(|index| (index, (0, index as isize))));

        (next_state, plays)
    }

    #[inline]
    fn set_up_next_play_full_match() -> (NextState, Plays) {
        let mut rng = rand::thread_rng();
        let mut next_state = NextState::empty_next_state();
        next_state.random_players(&mut rng);
        next_state.random_bag(&mut rng);
        let color = rng.gen();
        let hand = &mut next_state.hands[0];
        hand.extend(Shape::shapes().into_iter().map(|shape| (color, shape)));

        let tile = random_matching_tile(&mut rng, &next_state.hands[0][0]);
        next_state.board.insert((1, 0), tile);

        let mut plays = BiBTreeMap::new();
        plays.extend((0..Shape::SHAPES_LEN).map(|index| (index, (0, index as isize))));

        (next_state, plays)
    }

    #[inline]
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
