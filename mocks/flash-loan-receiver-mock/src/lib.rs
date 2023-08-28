#![deny(warnings)]
#![no_std]

use flash_loan_receiver_interface::{Asset, FlashLoanReceiverTrait};
use soroban_sdk::{contract, contractclient, contractimpl, token, Address, Bytes, Env, Vec};
use storage::{
    read_owner, read_pool, read_should_fail, write_owner, write_pool, write_should_fail,
};

mod storage;

#[contractclient(name = "FlashLoanReceiverAdminClient")]
pub trait FlashLoanReceiverAdminTrait {
    fn initialize(env: Env, owner: Address, pool: Address, should_fail: bool);
}

#[contract]
pub struct FlashLoanReceiver;

#[contractimpl]
impl FlashLoanReceiverTrait for FlashLoanReceiver {
    fn receive(env: Env, assets: Vec<Asset>, _params: Bytes) -> bool {
        if read_should_fail(&env) {
            return false;
        }

        let pool = read_pool(&env);
        let owner = read_owner(&env);
        let ledger = env.ledger().sequence() + 1;

        for asset in assets {
            let token_client = token::Client::new(&env, &asset.asset);

            token_client.transfer(&owner, &env.current_contract_address(), &asset.premium);

            token_client.approve(
                &env.current_contract_address(),
                &pool,
                &(asset.amount + asset.premium),
                &ledger,
            );
        }

        true
    }
}

#[contractimpl]
impl FlashLoanReceiverAdminTrait for FlashLoanReceiver {
    fn initialize(env: Env, owner: Address, pool: Address, should_fail: bool) {
        write_owner(&env, &owner);
        write_pool(&env, &pool);
        write_should_fail(&env, should_fail);
    }
}
