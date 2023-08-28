#![deny(warnings)]
#![no_std]

use flash_loan_receiver_interface::{Asset, FlashLoanReceiverTrait};
use soroban_sdk::{
    contract, contractclient, contractimpl, contractspecfn, token, xdr::FromXdr, Address, Bytes,
    Env, Vec,
};

pub struct Spec;

#[contract]
pub struct FlashLoanReceiver;

#[contractimpl]
impl FlashLoanReceiverTrait for FlashLoanReceiver {
    fn receive(env: Env, assets: Vec<Asset>, params: Bytes) -> bool {
        let pool = Address::from_xdr(&env, &params);

        for asset in assets {
            let token_client = token::Client::new(&env, &asset.asset);
            token_client.transfer(owner, to, amount)
        }

        true
    }
}

#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "FlashLoanReceiverAdminClient")]
pub trait FlashLoanReceiverAdminTrait {
    fn initialize(env: Env, owner: Address, pool: Address);
}
