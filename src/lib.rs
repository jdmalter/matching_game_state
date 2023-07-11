//! Concrete structs to represent and protect the state of the game with methods to progress
//! through the phases of the game.
//!
//! ## Summary
//!
//! Implemented through `3` different phases of the game: `First`, `Next`, and `Last`. The game
//! starts before and opens with the `First` phase, advances through one or more turns during
//! the `Next` phase, and ends at the `Last` phase. Each player takes turns to advance the game by
//! either [playing](NextState::next_play) or [exchanging](NextState::next_exchange) [tiles](Tile).
//! Players earn points by playing unique [tiles](Tile) which match each other by either
//! [shape](Shape) or [color](Color). Each phase of the game offers a different view
//! with some publicly visible and some privately visible properties for each player.
//! The game ends when some player runs out of [tiles](Tile) [to play](NextState::next_play)
//! or the board cannot be legally [played on](NextState::next_play).
//! The player with the most points wins.
//!
//! ## What are the phases of the game?
//!
//! * `First`: The first turn of the game where the board is empty and points
//! only contain `0`s. The maximum number of [tiles](Tile) that each player can
//! [play](FirstState::first_play) is visible to all players. Some player with the
//! maximum of those maximums becomes the current player.
//! Represented by [FirstState](FirstState).
//! * `Next`: The turns after the first turn and before the last turn where the board
//! is not empty and points have been earned. Each player takes turns being the current player.
//! Represented by [NextState](NextState).
//! * `Last`: The first turn where the game has ended. Represented by
//! [LastState](LastState).
//!
//! ## How is the game created?
//!
//! [FirstState](FirstState) offers the only public endpoint to create the game state.
//! [FirstState::new] and [FirstState::new_random_first_player] create
//! the game before the `First` phase.
//!
//! ## How is the game advanced?
//!
//! * `First`: The current player [plays](FirstState::first_play) first to advance the game
//! to the `Next` phase.
//! * `Next`: The current player either [plays](NextState::next_play) or
//! [exchanges](NextState::next_exchange) [tiles](Tile)
//! to advance the game to either the `Next` or `Last` phase (not respectively).
//! * `Last`: It is not possible to advance the game once the game has ended.
//!
//! The current player is represented as the index of the player whose turn it is in the range
//! `0`..(the number of players) which either increments or loops back to `0` when necessary.
//!
//! ### How are tiles played?
//!
//! Remove some [tiles](Tile) from the current player's hand, insert those [tiles](Tile)
//! into the board at their respective [coordinates](Coordinate) from [plays](Plays),
//! attempt to fill the current player's hand up to its previous length,
//! add the points earned by the [play](Plays) to the current player, and advance to
//! the next player if the game has not ended.
//!
//! Implemented by [FirstState::first_play](FirstState::first_play) and
//! [NextState::next_play](NextState::next_play) for the `First` and `Next` phases of
//! the game respectively.
//!
//! ### How are tiles exchanged?
//!
//! [Exchange](NextState::next_exchange) [tiles](Tile) from the current player's hand with
//! [tiles](Tile) from the bag, ignore points, and advance to the next player.
//!
//! ## How are points calculated?
//!
//! The number of points earned by a [play](Plays) is the sum of points scored from each line that
//! contains played [tiles](Tile). Each [tile](Tile) can be counted twice if the [tile](Tile)
//! is a part of a vertical and horizontal line.
//!
//! The number of points from a line is the number of [tiles](Tile) in that line. If the line
//! creates a full match on the board where a line contains either
//! [every color](Color::colors) or [every shape](Shape::shapes), an extra
//! [full match bonus](FULL_MATCH_BONUS) is earned.
//!
//! If the current player's hand is empty (and therefore no [tiles](Tile) in the bag
//! are available) or the board becomes deadlocked (a filled rectangle of
//! [every color](Color::colors) and [every shape](Shape::shapes)) where
//! no additional [plays](Plays) are allowed despite players still holding some [tiles](Tile),
//! an extra [last play bonus](LAST_PLAY_BONUS) is earned.
//!
//! ## How is the game viewed?
//!
//! To obtain an immutable representation of the current state of the game visible to all players,
//! call [FirstState::first_view](FirstState::first_view),
//! [NextState::next_view](NextState::next_view), or [LastState::last_view](LastState::last_view)
//! for the `First`, `Next`, or `Last` phase of the game respectively.
//!
//! [FirstState::get_hand](FirstState::get_hand) and
//! [NextState::get_hand](NextState::get_hand) share private information for each
//! individual player.
//!
//! ## How is the game ended?
//!
//! The game ends when either the current player's hand is empty or the board is
//! a filled rectangle of [every color](Color::colors) and [every shape](Shape::shapes).
//! It is impossible [to legally play](NextState::next_play) on the board since either
//! the current player has no [tiles](Tile) [to play](NextState::next_play) or all [plays](Plays)
//! will contain duplicate [tiles](Tile) in a line on the board.
//!
//! ## How are game states tested when properties are private?
//!
//! The `test` build configuration adds many required methods for testing. Each state struct
//! implements methods to get mutable references to their properties, helper methods to add
//! random data to specific properties, and methods to set properties for common scenarios.

// Document!
#![forbid(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    missing_docs,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_html_tags,
    rustdoc::bare_urls
)]
// Don't leave a build in a half finished state!
#![deny(
    warnings,
    future_incompatible,
    nonstandard_style,
    rust_2018_compatibility,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused,
    single_use_lifetimes,
    unreachable_pub,
    missing_debug_implementations,
    unsafe_code
)]

pub use consts::*;
pub use coordinate::*;
pub use first_state::*;
pub use last_state::*;
pub use next_state::*;
pub use play::*;
#[cfg(test)]
pub use random::*;
pub use tile::*;
pub use types::*;

mod consts;
mod coordinate;
mod first_state;
mod last_state;
mod next_state;
mod play;
#[cfg(test)]
mod random;
mod tile;
mod types;
