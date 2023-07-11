use crate::runtime::Player;
use crate::state::{NextOrLastState, NextState, PlaysOrExchanges};
use futures::future;
use itertools::Itertools;

/// It repeatedly asks `current_player` for a play or exchange, and if the play or exchange is
/// invalid, it tells the player about the error and asks again. If the play or exchange is valid,
/// stops asking for a play or exchange and produces the next state of the game.
///
/// Calls [`Player::next_get`] for an input. Attempts either
/// [`NextState::next_play`](NextState::next_play)
/// or [`NextState::next_exchange`](NextState::next_exchange)
/// depending on the input, and if the input is invalid, calls
/// [`Player::next_update_play_errors`] or [`Player::next_update_exchange_errors`]
/// respectively.
///
/// # Arguments
///
/// * `players`: A vector of players.
/// * `next_state`: The current state of the game.
///
/// # Errors
///
/// When `current_player` fails to send input or receive an error update.
///
/// # Returns
///
/// The next state of the game
pub fn next_process_input<P, E>(
    players: &Vec<P>,
    mut next_state: NextState,
) -> Result<NextOrLastState, E>
where
    P: Player<E>,
{
    let current_player = next_state.current_player();
    let player = &players[current_player];

    loop {
        let plays_or_exchanges = player.next_get()?;
        match plays_or_exchanges {
            PlaysOrExchanges::Play(plays) => match next_state.next_play(&plays) {
                Ok(next_or_last_state) => return Ok(next_or_last_state),
                Err((same_next_state, play_errors)) => {
                    next_state = same_next_state;
                    // cannot use map_err since E needs to be propagated here
                    player.next_update_play_errors(
                        &next_state.next_view(),
                        next_state.get_hand(current_player).unwrap(),
                        plays,
                        play_errors,
                    )?;
                }
            },
            PlaysOrExchanges::Exchange(exchanges) => match next_state.next_exchange(&exchanges) {
                Ok(()) => return Ok(NextOrLastState::Next(next_state)),
                Err(exchange_errors) => {
                    // cannot use map_err since E needs to be propagated here
                    player.next_update_exchange_errors(
                        &next_state.next_view(),
                        next_state.get_hand(current_player).unwrap(),
                        exchanges,
                        exchange_errors,
                    )?;
                }
            },
        }
    }
}

/// Asynchronously sends the current state of the game to [`Player`]s.
///
/// # Arguments
///
/// * `players`: A vector of [`Player`]s.
/// * `next_state`: The current state of the game.
///
/// # Errors
///
/// Accumulates all errors from [`Player::next_update_view`] into a vector.
pub async fn next_send_updates<P, E>(players: &Vec<P>, next_state: &NextState) -> Result<(), Vec<E>>
where
    P: Player<E>,
{
    let next_view = next_state.next_view();
    let update_tasks = (0..players.len()).map(|player| {
        players[player].next_update_view(&next_view, next_state.get_hand(player).unwrap())
    });

    let errors = future::join_all(update_tasks)
        .await
        .into_iter()
        .filter_map(Result::err)
        .collect_vec();
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(())
}
