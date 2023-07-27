use crate::*;
use debt_token_interface::DebtTokenClient;
use price_feed_interface::PriceFeedClient;
use s_token_interface::STokenClient;
use soroban_sdk::{token::AdminClient as TokenAdminClient, token::Client as TokenClient};

extern crate std;

#[allow(dead_code)]
pub struct ReserveConfig<'a> {
    pub token: TokenClient<'a>,
    pub token_admin: TokenAdminClient<'a>,
    pub s_token: STokenClient<'a>,
    pub debt_token: DebtTokenClient<'a>,
}

#[allow(dead_code)]
pub struct Sut<'a> {
    pub pool: LendingPoolClient<'a>,
    pub price_feed: PriceFeedClient<'a>,
    pub pool_admin: Address,
    pub token_admin: Address,
    pub reserves: std::vec::Vec<ReserveConfig<'a>>,
}

impl<'a> Sut<'a> {
    pub fn token(&self) -> &TokenClient<'a> {
        &self.reserves[0].token
    }

    pub fn token_admin(&self) -> &TokenAdminClient<'a> {
        &self.reserves[0].token_admin
    }

    pub fn debt_token(&self) -> &DebtTokenClient<'a> {
        &self.reserves[0].debt_token
    }

    pub fn s_token(&self) -> &STokenClient<'a> {
        &self.reserves[0].s_token
    }
}
