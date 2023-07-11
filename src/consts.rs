use crate::{Color, Shape, TILES_LEN};
use konst::primitive::{parse_isize, parse_usize};
use konst::{option, result};

/// The amount of extra points given for each line completed with either
/// [every shape](Shape::shapes) or [every color](Color::colors). `6` additional points.
///
/// # See Also
///
/// * [check_line](crate::check_line)
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
pub const FULL_MATCH_BONUS: usize = 6;
/// The amount of extra points given when there are no available [tiles](crate::Tile) and a player
/// [plays](crate::Plays) their last [tile](crate::Tile). `6` additional points.
///
/// # See Also
///
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
pub const LAST_PLAY_BONUS: usize = 6;
/// All small, dynamically allocated structs which store player data will be stored on the stack
/// until the number of players becomes greater than `PLAYER_CAPACITY`. When there are more than
/// `PLAYER_CAPACITY` players, player data will be heap allocated. If the environment variable
/// named `PLAYER_CAPACITY` is present at compile time and is able to be parsed into a `usize`,
/// set to the value of the environment variable. Otherwise, it is set to `4`.
///
/// # See Also
///
/// * [FirstState](crate::FirstState)
/// * [NextState](crate::NextState)
/// * [LastState](crate::LastState)
pub const PLAYER_CAPACITY: usize = option::unwrap_or!(
    option::and_then!(option_env!("PLAYER_CAPACITY"), |str| result::ok!(
        parse_usize(str)
    )),
    4
);
/// All small, dynamically allocated properties which store hand data will be stored
/// on the stack until the number of [tiles](crate::Tile) in a hand becomes greater
/// than `HAND_CAPACITY`. When there are more than `HAND_CAPACITY` [tiles](crate::Tile)
/// in a hand, its data will be heap allocated. If the environment variable named
/// `HAND_CAPACITY` is present at compile time and is able to be parsed into a `usize`,
/// set to the value of the environment variable. Otherwise, it is set to the maximum of
/// the [number of colors](Color::COLORS_LEN) and the [number of shapes](Shape::SHAPES_LEN).
///
/// # See Also
///
/// * [Color::COLORS_LEN]
/// * [Shape::SHAPES_LEN]
pub const HAND_CAPACITY: usize = option::unwrap_or!(
    option::and_then!(option_env!("HAND_CAPACITY"), |str| result::ok!(
        parse_usize(str)
    )),
    if Color::COLORS_LEN >= Shape::SHAPES_LEN {
        Color::COLORS_LEN
    } else {
        Shape::SHAPES_LEN
    }
);
/// The maximum number of [tiles](crate::Tile) allowed in the bag. If
/// the environment variable named `TILE_LIMIT` is present at compile time, is able to be parsed
/// into a `usize`, and is greater than or equal to the [number of tile variants](TILES_LEN),
/// set to the value of the environment variable. Otherwise, it is set to `10_000`.
///
/// It is important to not allocated too many [tiles](crate::Tile) where the game could run
/// for longer than players would be realistically willing to play and consume
/// too much time and memory.
///
/// # Panics
///
/// * When the given value is less than the [number of tile variants](TILES_LEN)
/// * When ([isize::MAX] / [COORDINATE_LIMIT] >= [TILE_LIMIT] as `isize`) is true so that overflow
/// is prevented
///
/// # See Also
///
/// * [PLAYER_CAPACITY]
/// * [HAND_CAPACITY]
/// * [FirstState::new](crate::FirstState::new)
/// * [FirstState::new_random_first_player_selector](crate::FirstState::new_random_first_player)
pub const TILE_LIMIT: usize = option::unwrap_or!(
    option::and_then!(option_env!("TILE_LIMIT"), |str| result::ok!(parse_usize(
        str
    ))),
    10_000
);
const _: () = assert!(TILE_LIMIT >= TILES_LEN);
/// The exclusive maximum absolute value of a component in a [coordinate](crate::Coordinate).
/// If the environment variable named `COORDINATE_LIMIT` is present at compile time,
/// is able to be parsed into a `isize`, is not `0` and would not cause an overflow,
/// set to the saturating absolute value of the environment variable.
/// Otherwise, it is set to `10_000`.
///
/// It is recommended to set the value of `COORDINATE_LIMIT` greater than [TILES_LEN] *
/// the [expected maximum unique tile copied count](crate::FirstState::new) to prevent the game
/// from reaching the edge of the board.
///
/// # Panics
///
/// * When the given value is `0`, [isize::MIN], [isize::MIN] `+ 1` or [isize::MAX]
/// * When ([isize::MAX] / [COORDINATE_LIMIT] >= [TILE_LIMIT] as `isize`) is true so that overflow
/// is prevented
///
/// # See Also
///
/// * [Coordinate](crate::Coordinate)
/// * [FirstState::first_play](crate::FirstState::first_play)
/// * [NextState::next_play](crate::NextState::next_play)
pub const COORDINATE_LIMIT: isize = option::unwrap_or!(
    option::and_then!(option_env!("COORDINATE_LIMIT"), |str| result::ok!(
        parse_isize(str)
    )),
    10_000
)
.saturating_abs();
const _: () = assert!(COORDINATE_LIMIT > 0);
// cannot use assert_ne! in a const context
//noinspection RsAssertEqual
const _: () = assert!(COORDINATE_LIMIT != isize::MAX);
const _: () = assert!(isize::MAX / COORDINATE_LIMIT >= TILE_LIMIT as isize);
/// The maximum number of holes that can be returned in an error. If the environment variable
/// named `HOLES_LIMIT` is present at compile time and is able to be parsed into a `usize`,
/// set to the value of the environment variable. Otherwise, it is set to `100`.
///
/// Setting `HOLES_LIMIT` to `0` stops any searching for holes.
///
/// It is important to set both `HOLES_LIMIT` and [COORDINATE_LIMIT] to a small value
/// to limit time and memory cost. When both values are large, it is possible for some
/// [play](crate::Plays) to cause a large set to be allocated. For example, placing two tiles near
/// [isize::MIN] and [isize::MAX] would cause a set to be allocated for nearly [usize::MAX] items
/// which obviously would grind execution to a halt.
///
/// # See Also
///
/// * [FirstPlayError::Holes](crate::FirstPlayError::Holes)
/// * [NextPlayError::Holes](crate::NextPlayError::Holes)
/// * [COORDINATE_LIMIT]
pub const HOLES_LIMIT: usize = option::unwrap_or!(
    option::and_then!(option_env!("HOLES_LIMIT"), |str| result::ok!(parse_usize(
        str
    ))),
    100
);
