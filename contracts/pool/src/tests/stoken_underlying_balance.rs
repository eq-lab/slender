use crate::tests::sut::init_pool;
use crate::*;
use soroban_sdk::testutils::Address as _;

#[test]
fn should_not_be_changed_when_direct_transfer_to_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let lender = Address::generate(&env);

    sut.reserves[0].token_admin.mint(&lender, &2_000_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token().address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);

    sut.reserves[0]
        .token
        .transfer(&lender, &sut.reserves[0].s_token().address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token().address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);
}
