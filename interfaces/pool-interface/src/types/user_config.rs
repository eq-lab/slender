use soroban_sdk::{assert_with_error, contracttype, Env};

use crate::types::error::Error;

const BORROWING_MASK: u128 = 0x55555555555555555555555555555555;

#[contracttype]
#[derive(Default)]
/// Implements the bitmap logic to handle the user configuration.
/// Even positions is collateral flags and uneven is borrowing flags.
pub struct UserConfiguration(u128);

impl UserConfiguration {
    pub fn set_borrowing(&mut self, env: &Env, reserve_index: u8, borrowing: bool) {
        Self::require_reserve_index(env, reserve_index);

        let reserve_index: u128 = reserve_index.into();
        self.0 = (self.0 & !(1 << (reserve_index * 2)))
            | ((if borrowing { 1 } else { 0 }) << (reserve_index * 2));
    }

    pub fn set_using_as_collateral(
        &mut self,
        env: &Env,
        reserve_index: u8,
        using_as_collateral: bool,
    ) {
        Self::require_reserve_index(env, reserve_index);

        let reserve_index: u128 = reserve_index.into();
        self.0 = (self.0 & !(1 << (reserve_index * 2 + 1)))
            | ((if using_as_collateral { 1 } else { 0 }) << (reserve_index * 2 + 1));
    }

    pub fn is_using_as_collateral(&self, env: &Env, reserve_index: u8) -> bool {
        Self::require_reserve_index(env, reserve_index);

        let reserve_index: u128 = reserve_index.into();
        (self.0 >> (reserve_index * 2 + 1)) & 1 != 0
    }

    pub fn is_using_as_collateral_or_borrowing(&self, env: &Env, reserve_index: u8) -> bool {
        Self::require_reserve_index(env, reserve_index);

        let reserve_index: u128 = reserve_index.into();
        (self.0 >> (reserve_index * 2)) & 3 != 0
    }

    pub fn is_borrowing(&self, env: &Env, reserve_index: u8) -> bool {
        Self::require_reserve_index(env, reserve_index);
        let reserve_index: u128 = reserve_index.into();
        (self.0 >> (reserve_index * 2)) & 1 != 0
    }

    pub fn is_borrowing_any(&self) -> bool {
        self.0 & BORROWING_MASK != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    fn require_reserve_index(env: &Env, reserve_index: u8) {
        assert_with_error!(
            env,
            reserve_index < core::mem::size_of::<u128>() as u8 / 2,
            Error::UserConfigInvalidIndex
        );
    }
}
