#![no_std]

use soroban_sdk::{contractclient, contractspecfn, contracttype, Address, Bytes, Env, Vec};

pub struct Spec;

#[contracttype]
pub struct LoanAsset {
    pub asset: Address,
    pub amount: i128,
    pub premium: i128,
    pub borrow: bool,
}

#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "FlashLoanReceiverClient")]
pub trait FlashLoanReceiverTrait {
    fn receive(env: Env, initiator: Address, assets: Vec<LoanAsset>, params: Bytes) -> bool;
}
