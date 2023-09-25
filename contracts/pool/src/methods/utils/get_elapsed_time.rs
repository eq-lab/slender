use soroban_sdk::Env;

use crate::storage::read_reserve_timestamp_window;

/// Returns (current_time, elapsed_time)
pub fn get_elapsed_time(env: &Env, last_update_timestamp: u64) -> (u64, u64) {
    let current_time = env.ledger().timestamp();
    let window = read_reserve_timestamp_window(env);

    current_time
        .checked_sub(last_update_timestamp)
        .and_then(|el| el.checked_rem(window))
        .and_then(|rem| current_time.checked_sub(rem))
        .and_then(|cur_t| cur_t.checked_sub(last_update_timestamp))
        .map_or((current_time, 0), |el| (current_time, el))
}
