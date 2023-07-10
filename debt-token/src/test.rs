#![cfg(test)]
extern crate std;

use crate::DebtToken;
use debt_token_interface::DebtTokenClient;
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, IntoVal, Symbol};

fn create_token<'a>(e: &Env) -> (DebtTokenClient<'a>, Address) {
    let pool = Address::random(e);

    let token = DebtTokenClient::new(e, &e.register_contract(None, DebtToken {}));

    let underlying_asset = Address::random(&e);

    token.initialize(
        &7,
        &"name".into_val(e),
        &"symbol".into_val(e),
        &pool,
        &underlying_asset,
    );

    (token, pool)
}

#[test]
fn initialize() {
    let e = Env::default();
    e.mock_all_auths();
    let pool = Address::random(&e);

    let token = DebtTokenClient::new(&e, &e.register_contract(None, DebtToken {}));

    let underlying_asset = Address::random(&e);

    token.initialize(
        &7,
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &underlying_asset,
    );

    assert_eq!(token.decimals(), 7);
    assert_eq!(token.name(), Bytes::from_slice(&e, b"name"));
    assert_eq!(token.symbol(), Bytes::from_slice(&e, b"symbol"));
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();

    let (token, pool) = create_token(&e);

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let user1_amount = 1000;
    let total_supply = user1_amount;

    token.mint(&user1, &user1_amount);
    assert_eq!(
        e.auths(),
        [(
            pool.clone(),
            token.address.clone(),
            Symbol::short("mint"),
            (&user1, user1_amount).into_val(&e),
        )]
    );
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.set_authorized(&user2, &false);
    assert_eq!(
        e.auths(),
        [(
            pool.clone(),
            token.address.clone(),
            Symbol::new(&e, "set_authorized"),
            (&user2, false).into_val(&e),
        )]
    );
    assert_eq!(token.authorized(&user2), false);

    token.set_authorized(&user2, &true);
    assert_eq!(token.authorized(&user2), true);

    let clawback = 100;
    token.clawback(&user1, &clawback);
    assert_eq!(
        e.auths(),
        [(
            pool.clone(),
            token.address.clone(),
            Symbol::short("clawback"),
            (&user1, clawback).into_val(&e),
        )]
    );
    assert_eq!(token.balance(&user1), user1_amount - clawback);
    assert_eq!(token.total_supply(), total_supply - clawback);
}

#[test]
#[should_panic(expected = "not implemented")]
fn test_burn() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.burn_from(&user2, &user1, &500);
}

#[test]
#[should_panic(expected = "already initialized")]
fn initialize_already_initialized() {
    let e = Env::default();
    let (token, _pool) = create_token(&e);

    let pool = Address::random(&e);
    let underlying_asset = Address::random(&e);

    token.initialize(
        &10,
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &underlying_asset,
    );
}

#[test]
#[should_panic(expected = "decimal must fit in a u8")]
fn decimal_is_over_max() {
    let e = Env::default();
    let token = DebtTokenClient::new(&e, &e.register_contract(None, DebtToken {}));

    let pool = Address::random(&e);
    let underlying_asset = Address::random(&e);

    token.initialize(
        &(u32::from(u8::MAX) + 1),
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &underlying_asset,
    );
}
