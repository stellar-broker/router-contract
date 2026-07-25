use soroban_sdk::{contract, contractclient, contracttype, Address, Env, Error, U256};

use super::adapter::AdapterTrait;
use crate::auth::add_transfer_auth;
use crate::types::swapinfo::LPSwap;

// Oracle hints consumed by the SushiSwap V3 pool's `swap` (fetched on-chain via get_oracle_hints).
#[contracttype]
#[derive(Clone)]
pub struct OracleHints {
    pub checkpoint: u32,
    pub checkpoint_min: u32,
    pub slot: u128,
}

// Result returned by the SushiSwap V3 pool's `swap`. Signed amounts: positive = paid into the
// pool, negative = paid out to the swapper.
#[contracttype]
#[derive(Clone)]
pub struct SwapResult {
    pub amount0: i128,
    pub amount1: i128,
    pub liquidity: u128,
    pub sqrt_price_x96: U256,
    pub tick: i32,
}

#[contractclient(name = "SushiPoolClient")]
#[allow(dead_code)]
pub trait SushiPoolTrait {
    fn get_oracle_hints(e: Env) -> OracleHints;
    fn swap(
        e: Env,
        sender: Address,
        recipient: Address,
        zero_for_one: bool,
        amount_specified: i128,
        sqrt_price_limit_x96: U256,
        hints: OracleHints,
    ) -> Result<SwapResult, Error>;
}

#[contract]
pub struct SushiswapAdapter;

// Uniswap-V3 price bounds (Q64.96); MIN+1 / MAX-1 are the "no price cap" limits, passed as the
// swap's sqrt_price_limit so the pool never caps the trade (slippage is enforced by the broker's
// route-level verify_bought). Encoded as 256-bit from_parts limbs (round-trip verified).
pub(crate) fn min_sqrt_ratio(env: &Env) -> U256 {
    // MIN_SQRT_RATIO + 1 = 4295128740
    U256::from_parts(env, 0, 0, 0, 4295128740)
}
pub(crate) fn max_sqrt_ratio(env: &Env) -> U256 {
    // MAX_SQRT_RATIO - 1 = 1461446703485210103287273052203988822378723970341
    U256::from_parts(
        env,
        0,
        4294805859,
        17280870778742802505,
        6743328256752651557,
    )
}

impl AdapterTrait for SushiswapAdapter {
    fn swap(&self, env: &Env, si: LPSwap) -> i128 {
        let client = SushiPoolClient::new(env, &si.step.pool);

        // selling index 0 == selling token0 == zero_for_one (router enforces assetA == token0)
        let zero_for_one = si.step.si == 0;
        let sqrt_price_limit = if zero_for_one {
            min_sqrt_ratio(env)
        } else {
            max_sqrt_ratio(env)
        };

        // Hints are a search-start optimization owned by the pool; fetch them on-chain in the same
        // transaction so they reflect the state `swap` acts on. Fetch BEFORE authorizing the
        // transfer: `authorize_as_current_contract` is scoped to the next sub-call, so the transfer
        // auth must be set up immediately before `swap`, not before this read.
        let hints = client.get_oracle_hints();

        // Authorize the pool to pull the input token from the broker, immediately before the swap.
        if si.to == env.current_contract_address() {
            add_transfer_auth(env, &si.step.pool, &si.in_token, si.amount);
        }

        let res = client.swap(
            &si.to,
            &si.to,
            &zero_for_one,
            &si.amount,
            &sqrt_price_limit,
            &hints,
        );

        // The swapper-received side is negative in SwapResult -> return its positive magnitude.
        if zero_for_one {
            -res.amount1
        } else {
            -res.amount0
        }
    }
}
