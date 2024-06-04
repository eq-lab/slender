use pool_interface::types::permission::Permission;
use soroban_sdk::{Address, Env};

use crate::read_permission_owners;

pub fn permissioned(env: &Env, who: &Address, permission: &Permission) -> bool {
    let permission_owners = read_permission_owners(env, permission);
    match permission_owners.binary_search(who) {
        Ok(_) => true,
        Err(_) => false,
    }
}
