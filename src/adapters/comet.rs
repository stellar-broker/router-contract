use soroban_sdk::{contract, contractclient, Address, Env};

use crate::{extensions::auth_helper::add_approve_auth, types::swapinfo::LPSwap};

use super::adapter::AdapterTrait;

#[contractclient(name = "CometPoolClient")]
#[allow(dead_code)]
pub trait CometPoolTrait {
    fn swap_exact_amount_in(
        token_in: Address,
        token_amount_in: i128,
        token_out: Address,
        min_amount_out: i128,
        max_price: i128,
        user: Address,
    ) -> (i128, i128);
}

#[contract]
pub struct CometAdapter;

impl AdapterTrait for CometAdapter {
    fn swap(&self, env: &Env, si: LPSwap) -> i128 {
        let client = CometPoolClient::new(&env, &si.step.pool);

        if si.to == env.current_contract_address() {
            add_approve_auth(env, &si.step.pool, &si.in_token, si.amount);
        }

        client
            .swap_exact_amount_in(
                &si.in_token,
                &si.amount,
                &si.step.asset,
                &1i128,
                &MAX_PRICE,
                &si.to,
            )
            .0
    }
}

const MAX_PRICE: i128 = 18_446_744_073_709_551_615;
