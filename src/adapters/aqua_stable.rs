use soroban_sdk::{contract, contractclient, Address, Env};

use super::adapter::AdapterTrait;
use crate::auth::add_transfer_auth;
use crate::types::swapinfo::LPSwap;

#[contractclient(name = "AquaStablePoolClient")]
#[allow(dead_code)]
pub trait AquaStablePoolTrait {
    fn swap(
        e: Env,
        user: Address,
        in_idx: u32,
        out_idx: u32,
        in_amount: u128,
        out_min: u128,
    ) -> u128;
}

#[contract]
pub struct AquaStableAdapter;

impl AdapterTrait for AquaStableAdapter {
    fn swap(&self, env: &Env, si: LPSwap) -> i128 {
        let client = AquaStablePoolClient::new(&env, &si.step.pool);

        if si.to == env.current_contract_address() {
            add_transfer_auth(env, &si.step.pool, &si.in_token, si.amount);
        }
        let selling = &(si.amount as u128);
        client.swap(&si.to, &si.step.si, &si.step.bi, &selling, &1u128) as i128
    }
}
