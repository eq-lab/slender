use soroban_sdk::contracttype;

#[derive(Clone)]
#[contracttype]
pub enum TimestampPrecision {
    Mili,
    Seconds,
}
