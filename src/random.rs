use crate::{
    Bag, Board, Color, Coordinate, Hand, Hands, Points, Shape, Tile, COORDINATE_LIMIT,
    HAND_CAPACITY, PLAYER_CAPACITY,
};
use rand::distributions::{Distribution, Uniform};
use rand::seq::SliceRandom;
use rand::Rng;
use tap::Tap;

/// It inserts a random, small, non-zero number of empty hands into hands and
/// `0`s into points.
///
/// Intended to be used during the `Next` and `Last` phases  but not during the `First` phase.
///
/// # Returns
///
/// The number of additional points/hands.
pub fn random_players<R: Rng + ?Sized>(
    rng: &mut R,
    points: &mut Points,
    hands: &mut Hands,
) -> usize {
    let players = rng.gen_range(2..=PLAYER_CAPACITY);
    for _ in 0..players {
        points.push(0);
        hands.push(Hand::with_capacity(HAND_CAPACITY));
    }

    players
}

/// It inserts a random, small, non-zero number of [tiles](Tile) into the bag.
///
/// # Returns
///
/// The number of additional [tiles](Tile) in the bag.
pub fn random_bag<R: Rng + ?Sized>(rng: &mut R, bag: &mut Bag) -> usize {
    let bag_len = rng.gen_range(10..20);
    bag.extend((0..bag_len).map(|_| rng.gen::<Tile>()));

    bag_len
}

/// It inserts one [tile](Tile) for every other x in a random, small, non-zero horizontal range
/// at a random, small y into the board.
///
/// # Returns
///
/// The number of additional [tiles](Tile) on the board.
pub fn random_board<R: Rng + ?Sized>(rng: &mut R, board: &mut Board) -> usize {
    let possible_coordinates = Uniform::from(-20..=20);
    let board_len_isize: isize = rng.gen_range(5..10);
    let board_len: usize = (board_len_isize as usize) + 1;

    board.extend(
        (-board_len_isize..=board_len_isize)
            .step_by(2)
            .map(|index| ((index, possible_coordinates.sample(rng)), rng.gen())),
    );

    board_len
}

/// Sets each player's points to a random, medium, non-zero number.
pub fn random_points<R: Rng + ?Sized>(rng: &mut R, points: &mut Points) {
    let possible_points = Uniform::from(100..200);
    points.fill_with(|| possible_points.sample(rng));
}

/// Pushes the same random, small, non-zero number of [tiles](Tile) into
/// each player's hand.
///
/// # Returns
///
/// The number of additional [tiles](Tile) in each player's hand.
pub fn random_hands<R: Rng + ?Sized>(rng: &mut R, hands: &mut Hands) -> usize {
    let hand_len = rng.gen_range(2..=HAND_CAPACITY);
    for hand in hands {
        hand.extend((0..hand_len).map(|_| rng.gen::<Tile>()));
    }

    hand_len
}

/// If `players` is not `0`, sets the current player to a random number between `0` inclusive
/// and `players` exclusive. Otherwise, does nothing.
///
/// # Returns
///   
/// The index of the player whose turn it is.
pub fn random_current_player<R: Rng + ?Sized>(
    rng: &mut R,
    current_player: &mut usize,
    players: usize,
) -> usize {
    if players > 0 {
        *current_player = rng.gen_range(0..players);
    }

    *current_player
}

/// A new [tile](Tile) with a random, different [shape](Shape) but the same [color](Color).
pub fn random_different_shape_same_color<R: Rng + ?Sized>(
    rng: &mut R,
    (color, shape): (Color, Shape),
) -> Tile {
    let possible_indexes = Uniform::from(0..Shape::SHAPES_LEN - 1);
    let random_index = possible_indexes.sample(rng);
    // removing the shape at its own index in the array shapes
    let random_different_index = random_index + if random_index < shape as usize { 0 } else { 1 };
    (color, Shape::shapes()[random_different_index])
}

/// A new [tile](Tile) with a random, different [color](Color) but the same [shape](Shape).
pub fn random_different_color_same_shape<R: Rng + ?Sized>(
    rng: &mut R,
    (color, shape): (Color, Shape),
) -> Tile {
    let possible_indexes = Uniform::from(0..Color::COLORS_LEN - 1);
    let random_index = possible_indexes.sample(rng);
    // removing the color at its own index in the array colors
    let random_different_index = random_index + if random_index < color as usize { 0 } else { 1 };
    (Color::colors()[random_different_index], shape)
}

/// An [iterator](Iterator) of [coordinates](Coordinate) where the values of both components lie
/// inside the range -[COORDINATE_LIMIT] exclusive to [COORDINATE_LIMIT] exclusive.
pub fn random_legal_coordinates<R: Rng + ?Sized>(rng: &mut R) -> impl Iterator<Item = Coordinate> {
    let possible_legal_coordinates = Uniform::from(0..COORDINATE_LIMIT);
    [
        (
            -possible_legal_coordinates.sample(rng),
            -possible_legal_coordinates.sample(rng),
        ),
        (
            -possible_legal_coordinates.sample(rng),
            possible_legal_coordinates.sample(rng),
        ),
        (
            possible_legal_coordinates.sample(rng),
            -possible_legal_coordinates.sample(rng),
        ),
        (
            possible_legal_coordinates.sample(rng),
            possible_legal_coordinates.sample(rng),
        ),
    ]
    .tap_mut(|coordinates| coordinates.shuffle(rng))
    .into_iter()
}

/// An [iterator](Iterator) of [coordinates](Coordinate) where the value of some component lies
/// outside the range -[COORDINATE_LIMIT] exclusive to [COORDINATE_LIMIT] exclusive.
pub fn random_illegal_coordinates<R: Rng + ?Sized>(
    rng: &mut R,
) -> impl Iterator<Item = Coordinate> {
    let possible_coordinates = Uniform::from(0..isize::MAX);
    let possible_illegal_coordinates = Uniform::from(COORDINATE_LIMIT..isize::MAX);
    [
        (
            -possible_coordinates.sample(rng),
            -possible_illegal_coordinates.sample(rng),
        ),
        (
            -possible_illegal_coordinates.sample(rng),
            possible_coordinates.sample(rng),
        ),
        (
            possible_illegal_coordinates.sample(rng),
            -possible_coordinates.sample(rng),
        ),
        (
            possible_coordinates.sample(rng),
            possible_illegal_coordinates.sample(rng),
        ),
    ]
    .tap_mut(|coordinates| coordinates.shuffle(rng))
    .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hand, HAND_CAPACITY, PLAYER_CAPACITY, TILES_LEN};
    use itertools::Itertools;

    #[test]
    fn random_players_empty() {
        let mut points = Points::with_capacity(PLAYER_CAPACITY);
        let mut hands = Hands::with_capacity(PLAYER_CAPACITY);

        let players = random_players(&mut rand::thread_rng(), &mut points, &mut hands);

        assert_eq!(players, points.len());
        assert_eq!(players, hands.len());

        for player in 0..players {
            assert_eq!(0, points[player]);
            assert!(hands[player].is_empty());
        }
    }

    #[test]
    fn random_bag_empty() {
        let mut bag = Bag::new();

        assert!(bag.is_empty());

        let bag_len = random_bag(&mut rand::thread_rng(), &mut bag);

        assert_eq!(bag.len(), bag_len);
    }

    #[test]
    fn random_board_empty() {
        let mut board = Board::with_capacity(TILES_LEN);

        let board_len = random_board(&mut rand::thread_rng(), &mut board);

        assert!(board_len > 0);
        let board_len_isize = (board_len - 1) as isize;
        for (index, x) in (-board_len_isize..=board_len_isize)
            .step_by(2)
            .zip_eq(board.keys().map(|&(x, _)| x).sorted())
        {
            assert_eq!(index, x);
        }
    }

    #[test]
    fn random_points_zeros() {
        let mut points = Points::with_capacity(PLAYER_CAPACITY);

        for _ in 0..points.capacity() {
            points.push(0);
        }

        random_points(&mut rand::thread_rng(), &mut points);

        for point in points {
            assert!(point > 0);
        }
    }

    #[test]
    fn random_hands_empty() {
        let mut hands = Hands::with_capacity(PLAYER_CAPACITY);

        for _ in 0..hands.capacity() {
            hands.push(Hand::with_capacity(HAND_CAPACITY));
        }

        let hand_len = random_hands(&mut rand::thread_rng(), &mut hands);

        for hand in &hands {
            assert_eq!(hand_len, hand.len());
        }
    }

    #[test]
    fn random_different_shape_same_color_single_sample() {
        let mut rng = rand::thread_rng();
        let tile = rng.gen();

        let (same_color, different_shape) = random_different_shape_same_color(&mut rng, tile);

        let (color, shape) = tile;
        assert_eq!(color, same_color);
        assert_ne!(shape, different_shape);
    }

    #[test]
    fn random_different_color_same_shape_single_sample() {
        let mut rng = rand::thread_rng();
        let tile = rng.gen();

        let (different_color, same_shape) = random_different_color_same_shape(&mut rng, tile);

        let (color, shape) = tile;
        assert_ne!(color, different_color);
        assert_eq!(shape, same_shape);
    }

    #[test]
    fn random_current_player_zero_players() {
        let mut rng = rand::thread_rng();
        let mut current_player = rng.gen();

        random_current_player(&mut rng, &mut current_player, 0);
    }

    #[test]
    fn random_current_player_some_players() {
        let mut current_player = 0;

        let random_current_player = random_current_player(
            &mut rand::thread_rng(),
            &mut current_player,
            PLAYER_CAPACITY,
        );

        assert!((0..PLAYER_CAPACITY).contains(&random_current_player));
        assert_eq!(random_current_player, current_player);
    }

    #[test]
    fn random_legal_coordinates_all_legal() {
        for (x, y) in random_legal_coordinates(&mut rand::thread_rng()) {
            assert!(
                -COORDINATE_LIMIT < x
                    && x < COORDINATE_LIMIT
                    && -COORDINATE_LIMIT < y
                    && y < COORDINATE_LIMIT
            );
        }
    }

    #[test]
    fn random_legal_coordinates_all_illegal() {
        for (x, y) in random_illegal_coordinates(&mut rand::thread_rng()) {
            assert!(
                -COORDINATE_LIMIT >= x
                    || x >= COORDINATE_LIMIT
                    || -COORDINATE_LIMIT >= y
                    || y >= COORDINATE_LIMIT
            );
        }
    }
}
