use soroban_sdk::Env;

/// Returns (current_time, elapsed_time)
pub fn get_elapsed_time(
    env: &Env,
    last_update_timestamp: u64,
    reserve_timestamp_window: u64,
) -> (u64, u64) {
    let current_time = env.ledger().timestamp();

    current_time
        .checked_sub(last_update_timestamp)
        .and_then(|el| el.checked_rem(reserve_timestamp_window))
        .and_then(|rem| current_time.checked_sub(rem))
        .and_then(|cur_t| cur_t.checked_sub(last_update_timestamp))
        .map_or((current_time, 0), |el| (current_time, el))
}
