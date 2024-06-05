use pool_interface::types::{error::Error, permission::Permission, reserve_type::ReserveType};
use s_token_interface::STokenClient;
use soroban_sdk::{token, Address, Env};

use crate::{read_protocol_fee_vault, read_reserve, write_protocol_fee_vault};

use super::utils::validation::require_permission;

pub fn claim_protocol_fee(
    env: &Env,
    who: &Address,
    asset: &Address,
    recipient: &Address,
) -> Result<(), Error> {
    require_permission(env, who, &Permission::ClaimProtocolFee)?;
    let reserve_data = read_reserve(env, asset)?;
    let amount = &read_protocol_fee_vault(env, asset);
    match reserve_data.reserve_type {
        ReserveType::Fungible(s_token, _) => {
            STokenClient::new(env, &s_token).transfer_underlying_to(recipient, amount);
        }
        ReserveType::RWA => token::Client::new(env, asset).transfer(
            &env.current_contract_address(),
            recipient,
            amount,
        ),
    }

    write_protocol_fee_vault(env, asset, 0);

    Ok(())
}
