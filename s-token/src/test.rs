#![cfg(test)]
extern crate std;

use crate::SToken;
use s_token_interface::STokenClient;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal, String, Symbol,
};

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
        &String::from_slice(e, &"name"),
        &String::from_slice(e, &"symbol"),
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
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    symbol_short!("mint"),
                    (&user1, 1000_i128).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.approve(&user2, &user3, &500, &0);
    assert_eq!(
        e.auths(),
        [(
            user2.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    Symbol::new(&e, "approve"),
                    (&user2, &user3, 500_i128, 0u32).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(token.allowance(&user2, &user3), 500);

    token.transfer(&user1, &user2, &600);
    assert_eq!(
        e.auths(),
        [(
            user1.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    symbol_short!("transfer"),
                    (&user1, &user2, 600_i128).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(token.balance(&user1), 400);
    assert_eq!(token.balance(&user2), 600);

    token.transfer_from(&user3, &user2, &user1, &400);
    assert_eq!(
        e.auths(),
        [(
            user3.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    Symbol::new(&e, "transfer_from"),
                    (&user3, &user2, &user1, 400_i128).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(token.balance(&user1), 800);
    assert_eq!(token.balance(&user2), 200);

    token.transfer(&user1, &user3, &300);
    assert_eq!(token.balance(&user1), 500);
    assert_eq!(token.balance(&user3), 300);
    assert_eq!(token.total_supply(), 1000);

    // Increase by 400, with an existing 100 = 500
    token.approve(&user2, &user3, &400, &0);
    assert_eq!(token.allowance(&user2, &user3), 400);
    token.approve(&user2, &user3, &0, &0);
    assert_eq!(
        e.auths(),
        [(
            user2.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    Symbol::new(&e, "approve"),
                    (&user2, &user3, 0_i128, 0u32).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(token.allowance(&user2, &user3), 0);
}

#[test]
#[should_panic(expected = "not used")]
fn test_burn() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::random(&e);
    let user2 = Address::random(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.approve(&user1, &user2, &500, &0);
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

    token.approve(&user1, &user3, &100, &0);
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
