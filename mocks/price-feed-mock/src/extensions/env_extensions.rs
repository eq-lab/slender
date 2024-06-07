#![allow(non_upper_case_globals)]
use price_feed_interface::types::asset::Asset;
use soroban_sdk::storage::{Instance, Temporary};
use soroban_sdk::Env;

// use crate::extensions;

// use extensions::u128_helper::U128Helper;
// const ADMIN_KEY: &str = "admin";
const LAST_TIMESTAMP: &str = "last_timestamp";
// const RETENTION_PERIOD: &str = "period";
// const ASSETS: &str = "assets";
// const BASE_ASSET: &str = "base_asset";
// const DECIMALS: &str = "decimals";
const RESOLUTION: &str = "resolution";

pub trait EnvExtensions {
    fn get_asset_index(&self, asset: &Asset) -> Option<u32>;
    fn get_price(&self, asset: u32) -> Option<i128>;
    fn get_resolution(&self) -> u32;
    fn get_last_timestamp(&self) -> u64;
}

impl EnvExtensions for Env {
    fn get_asset_index(&self, asset: &Asset) -> Option<u32> {
        match asset {
            Asset::Stellar(address) => get_instance_storage(self).get(&address),
            Asset::Other(symbol) => get_instance_storage(self).get(&symbol),
        }
    }

    fn get_price(&self, asset_index: u32) -> Option<i128> {
        //build the key for the price
        // let data_key = U128Helper::encode_price_record_key(timestamp, asset);
        let data_key = asset_index;
        //get the price
        get_temporary_storage(self).get(&data_key)
    }

    fn get_resolution(&self) -> u32 {
        get_instance_storage(self).get(&RESOLUTION).unwrap()
    }

    fn get_last_timestamp(&self) -> u64 {
        //get the marker
        get_instance_storage(self)
            .get(&LAST_TIMESTAMP)
            .unwrap_or_default()
    }
}

fn get_instance_storage(e: &Env) -> Instance {
    e.storage().instance()
}

fn get_temporary_storage(e: &Env) -> Temporary {
    e.storage().temporary()
}
