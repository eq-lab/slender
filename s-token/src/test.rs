#![cfg(test)]
extern crate std;

use crate::SToken;
use s_token_interface::STokenClient;
use soroban_sdk::{testutils::Address as _, Address, Env, IntoVal, Symbol};

mod pool {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/pool.wasm");
}

fn create_token<'a>(e: &Env) -> (STokenClient<'a>, pool::Client<'a>) {
    let pool = pool::Client::new(e, &e.register_contract_wasm(None, pool::WASM));
    let pool_admin = Address::random(e);
    pool.initialize(&pool_admin);

    let token = STokenClient::new(e, &e.register_contract(None, SToken {}));

    let treasury = Address::random(&e);
    let underlying_asset = Address::random(&e);

    token.initialize(
        &7,
        &"name".into_val(e),
        &"symbol".into_val(e),
        &pool.address,
        &treasury,
        &underlying_asset,
    );

    (token, pool)
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();

    let (token, pool) = create_token(&e);

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let user3 = Address::random(&e);

    token.mint(&user1, &1000);
    assert_eq!(
        e.auths(),
        [(
            pool.address.clone(),
            token.address.clone(),
            Symbol::short("mint"),
            (&user1, 1000_i128).into_val(&e),
        )]
    );
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.increase_allowance(&user2, &user3, &500);
    assert_eq!(
        e.auths(),
        [(
            user2.clone(),
            token.address.clone(),
            Symbol::new(&e, "increase_allowance"),
            (&user2, &user3, 500_i128).into_val(&e),
        )]
    );
    assert_eq!(token.allowance(&user2, &user3), 500);

    token.transfer(&user1, &user2, &600);
    assert_eq!(
        e.auths(),
        [(
            user1.clone(),
            token.address.clone(),
            Symbol::short("transfer"),
            (&user1, &user2, 600_i128).into_val(&e),
        )]
    );
    assert_eq!(token.balance(&user1), 400);
    assert_eq!(token.balance(&user2), 600);

    token.transfer_from(&user3, &user2, &user1, &400);
    assert_eq!(
        e.auths(),
        [(
            user3.clone(),
            token.address.clone(),
            Symbol::new(&e, "transfer_from"),
            (&user3, &user2, &user1, 400_i128).into_val(&e),
        )]
    );
    assert_eq!(token.balance(&user1), 800);
    assert_eq!(token.balance(&user2), 200);

    token.transfer(&user1, &user3, &300);
    assert_eq!(token.balance(&user1), 500);
    assert_eq!(token.balance(&user3), 300);

    token.set_authorized(&user2, &false);
    assert_eq!(
        e.auths(),
        [(
            pool.address.clone(),
            token.address.clone(),
            Symbol::new(&e, "set_authorized"),
            (&user2, false).into_val(&e),
        )]
    );
    assert_eq!(token.authorized(&user2), false);

    token.set_authorized(&user3, &true);
    assert_eq!(token.authorized(&user3), true);

    token.clawback(&user3, &100);
    assert_eq!(
        e.auths(),
        [(
            pool.address.clone(),
            token.address.clone(),
            Symbol::short("clawback"),
            (&user3, 100_i128).into_val(&e),
        )]
    );
    assert_eq!(token.balance(&user3), 200);
    assert_eq!(token.total_supply(), 900);

    // Increase by 400, with an existing 100 = 500
    token.increase_allowance(&user2, &user3, &400);
    assert_eq!(token.allowance(&user2, &user3), 500);
    token.decrease_allowance(&user2, &user3, &501);
    assert_eq!(
        e.auths(),
        [(
            user2.clone(),
            token.address.clone(),
            Symbol::new(&e, "decrease_allowance"),
            (&user2, &user3, 501_i128).into_val(&e),
        )]
    );
    assert_eq!(token.allowance(&user2, &user3), 0);
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

    token.increase_allowance(&user1, &user2, &500);
    assert_eq!(token.allowance(&user1, &user2), 500);

    token.burn_from(&user2, &user1, &500);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_insufficient_balance() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.transfer(&user1, &user2, &1001);
}

#[test]
#[should_panic(expected = "can't receive when deauthorized")]
fn transfer_receive_deauthorized() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.set_authorized(&user2, &false);
    token.transfer(&user1, &user2, &1);
}

#[test]
#[should_panic(expected = "can't spend when deauthorized")]
fn transfer_spend_deauthorized() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.set_authorized(&user1, &false);
    token.transfer(&user1, &user2, &1);
}

#[test]
#[should_panic(expected = "insufficient allowance")]
fn transfer_from_insufficient_allowance() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let user3 = Address::random(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.increase_allowance(&user1, &user3, &100);
    assert_eq!(token.allowance(&user1, &user3), 100);

    token.transfer_from(&user3, &user1, &user2, &101);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn initialize_already_initialized() {
    let e = Env::default();
    let (token, _pool) = create_token(&e);

    let pool = Address::random(&e);
    let treasury = Address::random(&e);
    let underlying_asset = Address::random(&e);

    token.initialize(
        &10,
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &treasury,
        &underlying_asset,
    );
}

#[test]
#[should_panic(expected = "Decimal must fit in a u8")]
fn decimal_is_over_max() {
    let e = Env::default();
    let token = STokenClient::new(&e, &e.register_contract(None, SToken {}));

    let pool = Address::random(&e);
    let treasury = Address::random(&e);
    let underlying_asset = Address::random(&e);

    token.initialize(
        &(u32::from(u8::MAX) + 1),
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &treasury,
        &underlying_asset,
    );
}
