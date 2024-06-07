use crate::*;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _, LedgerInfo},
    vec, Address, Env,
};

use crate::PriceFeedMock;

#[test]
fn t() {
    let e = Env::default();
    let client = PriceFeedClient::new(&e, &e.register_contract(None, PriceFeedMock));
    let token_address_1 = Address::generate(&e);
    let token_address_2 = Address::generate(&e);
    let ledger_info = e.ledger().get();
    e.ledger().set(LedgerInfo {
        timestamp: 900,
        ..ledger_info
    });
    client.init(
        &Asset::Stellar(token_address_1.clone()),
        &vec![
            &e,
            PriceData {
                price: 100_000_000_000_000,
                timestamp: 0,
            },
            PriceData {
                price: 100_000_000_000_001,
                timestamp: 1,
            },
        ],
    );

    client.init(
        &Asset::Stellar(token_address_2.clone()),
        &vec![
            &e,
            PriceData {
                price: 10_000_000_000_000_000,
                timestamp: 0,
            },
        ],
    );

    let prices_1 = client.prices(&Asset::Stellar(token_address_1), &1000);

    assert!(!prices_1.clone().unwrap_or(vec![&e]).is_empty());
    assert_eq!(
        prices_1.clone().unwrap().get(0).unwrap().price,
        100_000_000_000_000
    );
    assert_eq!(prices_1.clone().unwrap().get(0).unwrap().timestamp, 0);
    assert_eq!(
        prices_1.clone().unwrap().get(1).unwrap().price,
        100_000_000_000_001
    );
    assert_eq!(prices_1.unwrap().get(1).unwrap().timestamp, 1);

    let prices_2 = client.prices(&Asset::Stellar(token_address_2), &1000);
    assert!(!prices_2.clone().unwrap_or(vec![&e]).is_empty());
    assert_eq!(
        prices_2.clone().unwrap().get(0).unwrap().price,
        10_000_000_000_000_000
    );
    assert_eq!(prices_2.unwrap().get(0).unwrap().timestamp, 0);
}
