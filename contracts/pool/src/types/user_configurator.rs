use pool_interface::types::error::Error;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::{read_user_config, write_user_config};

pub struct UserConfigurator<'a> {
    env: &'a Env,
    user: &'a Address,
    create_if_none: bool,
    should_write: bool,
    user_config: Option<UserConfiguration>,
}

impl<'a> UserConfigurator<'a> {
    pub fn new(env: &'a Env, user: &'a Address, create_if_none: bool) -> Self {
        Self {
            env,
            create_if_none,
            user,
            user_config: None,
            should_write: false,
        }
    }

    pub fn withdraw(
        &mut self,
        reserve_id: u8,
        asset: &Address,
        fully_withdrawn: bool,
    ) -> Result<&mut Self, Error> {
        if !fully_withdrawn {
            return Ok(self);
        }

        let env = self.env;
        let user_config = self.read_user_config()?.user_config.as_mut().unwrap(); //@audit 1 read

        user_config.set_using_as_collateral(env, reserve_id, false);
        event::reserve_used_as_collateral_disabled(env, self.user, asset);

        self.should_write = true;

        Ok(self)
    } //@audit 1 read

    pub fn deposit(
        &mut self,
        reserve_id: u8,
        asset: &Address,
        first_deposit: bool,
    ) -> Result<&mut Self, Error> {
        if !first_deposit {
            return Ok(self);
        }

        let env = self.env;
        let user_config = self.read_user_config()?.user_config.as_mut().unwrap(); //@audit 1 read

        user_config.set_using_as_collateral(env, reserve_id, true);
        event::reserve_used_as_collateral_enabled(env, self.user, asset);

        self.should_write = true;

        Ok(self)
    } //@audit 1 read

    pub fn borrow(&mut self, reserve_id: u8, first_borrow: bool) -> Result<&mut Self, Error> {
        if !first_borrow {
            return Ok(self);
        }

        let env = self.env;
        let user_config = self.read_user_config()?.user_config.as_mut().unwrap(); //@audit 1 read

        user_config.set_borrowing(env, reserve_id, true);

        self.should_write = true;

        Ok(self)
    } //@audit 1 read

    pub fn repay(&mut self, reserve_id: u8, fully_repayed: bool) -> Result<&mut Self, Error> {
        if !fully_repayed {
            return Ok(self);
        }

        let env = self.env;
        let user_config = self.read_user_config()?.user_config.as_mut().unwrap(); //@audit 1 read
        user_config.set_borrowing(env, reserve_id, false);

        self.should_write = true;

        Ok(self)
    } //@audit 1 read

    pub fn write(&mut self) {
        if self.user_config.is_none() || !self.should_write {
            return;
        }

        let user_config = self.user_config.as_ref();

        write_user_config(self.env, self.user, user_config.unwrap());
    } //@audit 1 write

    pub fn user_config(&mut self) -> Result<&UserConfiguration, Error> {
        let user_config = self.read_user_config()?.user_config.as_ref().unwrap();

        Ok(user_config)
    } //@audit 1 read

    fn read_user_config(&mut self) -> Result<&mut Self, Error> {
        if self.user_config.is_some() {
            return Ok(self);
        }

        self.user_config = Some(if self.create_if_none {
            read_user_config(self.env, self.user).unwrap_or_default() //@audit 1 read
        } else {
            read_user_config(self.env, self.user)? //@audit 1 read
        });

        Ok(self)
    }
}
