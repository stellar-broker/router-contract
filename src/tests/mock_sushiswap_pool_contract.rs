#![allow(dead_code)]
use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, U256};

use crate::adapters::sushiswap::{OracleHints, SwapResult};

// Minimal SushiSwap V3 pool mock for the broker adapter test. Exposes the two functions the
// SushiswapAdapter calls: `get_oracle_hints` and `swap`. `swap` requires the sender's auth (like
// the real pool), pulls the input token from `sender`, and pays a fixed 99% of it in the output
// token to `recipient`, returning a SwapResult whose received (out) side is negative — matching the
// real pool's sign convention.
#[contract]
pub struct MockSushiPoolContract;

#[contractimpl]
impl MockSushiPoolContract {
    pub fn init(e: Env, token0: Address, token1: Address) {
        e.storage().instance().set(&"token0", &token0);
        e.storage().instance().set(&"token1", &token1);
    }

    pub fn get_oracle_hints(_e: Env) -> OracleHints {
        OracleHints {
            checkpoint: 1,
            checkpoint_min: 0,
            slot: 0,
        }
    }

    pub fn swap(
        e: Env,
        sender: Address,
        recipient: Address,
        zero_for_one: bool,
        amount_specified: i128,
        _sqrt_price_limit_x96: U256,
        _hints: OracleHints,
    ) -> SwapResult {
        sender.require_auth();

        let token0: Address = e.storage().instance().get(&"token0").unwrap();
        let token1: Address = e.storage().instance().get(&"token1").unwrap();
        let (in_token, out_token) = if zero_for_one {
            (token0, token1)
        } else {
            (token1, token0)
        };
        let out = amount_specified * 99 / 100;
        // pull input from sender, pay output to recipient
        TokenClient::new(&e, &in_token).transfer(
            &sender,
            &e.current_contract_address(),
            &amount_specified,
        );
        TokenClient::new(&e, &out_token).transfer(&e.current_contract_address(), &recipient, &out);
        // received side is negative
        if zero_for_one {
            SwapResult {
                amount0: amount_specified,
                amount1: -out,
                liquidity: 0,
                sqrt_price_x96: U256::from_u32(&e, 1),
                tick: 0,
            }
        } else {
            SwapResult {
                amount0: -out,
                amount1: amount_specified,
                liquidity: 0,
                sqrt_price_x96: U256::from_u32(&e, 1),
                tick: 0,
            }
        }
    }
}