use crate::{Board, Color, Coordinate, Plays, Shape, Tile, COORDINATE_LIMIT, FULL_MATCH_BONUS};
use itertools::Itertools;
use map_macro::btree_set;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter::Peekable;

/// Partitions [plays](Plays) by whether each [play](Plays) is inside
/// the [coordinate limit](COORDINATE_LIMIT) or not.
///
/// # See Also
///
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// A tuple two collections of [plays](Plays) where the first collection contains
/// [coordinates](Coordinate) inside the [coordinate limit](COORDINATE_LIMIT) and
/// the second collection contains [coordinates](Coordinate) on or outside
/// the [coordinate limit](COORDINATE_LIMIT).
pub fn partition_by_coordinates(plays: &Plays) -> (Plays, Plays) {
    let x_outside_coordinate_limit = plays
        .right_range(..=(-COORDINATE_LIMIT, isize::MAX))
        .chain(plays.right_range((COORDINATE_LIMIT, isize::MIN)..))
        .map(|(&index, &coordinate)| (index, coordinate));
    let (y_outside_coordinate_limit, y_inside_coordinate_limit): (Plays, Plays) = plays
        .right_range((-COORDINATE_LIMIT + 1, isize::MIN)..(COORDINATE_LIMIT, isize::MIN))
        .map(|(&index, &coordinate)| (index, coordinate))
        .partition(|&(_, (_, y))| -COORDINATE_LIMIT >= y || y >= COORDINATE_LIMIT);
    (
        y_inside_coordinate_limit,
        x_outside_coordinate_limit
            .chain(y_outside_coordinate_limit)
            .collect(),
    )
}

/// Finds a collection of indexes where each item is a combination of length `k`
/// unique [tiles](Tile) from the hand where all [tiles](Tile) have either
/// the same [color](Color) or the same [shape](Shape).
///
/// # See Also
///
/// * [FirstState::first_play](crate::FirstState::first_play)
///
/// # Returns
///
/// A collection of possible plays of length `k` taken from the hand.
pub fn possible_plays<B, P>(hand: impl IntoIterator<Item = Tile>, k: usize) -> B
where
    P: FromIterator<usize>,
    B: FromIterator<P>,
{
    hand.into_iter()
        // indexes collected after filter
        .enumerate()
        // combinations must use unique tiles to prevent replacements
        .unique_by(|&(_, tile)| tile)
        // no replacements
        .combinations(k)
        // is combination matching?
        .filter(|combination: &Vec<(usize, Tile)>| {
            let mut iter = combination.iter();
            let Some((_,(first_color, first_shape))) = iter.next() else {
                return k == 0;
            };
            let Some((_,(second_color, second_shape))) = iter.next() else {
                return k == 1;
            };

            if first_color == second_color {
                iter.all(|(_, (other_color, _))| first_color == other_color)
            } else if first_shape == second_shape {
                iter.all(|(_, (_, other_shape))| first_shape == other_shape)
            } else {
                false
            }
        })
        // map vec to Indexes
        .map(|combination: Vec<(usize, Tile)>| {
            combination.into_iter().map(|(index, _)| index).collect()
        })
        .collect()
}

/// Takes a `line` of [tiles](Tile) being played on the board and returns
/// earned points if the `line` is legal. Otherwise, it returns duplicate groups
/// and/or multiple matching groups.
///
/// # Points Calculation
///
/// The number of points from a line is the number of [tiles](Tile) in that line. If the
/// line creates a full match on the board where a line contains either
/// [every color](Color::colors) or [every shape](Shape::shapes), an extra
/// [full match bonus](FULL_MATCH_BONUS) is earned.
///
/// # Arguments
///
/// * `line`: A map of [coordinates](Coordinate) to [tiles](Tile) being played
/// on the board.
///
/// # Errors
///
/// * If there are duplicate [tiles](Tile) in the `line`, groups of duplicates are returned
/// in the first tuple field.
/// * If there are multiple matching groups in the `line`, groups of matches are returned
/// in the second tuple field.
///
/// # See Also
///
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// The earned points of the `line`.
pub fn check_line(
    line: &Board,
) -> Result<
    usize,
    (
        BTreeSet<BTreeSet<Coordinate>>,
        BTreeSet<BTreeSet<Coordinate>>,
    ),
> {
    // Used at the end of the method
    let len = line.len();

    // Capacity set to highest expected demand.
    let mut duplicates = HashMap::with_capacity(len);
    let mut matching_colors = HashMap::with_capacity(Color::COLORS_LEN);
    let mut matching_shapes = HashMap::with_capacity(Shape::SHAPES_LEN);

    // Build groupings by tile, color, and shape
    for (&coordinate, &tile) in line {
        duplicates
            .entry(tile)
            .or_insert(BTreeSet::new())
            .insert(coordinate);
        let (color, shape) = tile;
        matching_colors
            .entry(color)
            .or_insert(BTreeSet::new())
            .insert(coordinate);
        matching_shapes
            .entry(shape)
            .or_insert(BTreeSet::new())
            .insert(coordinate);
    }

    // Filter duplicates from groupings
    let duplicates: BTreeSet<BTreeSet<Coordinate>> = duplicates
        .into_iter()
        .map(|(_, duplicates)| duplicates)
        .filter(|duplicates| duplicates.len() > 1)
        .collect();

    // Filter subsets from groupings
    let matching_colors: HashSet<BTreeSet<Coordinate>> = matching_colors
        .into_iter()
        .map(|(_, matching)| matching)
        .filter(|matching| {
            !matching_shapes
                .iter()
                .any(|(_, matching_shape)| matching.is_subset(matching_shape))
        })
        .collect();
    let matching_shapes: HashSet<BTreeSet<Coordinate>> = matching_shapes
        .into_iter()
        .map(|(_, matching)| matching)
        .filter(|matching| {
            !matching_colors
                .iter()
                .any(|matching_color| matching.is_subset(matching_color))
        })
        .collect();

    // If there are multiple matching groups or duplicates exist, return error(s).
    let multiple_matching_len = matching_colors.len() + matching_shapes.len();
    let mut multiple_matching = btree_set! {};
    if multiple_matching_len > 1 {
        multiple_matching.extend(matching_colors);
        multiple_matching.extend(matching_shapes);
        return Err((duplicates, multiple_matching));
    } else if !duplicates.is_empty() {
        return Err((duplicates, multiple_matching));
    };

    // If matching_shapes is not empty, then colors are different but the shapes are all the same
    // which means line is a color line. If line is not a color line,
    // it is a shape line. If the line is both (single tile), then line is
    // not long enough for bonus anyways. Checks if full match has been played for bonus.
    let is_color_line = !matching_shapes.is_empty();
    if (is_color_line && len == Color::COLORS_LEN) || len == Shape::SHAPES_LEN {
        Ok(len + FULL_MATCH_BONUS)
    } else {
        Ok(len)
    }
}

/// An ordered tuple where the second item is next value from `peekable` and the first item is
/// the last value in a continuous, decreasing range from the first to the last value.
/// It is possible for the first and last values to be the same when the next value after first
/// is not continuous or decreasing. If the `peekable` iteration is finished before the first value,
/// returns [None]. Otherwise, returns [Some].
///
/// # See Also
///
/// * [Itertools::batching]
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// An tuple containing the next range from `peekable`.
pub fn batch_continuous_decreasing_range<I>(peekable: &mut Peekable<I>) -> Option<(isize, isize)>
where
    I: Iterator<Item = isize>,
{
    let Some(first) = peekable.next() else {
        return None;
    };

    let mut last = first;
    while let Some(next) = peekable.next_if_eq(&(last - 1)) {
        last = next;
    }
    Some((last, first))
}

/// An ordered tuple where the first item is next value from `peekable` and the second item is
/// the last value in a continuous, increasing range from the first to the last value.
/// It is possible for the first and last values to be the same when the next value after first
/// is not continuous or increasing. If the `peekable` iteration is finished before the first value,
/// returns [None]. Otherwise, returns [Some].
///
/// # See Also
///
/// * [Itertools::batching]
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// An tuple containing the next range from `peekable`.
pub fn batch_continuous_increasing_range<I>(peekable: &mut Peekable<I>) -> Option<(isize, isize)>
where
    I: Iterator<Item = isize>,
{
    let Some(first) = peekable.next() else {
        return None;
    };

    let mut last = first;
    while let Some(next) = peekable.next_if_eq(&(last + 1)) {
        last = next;
    }
    Some((first, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{random_illegal_coordinates, random_legal_coordinates, Hand};
    use map_macro::hash_map;
    use rand::Rng;
    use std::iter;

    #[test]
    fn partition_by_coordinates_empty_plays() {
        test_partition_by_coordinates(Plays::new(), Plays::new(), Plays::new());
    }

    #[test]
    fn partition_by_coordinates_in_bounds() {
        let plays: Plays = (1..)
            .zip(random_legal_coordinates(&mut rand::thread_rng()))
            .collect();

        test_partition_by_coordinates(plays.clone(), plays, Plays::new());
    }

    #[test]
    fn partition_by_coordinates_out_of_bounds() {
        let plays: Plays = (1..)
            .zip(random_illegal_coordinates(&mut rand::thread_rng()))
            .collect();

        test_partition_by_coordinates(plays.clone(), Plays::new(), plays);
    }

    #[test]
    fn partition_by_coordinates_on_edge_case() {
        let plays: Plays = (1..)
            .zip([
                (0, COORDINATE_LIMIT),
                (0, -COORDINATE_LIMIT),
                (COORDINATE_LIMIT, 0),
                (-COORDINATE_LIMIT, 0),
            ])
            .collect();

        test_partition_by_coordinates(plays.clone(), Plays::new(), plays);
    }

    #[test]
    fn partition_by_coordinates_both_in_and_out_of_bounds() {
        let mut rng = rand::thread_rng();
        let legal_plays: Plays = (1..)
            .step_by(2)
            .zip(random_legal_coordinates(&mut rng))
            .collect();
        let illegal_plays: Plays = (2..)
            .step_by(2)
            .zip(random_illegal_coordinates(&mut rng))
            .collect();
        let plays = legal_plays
            .clone()
            .into_iter()
            .chain(illegal_plays.clone())
            .collect();

        test_partition_by_coordinates(plays, legal_plays, illegal_plays);
    }

    #[test]
    fn possible_plays_empty_hand() {
        let hand = Hand::new();

        let actual_plays: BTreeSet<BTreeSet<usize>> = possible_plays(hand, 1);

        assert!(actual_plays.is_empty());
    }

    #[test]
    fn possible_plays_k_0() {
        test_possible_plays(0, btree_set! { btree_set! {} });
    }

    #[test]
    fn possible_plays_k_1() {
        test_possible_plays(
            1,
            btree_set! {
              btree_set! { 0 },
              btree_set! { 1 },
              btree_set! { 2 },
              btree_set! { 3 },
              btree_set! { 5 },
            },
        );
    }

    #[test]
    fn possible_plays_k_2() {
        test_possible_plays(
            2,
            btree_set! {
              btree_set! { 0, 1 },
              btree_set! { 0, 2 },
              btree_set! { 1, 2 },
              btree_set! { 2, 3 },
            },
        );
    }

    #[test]
    fn possible_plays_k_3() {
        test_possible_plays(3, btree_set! { btree_set! { 0, 1, 2 } });
    }

    #[test]
    fn possible_plays_k_4() {
        test_possible_plays(4, btree_set! {});
    }

    #[test]
    fn check_line_duplicates() {
        let first_duplicate = (Color::Green, Shape::Square);
        let second_duplicate = (Color::Green, Shape::X);
        let third_duplicate = (Color::Green, Shape::Circle);
        test_check_line_error(
            hash_map! {
              (0, 0) => first_duplicate,
              (0, 1) => second_duplicate,
              (0, 2) => third_duplicate,
              (0, 3) => second_duplicate,
              (0, 4) => first_duplicate,
              (0, 5) => second_duplicate,
              (0, 6) => third_duplicate,
            },
            btree_set! {
              btree_set! { (0, 0), (0, 4) },
              btree_set! { (0, 1), (0, 3), (0, 5) },
              btree_set! { (0, 2), (0, 6) },
            },
            btree_set! {},
        );
    }

    #[test]
    fn check_line_multiple_matching() {
        test_check_line_error(
            hash_map! {
              (0, 0) => (Color::Green, Shape::Square),
              (0, 1) => (Color::Red, Shape::X),
              (0, 2) => (Color::Red, Shape::Clover),
              (0, 3) => (Color::Yellow, Shape::X),
              (0, 4) => (Color::Green, Shape::X),
              (0, 5) => (Color::Blue, Shape::Diamond),
            },
            btree_set! {},
            btree_set! {
              btree_set! { (0, 0), (0, 4) },
              btree_set! { (0, 1), (0, 3), (0, 4) },
              btree_set! { (0, 1), (0, 2) },
              btree_set! { (0, 5) },
            },
        );
    }

    #[test]
    fn check_line_duplicates_multiple_matching() {
        test_check_line_error(
            hash_map! {
              (0, 0) => (Color::Purple, Shape::Starburst),
              (0, 1) => (Color::Purple, Shape::Starburst),
              (0, 2) => (Color::Red, Shape::X),
            },
            btree_set! {
              btree_set! { (0, 0), (0, 1) }
            },
            btree_set! {
              btree_set! { (0, 0), (0, 1) },
              btree_set! { (0, 2) },
            },
        );
    }

    #[test]
    fn check_line_empty() {
        test_check_line(hash_map! {}, 0);
    }

    #[test]
    fn check_line_partial_match() {
        test_check_line(
            hash_map! {
              (0, 0) => (Color::Orange, Shape::Starburst),
              (0, 1) => (Color::Blue, Shape::Starburst),
              (0, 2) => (Color::Purple, Shape::Starburst),
            },
            3,
        );
    }

    #[test]
    fn check_line_full_match() {
        let color = rand::thread_rng().gen();
        let line: Board = Shape::shapes()
            .into_iter()
            .map(|shape| (color, shape))
            .enumerate()
            .map(|(index, tile)| ((index as isize, 0), tile))
            .collect();
        test_check_line(line, Shape::SHAPES_LEN + FULL_MATCH_BONUS);
    }

    #[test]
    fn batch_continuous_decreasing_range_none() {
        test_batch_continuous_decreasing_range(&mut iter::empty().peekable(), None);
    }

    #[test]
    fn batch_continuous_decreasing_range_one_value() {
        let first = rand::thread_rng().gen();
        test_batch_continuous_decreasing_range(
            &mut [first].into_iter().peekable(),
            Some((first, first)),
        );
    }

    #[test]
    fn batch_continuous_decreasing_range_not_continuous() {
        let first = rand::thread_rng().gen_range(isize::MIN + 2..=isize::MAX);
        test_batch_continuous_decreasing_range(
            &mut [first, first - 2].into_iter().peekable(),
            Some((first, first)),
        );
    }

    #[test]
    fn batch_continuous_decreasing_range_not_decreasing() {
        let first = rand::thread_rng().gen_range(isize::MIN..=isize::MAX - 1);
        test_batch_continuous_decreasing_range(
            &mut [first, first + 1].into_iter().peekable(),
            Some((first, first)),
        );
    }

    #[test]
    fn batch_continuous_decreasing_range_wide() {
        let mut rng = rand::thread_rng();
        let diff = rng.gen_range(100..200);
        let first = rng.gen_range(isize::MIN..=isize::MAX - diff);
        test_batch_continuous_decreasing_range(
            &mut (first - diff..=first).rev().peekable(),
            Some((first - diff, first)),
        );
    }

    #[test]
    fn batch_continuous_decreasing_range_peekable_not_finished() {
        let mut rng = rand::thread_rng();
        let diff = rng.gen_range(100..200);
        let first = rng.gen_range(isize::MIN..=isize::MAX - 2 * diff);
        let next = first + diff + 2..=first + 2 * diff;
        let mut peekable = (first..=first + diff).chain(next.clone()).rev().peekable();
        test_batch_continuous_decreasing_range(
            &mut peekable,
            Some((first + diff + 2, first + 2 * diff)),
        );
        assert!(peekable.eq((first..=first + diff).rev()));
    }

    #[test]
    fn batch_continuous_increasing_range_none() {
        test_batch_continuous_increasing_range(&mut iter::empty().peekable(), None);
    }

    #[test]
    fn batch_continuous_increasing_range_one_value() {
        let first = rand::thread_rng().gen();
        test_batch_continuous_increasing_range(
            &mut [first].into_iter().peekable(),
            Some((first, first)),
        );
    }

    #[test]
    fn batch_continuous_increasing_range_not_continuous() {
        let first = rand::thread_rng().gen_range(isize::MIN..=isize::MAX - 2);
        test_batch_continuous_increasing_range(
            &mut [first, first + 2].into_iter().peekable(),
            Some((first, first)),
        );
    }

    #[test]
    fn batch_continuous_increasing_range_not_increasing() {
        let first = rand::thread_rng().gen_range(isize::MIN + 1..=isize::MAX);
        test_batch_continuous_increasing_range(
            &mut [first, first - 1].into_iter().peekable(),
            Some((first, first)),
        );
    }

    #[test]
    fn batch_continuous_increasing_range_wide() {
        let mut rng = rand::thread_rng();
        let diff = rng.gen_range(100..200);
        let first = rng.gen_range(isize::MIN..=isize::MAX - diff);
        test_batch_continuous_increasing_range(
            &mut (first..=first + diff).peekable(),
            Some((first, first + diff)),
        );
    }

    #[test]
    fn batch_continuous_increasing_range_peekable_not_finished() {
        let mut rng = rand::thread_rng();
        let diff = rng.gen_range(100..200);
        let first = rng.gen_range(isize::MIN..=isize::MAX - 2 * diff);
        let next = first + diff + 2..=first + 2 * diff;
        let mut peekable = (first..=first + diff).chain(next.clone()).peekable();
        test_batch_continuous_increasing_range(&mut peekable, Some((first, first + diff)));
        assert!(peekable.eq(next));
    }

    fn test_partition_by_coordinates(
        plays: Plays,
        expected_coordinates_in_bounds: Plays,
        expected_coordinates_out_of_bounds: Plays,
    ) {
        let (actual_coordinates_in_bounds, actual_coordinates_out_of_bounds) =
            partition_by_coordinates(&plays);

        assert_eq!(expected_coordinates_in_bounds, actual_coordinates_in_bounds);
        assert_eq!(
            expected_coordinates_out_of_bounds,
            actual_coordinates_out_of_bounds
        );
    }

    fn test_possible_plays(k: usize, expected_plays: BTreeSet<BTreeSet<usize>>) {
        let hand = [
            (Color::Green, Shape::X),
            (Color::Green, Shape::Clover),
            (Color::Green, Shape::Square),
            (Color::Red, Shape::Square),
            (Color::Red, Shape::Square),
            (Color::Orange, Shape::Circle),
        ];

        let actual_plays = possible_plays(hand, k);

        assert_eq!(expected_plays, actual_plays);
    }

    fn test_check_line_error(
        line: Board,
        expected_duplicates: BTreeSet<BTreeSet<Coordinate>>,
        expected_multiple_matching: BTreeSet<BTreeSet<Coordinate>>,
    ) {
        let (actual_duplicates, actual_multiple_matching) =
            check_line(&line).expect_err("check_line should return Err");

        assert_eq!(expected_duplicates, actual_duplicates);
        assert_eq!(expected_multiple_matching, actual_multiple_matching);
    }

    fn test_check_line(line: Board, expected_points: usize) {
        let actual_points = check_line(&line).expect("check_line should return Ok");

        assert_eq!(expected_points, actual_points);
    }

    fn test_batch_continuous_decreasing_range<I>(
        peekable: &mut Peekable<I>,
        expected_range: Option<(isize, isize)>,
    ) where
        I: Iterator<Item = isize>,
    {
        let actual_range = batch_continuous_decreasing_range(peekable);

        assert_eq!(expected_range, actual_range);
    }

    fn test_batch_continuous_increasing_range<I>(
        peekable: &mut Peekable<I>,
        expected_range: Option<(isize, isize)>,
    ) where
        I: Iterator<Item = isize>,
    {
        let actual_range = batch_continuous_increasing_range(peekable);

        assert_eq!(expected_range, actual_range);
    }
}
