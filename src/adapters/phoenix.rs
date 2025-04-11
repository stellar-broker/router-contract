use soroban_sdk::{contract, contractclient, Address, Env};

use super::adapter::AdapterTrait;
use crate::auth::add_transfer_auth;
use crate::types::swapinfo::LPSwap;

#[contractclient(name = "PhoenixPoolClient")]
#[allow(dead_code)]
pub trait PhoenixPoolTrait {
    fn swap(
        sender: Address,
        offer_asset: Address,
        offer_amount: i128,
        ask_asset_min_amount: Option<i128>,
        max_spread_bps: Option<i64>,
        deadline: Option<u64>,
        max_allowed_fee_bps: Option<i64>,
    ) -> i128;
}

#[contract]
pub struct PhoenixAdapter;

impl AdapterTrait for PhoenixAdapter {
    fn swap(&self, env: &Env, si: LPSwap) -> i128 {
        let client = PhoenixPoolClient::new(&env, &si.step.pool);

        if &si.to == &env.current_contract_address() {
            add_transfer_auth(env, &si.step.pool, &si.in_token, si.amount);
        }

        client.swap(&si.to, &si.in_token, &si.amount, &None, &None, &None, &None)
    }
}
