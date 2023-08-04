use pool_interface::{Error, UserConfiguration};
use soroban_sdk::{Address, Env};

use crate::{
    event,
    storage::{read_user_config, write_user_config},
};

pub struct UserConfigurator {
    pub user: Address,
    pub user_config: UserConfiguration,
}

#[allow(dead_code)]
impl UserConfigurator {
    pub fn new(env: &Env, user: &Address, create_if_not_exist: bool) -> Result<Self, Error> {
        let user_config = if create_if_not_exist {
            read_user_config(env, user).unwrap_or_default()
        } else {
            read_user_config(env, user)?
        };

        Ok(Self {
            user_config,
            user: user.clone(),
        })
    }

    pub fn use_as_collateral(
        &mut self,
        env: &Env,
        reserve_id: u8,
        asset: &Address,
        value: bool,
    ) -> &Self {
        self.user_config
            .set_using_as_collateral(env, reserve_id, value);

        if value {
            event::reserve_used_as_collateral_enabled(env, &self.user, asset);
        } else {
            event::reserve_used_as_collateral_disabled(env, &self.user, asset);
        }

        self
    }

    pub fn set_borrowing(&mut self, env: &Env, reserve_id: u8, value: bool) -> &Self {
        self.user_config.set_borrowing(env, reserve_id, value);

        self
    }

    pub fn write(&self, env: &Env) {
        write_user_config(env, &self.user, &self.user_config);
    }
}
