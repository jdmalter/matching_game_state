use std::cmp;

/// A tuple with two integer components for horizontal and vertical position on the board.
///
/// # See Also
///
/// * [Plays](crate::Plays)
/// * [FirstPlayError](crate::FirstPlayError)
/// * [NextPlayError](crate::NextPlayError)
pub type Coordinate = (isize, isize);

/// Finds the minimum and maximum components from [coordinates](Coordinate) for each component.
/// If `coordinates` is empty, [None] is returned.
///
/// # Arguments
///
/// * `coordinates`: An [iterator](Iterator) of [coordinates](Coordinate).
///
/// # See Also
///
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// A tuple with `4` different bounds in the following order:
///
/// * The minimum x component
/// * The minimum y component
/// * The maximum x component
/// * The maximum y component
pub fn find_component_minimums_and_maximums(
    mut coordinates: impl Iterator<Item = Coordinate>,
) -> Option<(isize, isize, isize, isize)> {
    let Some((x, y)) = coordinates.next() else {
        return None;
    };

    let (mut min_x, mut min_y, mut max_x, mut max_y) = (x, y, x, y);

    for (x, y) in coordinates {
        (min_x, min_y) = (cmp::min(min_x, x), cmp::min(min_y, y));
        (max_x, max_y) = (cmp::max(max_x, x), cmp::max(max_y, y));
    }

    Some((min_x, min_y, max_x, max_y))
}

/// Finds the [coordinate](Coordinate) with the minimum distance from the origin. If several
/// [coordinates](Coordinate) are equally distant from the origin, the first is returned.
/// If `coordinates` is empty, [None] is returned.
///
/// # Arguments
///
/// * `coordinates`: An [iterator](Iterator) of [coordinates](Coordinate).
///
/// # See Also
///
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// The [coordinate](Coordinate) with the minimum distance from the origin.
pub fn find_coordinate_by_minimum_distance(
    mut coordinates: impl Iterator<Item = Coordinate>,
) -> Option<Coordinate> {
    let Some(coordinate) = coordinates.next() else {
        return None;
    };

    fn distance(coordinate: Coordinate) -> f32 {
        ((coordinate.0 as f32).powi(2) + (coordinate.1 as f32).powi(2)).sqrt()
    }

    let mut minimum_coordinate = coordinate;
    let mut minimum_distance = distance(coordinate);

    for coordinate in coordinates {
        let other_distance = distance(coordinate);
        if other_distance < minimum_distance {
            minimum_coordinate = coordinate;
            minimum_distance = other_distance;
        }
    }

    Some(minimum_coordinate)
}

/// Finds the adjacent [coordinates](Coordinate) from the argument [coordinate](Coordinate)
/// where adjacent is 4 directional and not diagonal.
///
/// # Arguments
///
/// * `x`: The x component
/// * `y`: The y component
///
/// # See Also
///
/// * [NextState::next_play](crate::NextState::next_play)
///
/// # Returns
///
/// An array of 4 [coordinates](Coordinate) in natural lexicographic order.
pub fn adjacent_coordinates((x, y): Coordinate) -> [Coordinate; 4] {
    [(x - 1, y), (x, y - 1), (x, y + 1), (x + 1, y)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::{Distribution, Uniform};
    use rand::seq::SliceRandom;
    use rand::Rng;
    use std::iter;

    #[test]
    fn find_component_minimums_and_maximums_empty() {
        assert!(find_component_minimums_and_maximums(iter::empty()).is_none());
    }

    #[test]
    fn find_component_minimums_and_maximums_one_coordinate() {
        let (x, y) = rand::thread_rng().gen();

        test_find_component_minimums_and_maximums([(x, y)], (x, y, x, y));
    }

    #[test]
    fn find_component_minimums_and_maximums_mix_components() {
        let mut rng = rand::thread_rng();
        let (x1, y1) = (rng.gen_range(0..100), rng.gen_range(200..300));
        let (x2, y2) = (rng.gen_range(800..900), rng.gen_range(0..100));
        let (x3, y3) = (rng.gen_range(300..400), rng.gen_range(100..200));

        test_find_component_minimums_and_maximums([(x1, y1), (x2, y2), (x3, y3)], (x1, y2, x2, y1));
    }

    #[test]
    fn test_find_component_minimums_and_maximums_empty() {
        assert!(find_coordinate_by_minimum_distance(iter::empty()).is_none());
    }

    #[test]
    fn test_find_component_minimums_and_maximums_one_coordinate() {
        let coordinate = rand::thread_rng().gen();

        test_find_coordinate_by_minimum_distance([coordinate], coordinate);
    }

    #[test]
    fn test_find_component_minimums_and_maximums_different_coordinates_one_solution() {
        let mut rng = rand::thread_rng();
        let limit = rng.gen_range(5..10);
        let small_sample = Uniform::from(-limit..=limit);
        let large_sample = Uniform::from((limit + 1)..(2 * limit));
        let coordinate = (small_sample.sample(&mut rng), small_sample.sample(&mut rng));
        let coordinates = [(1, 1), (1, -1), (-1, -1), (-1, 1)]
            .into_iter()
            .flat_map(|(x_sign, y_sign)| {
                let count = rng.gen_range(2..=4);
                let mut vec = Vec::with_capacity(count);
                for _ in 0..count {
                    vec.push((
                        x_sign * large_sample.sample(&mut rng),
                        y_sign * large_sample.sample(&mut rng),
                    ))
                }
                vec.into_iter()
            })
            .chain(iter::once(coordinate));

        test_find_coordinate_by_minimum_distance(coordinates, coordinate);
    }
    #[test]
    fn test_find_component_minimums_and_maximums_different_coordinates_multiple_solutions() {
        let mut coordinates = [(1, 1), (1, -1), (-1, -1), (-1, 1)];
        coordinates.shuffle(&mut rand::thread_rng());

        test_find_coordinate_by_minimum_distance(coordinates, coordinates[0]);
    }

    #[test]
    fn test_adjacent_coordinates() {
        let actual_adjacent_coordinates = adjacent_coordinates((0, 0));
        let expected_adjacent_coordinates = [(-1, 0), (0, -1), (0, 1), (1, 0)];
        assert_eq!(expected_adjacent_coordinates, actual_adjacent_coordinates);
    }

    fn test_find_component_minimums_and_maximums(
        coordinates: impl IntoIterator<Item = Coordinate>,
        expected_component_minimums_and_maximums: (isize, isize, isize, isize),
    ) {
        let actual_component_minimums_and_maximums =
            find_component_minimums_and_maximums(coordinates.into_iter())
                .expect("find_component_minimums_and_maximums should return Some");

        assert_eq!(
            expected_component_minimums_and_maximums,
            actual_component_minimums_and_maximums
        );
    }

    fn test_find_coordinate_by_minimum_distance(
        coordinates: impl IntoIterator<Item = Coordinate>,
        expected_coordinate: Coordinate,
    ) {
        let actual_coordinate = find_coordinate_by_minimum_distance(coordinates.into_iter())
            .expect("find_coordinate_by_minimum_distance should return Some");

        assert_eq!(expected_coordinate, actual_coordinate);
    }
}
