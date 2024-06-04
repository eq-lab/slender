use debt_token_interface::DebtTokenClient;
use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::{
    get_fungible_lp_tokens::get_fungible_lp_tokens, validation::require_permission,
};

pub fn upgrade_debt_token(
    env: &Env,
    who: &Address,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
) -> Result<(), Error> {
    require_permission(&env, who, &Permission::UpgradeLPTokens).unwrap();

    let reserve = read_reserve(env, asset)?;
    let (_, debt_token_address) = get_fungible_lp_tokens(&reserve)?;
    let debt_token = DebtTokenClient::new(env, debt_token_address);
    debt_token.upgrade(new_wasm_hash);

    Ok(())
}
