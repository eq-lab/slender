use common_token::storage::{HIGH_INSTANCE_BUMP_LEDGERS, LOW_INSTANCE_BUMP_LEDGERS};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Allowance(AllowanceDataKey),
    UnderlyingAsset,
}

pub fn write_underlying_asset(env: &Env, asset: &Address) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .set(&DataKey::UnderlyingAsset, asset);
}

pub fn read_underlying_asset(env: &Env) -> Address {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::UnderlyingAsset)
        .unwrap()
}

pub fn read_allowance(e: &Env, from: Address, spender: Address) -> AllowanceValue {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    if let Some(allowance) = e.storage().temporary().get::<_, AllowanceValue>(&key) {
        if allowance.expiration_ledger < e.ledger().sequence() {
            AllowanceValue {
                amount: 0,
                expiration_ledger: allowance.expiration_ledger,
            }
        } else {
            allowance
        }
    } else {
        AllowanceValue {
            amount: 0,
            expiration_ledger: 0,
        }
    }
}

pub fn write_allowance(
    e: &Env,
    from: Address,
    spender: Address,
    amount: i128,
    expiration_ledger: u32,
) {
    let allowance = AllowanceValue {
        amount,
        expiration_ledger,
    };

    if amount > 0 && expiration_ledger < e.ledger().sequence() {
        panic!("s-token: expiration_ledger is less than ledger seq when amount > 0")
    }

    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().temporary().set(&key, &allowance);

    if amount > 0 {
        let new_expiration = expiration_ledger
            .checked_sub(e.ledger().sequence())
            .unwrap();

        e.storage()
            .temporary()
            .extend_ttl(&key, new_expiration, new_expiration)
    }
}
