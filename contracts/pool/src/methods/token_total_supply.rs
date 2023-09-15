use soroban_sdk::{Address, Env};

use crate::storage::read_token_total_supply;

pub fn token_total_supply(env: &Env, token: &Address) -> i128 {
    read_token_total_supply(&env, &token)
}
