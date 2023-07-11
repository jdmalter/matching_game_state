use crate::runtime::Player;
use crate::state::{FirstState, NextState};
use futures::future;
use itertools::Itertools;

/// It repeatedly asks `current_player` for a play, and if the play is
/// invalid, it tells the player about the error and asks again. If the play is valid,
/// stops asking for a play and produces the next state of the game.
///
/// Calls [`Player::next_get`] for an input. Attempts
/// [`FirstState::first_play`](FirstState::first_play),
/// and if the input is invalid, calls [`Player::first_update_play_errors`].
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
pub fn first_process_input<P, E>(
    players: &Vec<P>,
    mut first_state: FirstState,
) -> Result<NextState, E>
where
    P: Player<E>,
{
    let current_player = first_state.current_player();
    let player = &players[current_player];

    loop {
        let plays = player.first_get()?;
        match first_state.first_play(&plays) {
            Ok(next_state) => return Ok(next_state),
            Err((same_first_state, play_errors)) => {
                first_state = same_first_state;
                // cannot use map_err since E needs to be propagated here
                player.first_update_play_errors(
                    &first_state.first_view(),
                    first_state.get_hand(current_player).unwrap(),
                    plays,
                    play_errors,
                )?;
            }
        }
    }
}

/// Asynchronously sends the current state of the game to [`Player`]s.
///
/// # Arguments
///
/// * `players`: A vector of [`Player`]s.
/// * `first_state`: The current state of the game.
///
/// # Errors
///
/// Accumulates all errors from [`Player::first_update_view`] into a vector.
///
/// # Returns
///
/// An empty tuple if there are no errors; otherwise, a vector of errors.
pub async fn first_send_updates<P, E>(
    players: &Vec<P>,
    first_state: &FirstState,
) -> Result<(), Vec<E>>
where
    P: Player<E>,
{
    let first_view = first_state.first_view();
    let update_tasks = (0..players.len()).map(|player| {
        players[player].first_update_view(&first_view, first_state.get_hand(player).unwrap())
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
