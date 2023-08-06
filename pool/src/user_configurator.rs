use pool_interface::{Error, UserConfiguration};
use soroban_sdk::{Address, Env};

use crate::{
    event,
    storage::{read_user_config, write_user_config},
};

pub struct UserConfigurator {
    user: Address,
    create_if_none: bool,
    should_write: bool,
    user_config: Option<UserConfiguration>,
}

impl UserConfigurator {
    pub fn new(user: &Address, create_if_none: bool) -> Result<Self, Error> {
        Ok(Self {
            create_if_none,
            user: user.clone(),
            user_config: None,
            should_write: false,
        })
    }

    pub fn withdraw(
        &mut self,
        env: &Env,
        reserve_id: u8,
        asset: &Address,
        fully_withdrawn: bool,
    ) -> Result<&mut Self, Error> {
        if !fully_withdrawn {
            return Ok(self);
        }

        let user_config = Self::read_user_config(self, env)?
            .user_config
            .as_mut()
            .unwrap();
        user_config.set_using_as_collateral(env, reserve_id, false);
        event::reserve_used_as_collateral_disabled(env, &self.user, asset);

        self.should_write = true;

        Ok(self)
    }

    pub fn deposit(
        &mut self,
        env: &Env,
        reserve_id: u8,
        asset: &Address,
        first_deposit: bool,
    ) -> Result<&mut Self, Error> {
        if !first_deposit {
            return Ok(self);
        }

        let user_config = Self::read_user_config(self, env)?
            .user_config
            .as_mut()
            .unwrap();
        user_config.set_using_as_collateral(env, reserve_id, true);
        event::reserve_used_as_collateral_enabled(env, &self.user, asset);

        self.should_write = true;

        Ok(self)
    }

    pub fn borrow(
        &mut self,
        env: &Env,
        reserve_id: u8,
        first_borrow: bool,
    ) -> Result<&mut Self, Error> {
        if !first_borrow {
            return Ok(self);
        }

        let user_config = Self::read_user_config(self, env)?
            .user_config
            .as_mut()
            .unwrap();
        user_config.set_borrowing(env, reserve_id, true);

        self.should_write = true;

        Ok(self)
    }

    pub fn repay(
        &mut self,
        env: &Env,
        reserve_id: u8,
        fully_repayed: bool,
    ) -> Result<&mut Self, Error> {
        if !fully_repayed {
            return Ok(self);
        }

        let user_config = Self::read_user_config(self, env)?
            .user_config
            .as_mut()
            .unwrap();
        user_config.set_borrowing(env, reserve_id, false);

        self.should_write = true;

        Ok(self)
    }

    pub fn write(&mut self, env: &Env) {
        if self.user_config.is_none() || !self.should_write {
            return;
        }

        let user_config = self.user_config.as_ref();

        write_user_config(env, &self.user, user_config.unwrap());
    }

    pub fn user_config(&mut self, env: &Env) -> Result<&UserConfiguration, Error> {
        let user_config = Self::read_user_config(self, env)?
            .user_config
            .as_ref()
            .unwrap();

        Ok(user_config)
    }

    fn read_user_config(&mut self, env: &Env) -> Result<&mut Self, Error> {
        if self.user_config.is_some() {
            return Ok(self);
        }

        self.user_config = Some(if self.create_if_none {
            read_user_config(env, &self.user).unwrap_or_default()
        } else {
            read_user_config(env, &self.user)?
        });

        Ok(self)
    }
}
