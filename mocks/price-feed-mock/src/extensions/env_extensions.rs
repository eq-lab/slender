#![allow(non_upper_case_globals)]
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::storage::{Instance, Temporary};
use soroban_sdk::{contracttype, Env};

use super::u128_helper::U128Helper;

// use crate::extensions;

// use extensions::u128_helper::U128Helper;
// const ADMIN_KEY: &str = "admin";
pub const LAST_TIMESTAMP: &str = "last_timestamp";
// const RETENTION_PERIOD: &str = "period";
// const ASSETS: &str = "assets";
// const BASE_ASSET: &str = "base_asset";
// const DECIMALS: &str = "decimals";
#[contracttype]
pub struct PricesLength(u32);

pub trait EnvExtensions {
    fn get_asset_index(&self, asset: &Asset) -> Option<u32>;
    fn get_price(&self, asset: u32, timestamp: u64) -> Option<PriceData>;
    fn get_prices_length(&self, asset_index: u32) -> u32;
    fn get_last_timestamp(&self) -> u64;
    fn set_prices_length(&self, asset_index: u32, prices_length: u32);
}

impl EnvExtensions for Env {
    fn get_asset_index(&self, asset: &Asset) -> Option<u32> {
        match asset {
            Asset::Stellar(address) => get_instance_storage(self).get(&address),
            Asset::Other(symbol) => get_instance_storage(self).get(&symbol),
        }
    }

    fn get_price(&self, asset_index: u32, i: u64) -> Option<PriceData> {
        //build the key for the price
        let data_key = U128Helper::encode_price_record_key(i, asset_index);
        //get the price
        get_temporary_storage(self).get(&data_key)
    }

    fn get_prices_length(&self, asset_index: u32) -> u32 {
        get_instance_storage(self)
            .get(&PricesLength(asset_index))
            .unwrap()
    }

    fn get_last_timestamp(&self) -> u64 {
        //get the marker
        get_instance_storage(self)
            .get(&LAST_TIMESTAMP)
            .unwrap_or_default()
    }

    fn set_prices_length(&self, asset_index: u32, prices_length: u32) {
        get_instance_storage(self).set(&PricesLength(asset_index), &prices_length)
    }
}

fn get_instance_storage(e: &Env) -> Instance {
    e.storage().instance()
}

fn get_temporary_storage(e: &Env) -> Temporary {
    e.storage().temporary()
}
