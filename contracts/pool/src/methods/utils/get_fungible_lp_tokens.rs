use pool_interface::types::{error::Error, reserve_data::ReserveData, reserve_type::ReserveType};
use soroban_sdk::Address;

pub fn get_fungible_lp_tokens(reserve: &ReserveData) -> Result<(&Address, &Address), Error> {
    if let ReserveType::Fungible(s_token_address, debt_token_address) = &reserve.reserve_type {
        Ok((s_token_address, debt_token_address))
    } else {
        Err(Error::NotFungible)
    }
}
