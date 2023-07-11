use crate::state::{
    Exchanges, FirstPlayError, FirstView, Hand, LastView, NextExchangeError, NextPlayError,
    NextView, Plays, PlaysOrExchanges,
};
use async_trait::async_trait;
pub use first_runtime::*;
pub use last_runtime::*;
pub use next_runtime::*;
use std::collections::HashSet;

mod first_runtime;
mod last_runtime;
mod next_runtime;

/// Defines `(phase)_get` and `(phase)_update_(kind)` methods for each phase and error kind
/// of the game where appropriate. There are `first`, `next`, and `last` phases
/// and `view`, `play_errors`, and `exchange_errors` kinds.
///
/// `(phase)_get` methods block execution until getting input.
/// `(phase)_update_(play|exchange)_errors` methods block execution until updating output.
/// `(phase)_update_view` methods may execute in parallel with player updates.
///
/// # Errors
///
/// The implementor of [`Player`] is responsible for returning an error to prevent the runtime
/// from running indefinitely whether from no response or repeated invalid inputs. When a method
/// call fails, the runtime is stopped, and an error is returned and propagated out of the runtime
/// and back to the calling client code.
#[async_trait]
pub trait Player<E> {
    /// On the first turn, gets [`Plays`] from the current player.
    fn first_get(&self) -> Result<Plays, E>;

    /// When a call to [`FirstState::first_play`](crate::state::FirstState::first_play) fails,
    /// updates `current_player` with the state of the game, their hand, their play, and
    /// the reasons why their play could not be executed.
    fn first_update_play_errors<'a>(
        &self,
        first_view: &'a FirstView<'a>,
        hand: &'a Hand,
        plays: Plays,
        play_errors: HashSet<FirstPlayError>,
    ) -> Result<(), E>;

    /// During the first turn with game state [`FirstState`](crate::state::FirstState),
    /// updates each player with the state of the game and their hand.
    async fn first_update_view<'a>(
        &self,
        first_view: &'a FirstView<'a>,
        hand: &'a Hand,
    ) -> Result<(), E>;

    /// On the next turns, gets [`PlaysOrExchanges`] from the current player.
    fn next_get(&self) -> Result<PlaysOrExchanges, E>;

    /// When a call to [`NextState::next_play`](crate::state::NextState::next_play) fails,
    /// updates `current_player` with the state of the game, their hand, their play, and
    /// the reasons why their play could not be executed.
    fn next_update_play_errors<'a>(
        &self,
        next_view: &'a NextView<'a>,
        hand: &'a Hand,
        plays: Plays,
        play_errors: HashSet<NextPlayError>,
    ) -> Result<(), E>;

    /// When a call to [`NextState::next_exchange`](crate::state::NextState::next_exchange)
    /// fails, updates `current_player` with the state of the game, their hand, their play, and
    /// the reasons why their play could not be executed.
    fn next_update_exchange_errors<'a>(
        &self,
        next_view: &'a NextView<'a>,
        hand: &'a Hand,
        exchanges: Exchanges,
        exchange_errors: HashSet<NextExchangeError>,
    ) -> Result<(), E>;

    /// During the next turns with game state [`NextState`](crate::state::NextState),
    /// updates each player with the state of the game and their hand.
    async fn next_update_view<'a>(
        &self,
        next_view: &'a NextView<'a>,
        hand: &'a Hand,
    ) -> Result<(), E>;

    /// During the last turn with game state [`LastState`](crate::state::LastState),
    /// updates each player with the state of the game and their hand.
    async fn last_update_view<'a>(&self, last_view: &'a LastView<'a>) -> Result<(), E>;
}
