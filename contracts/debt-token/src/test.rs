#![cfg(test)]
extern crate std;

use debt_token_interface::DebtTokenClient;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{symbol_short, Address, Env, IntoVal, String, Symbol};

use crate::DebtToken;

fn create_token<'a>(e: &Env) -> (DebtTokenClient<'a>, Address) {
    let pool = Address::generate(e);

    let token = DebtTokenClient::new(e, &e.register_contract(None, DebtToken {}));

    let underlying_asset = TokenClient::new(
        &e,
        &e.register_stellar_asset_contract(Address::generate(&e)),
    );

    token.initialize(
        &"name".into_val(e),
        &"symbol".into_val(e),
        &pool,
        &underlying_asset.address,
    );

    (token, pool)
}

#[test]
fn initialize() {
    let e = Env::default();
    e.mock_all_auths();
    let pool = Address::generate(&e);

    let token = DebtTokenClient::new(&e, &e.register_contract(None, DebtToken {}));

    let underlying_asset = TokenClient::new(
        &e,
        &e.register_stellar_asset_contract(Address::generate(&e)),
    );

    token.initialize(
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &underlying_asset.address,
    );

    assert_eq!(token.decimals(), 7);
    assert_eq!(token.name(), String::from_str(&e, &"name"));
    assert_eq!(token.symbol(), String::from_str(&e, &"symbol"));
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();

    let (token, pool) = create_token(&e);

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user1_amount = 1000;
    let total_supply = user1_amount;

    token.mint(&user1, &user1_amount);

    assert_eq!(
        e.auths(),
        [(
            pool.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    symbol_short!("mint"),
                    (&user1, user1_amount).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.set_authorized(&user2, &false);
    assert_eq!(
        e.auths(),
        [(
            pool.clone(),
            AuthorizedInvocation {
                sub_invocations: std::vec![],
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    Symbol::new(&e, "set_authorized"),
                    (&user2, false).into_val(&e),
                ))
            }
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
            AuthorizedInvocation {
                sub_invocations: std::vec![],
                function: AuthorizedFunction::Contract((
                    token.address.clone(),
                    symbol_short!("clawback"),
                    (&user1, clawback).into_val(&e),
                ))
            }
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

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let (token, _pool) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    token.burn_from(&user2, &user1, &500);
}

#[test]
#[should_panic(expected = "debt-token: already initialized")]
fn initialize_already_initialized() {
    let e = Env::default();
    let (token, _pool) = create_token(&e);

    let pool = Address::generate(&e);
    let underlying_asset = Address::generate(&e);

    token.initialize(
        &String::from_str(&e, &"name"),
        &String::from_str(&e, &"symbol"),
        &pool,
        &underlying_asset,
    );
}
