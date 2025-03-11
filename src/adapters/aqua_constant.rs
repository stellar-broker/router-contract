use soroban_sdk::{contract, contractclient, Address, Env};

use crate::{extensions::auth_helper::add_transfer_auth, types::swapinfo::LPSwap};

use super::adapter::AdapterTrait;

#[contractclient(name = "AquaPoolClient")]
#[allow(dead_code)]
pub trait AquaPoolTrait {
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
pub struct AquaConstantAdapter;

impl AdapterTrait for AquaConstantAdapter {
    fn swap(&self, env: &Env, si: LPSwap) -> i128 {
        let client = AquaPoolClient::new(&env, &si.step.pool);

        if si.to == env.current_contract_address() {
            add_transfer_auth(env, &si.step.pool, &si.in_token, si.amount);
        }

        let selling = si.amount as u128;

        client.swap(&si.to, &si.step.si, &si.step.bi, &selling, &1u128) as i128
    }
}
