#![allow(non_upper_case_globals)]
use soroban_sdk::storage::Instance;
use soroban_sdk::{panic_with_error, Address, Env};

use crate::types;
use crate::types::protocol::Protocol;

use types::error::BrokerError;

const ADMIN_KEY: &str = "admin";

pub trait EnvExtensions {
    fn get_admin(&self) -> Option<Address>;

    fn set_admin(&self, admin: &Address);

    fn set_protocol_enabled(&self, protocol: &Protocol, enabled: bool);

    fn is_protocol_enabled(&self, protocol: &Protocol) -> bool;

    fn bump_instance(&self);

    fn panic_if_not_admin(&self);

    fn is_initialized(&self) -> bool;
}

impl EnvExtensions for Env {
    fn is_initialized(&self) -> bool {
        get_instance_storage(&self).has(&ADMIN_KEY)
    }

    fn get_admin(&self) -> Option<Address> {
        get_instance_storage(&self).get(&ADMIN_KEY)
    }

    fn set_admin(&self, admin: &Address) {
        get_instance_storage(&self).set(&ADMIN_KEY, admin);
    }

    fn set_protocol_enabled(&self, protocol: &Protocol, enabled: bool) {
        get_instance_storage(&self).set(protocol, &enabled);
    }

    fn is_protocol_enabled(&self, protocol: &Protocol) -> bool {
        get_instance_storage(&self)
            .get(protocol)
            .unwrap_or_default()
    }

    fn panic_if_not_admin(&self) {
        let admin = self.get_admin();
        if admin.is_none() {
            panic_with_error!(self, BrokerError::Unauthorized);
        }
        admin.unwrap().require_auth()
    }

    //extend for 20 days if less than 10 days TTL left
    fn bump_instance(&self) {
        self.storage()
            .instance()
            .extend_ttl(LPH * 24 * 10, LPH * 24 * 20);
    }
}

const LPH: u32 = 720;

fn get_instance_storage(e: &Env) -> Instance {
    e.storage().instance()
}
