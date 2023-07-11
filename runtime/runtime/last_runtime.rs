use crate::runtime::Player;
use crate::state::LastState;
use futures::future;
use itertools::Itertools;

/// Asynchronously sends the current state of the game to [`Player`](crate::runtime)s.
///
/// # Arguments
///
/// * `players`: A vector of [`Player`](crate::runtime)s.
/// * `last_state`: The current state of the game.
///
/// # Errors
///
/// Accumulates all errors from [`Player::last_update_view`] into a vector.
///
/// # Returns
///
/// An empty tuple if there are no errors; otherwise, a vector of errors.
pub async fn last_send_updates<P, E>(players: &Vec<P>, last_state: &LastState) -> Result<(), Vec<E>>
where
    P: Player<E>,
{
    let last_view = last_state.last_view();
    let update_tasks =
        (0..players.len()).map(|player| players[player].last_update_view(&last_view));

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
