use soroban_sdk::contracttype;

#[contracttype]
pub struct PauseInfo {
    pub paused: bool,
    pub grace_period_secs: u64,
    pub unpaused_at: u64,
}

impl PauseInfo {
    pub fn grace_period_ends_at(&self) -> u64 {
        self.unpaused_at + self.grace_period_secs
    }
}
