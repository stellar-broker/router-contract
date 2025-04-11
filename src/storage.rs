use soroban_sdk::{Address, Env};

use crate::types;
use crate::types::protocol::Protocol;

use types::error::BrokerError;

const ADMIN_KEY: &str = "admin"; //admin key
const FEE_TOKEN_KEY: &str = "ft"; //fee token key

const LPH: u32 = 720; //estimated ledgers per hour

// Initialize contract settings
pub fn init_settings(e: &Env, admin: &Address, fee_token: &Address) {
    let storage = e.storage().instance();
    if storage.has(&ADMIN_KEY) {
        //can be initialized only once
        e.panic_with_error(BrokerError::Unauthorized);
    }
    storage.set(&ADMIN_KEY, &admin);
    storage.set(&FEE_TOKEN_KEY, &fee_token);
}

// Retrieve admin address
pub fn get_admin(e: &Env) -> Option<Address> {
    e.storage().instance().get(&ADMIN_KEY)
}

// Retrieve fee token address
pub fn get_fee_token(e: &Env) -> Option<Address> {
    e.storage().instance().get(&FEE_TOKEN_KEY)
}

// Enable the protocol
pub fn set_protocol_enabled(e: &Env, protocol: &Protocol, enabled: bool) {
    e.storage().instance().set(protocol, &enabled);
}

// Check whether the protocol is enabled
pub fn is_protocol_enabled(e: &Env, protocol: &Protocol) -> bool {
    e.storage().instance().get(protocol).unwrap_or_default()
}

// Extend TTL for 30 days if less than X days TTL left
pub fn bump_instance(e: &Env, days_left: u32) {
    let min = LPH * 24 * days_left;
    let extend = LPH * 24 * 30;
    e.storage().instance().extend_ttl(min, extend);
}
