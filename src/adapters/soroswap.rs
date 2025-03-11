use soroban_sdk::{
    contract, contractclient, panic_with_error, token::TokenClient, Address, Env, Error,
};

use super::adapter::AdapterTrait;
use crate::types::{error::BrokerError, swapinfo::LPSwap};

#[contractclient(name = "SoroswapClient")]
#[allow(dead_code)]
pub trait SoroswapPairTrait {
    fn get_reserves(e: Env) -> (i128, i128);
    fn swap(e: Env, amount_0_out: i128, amount_1_out: i128, to: Address) -> Result<(), Error>;
}

#[contract]
pub struct SoroswapAdapter;

impl AdapterTrait for SoroswapAdapter {
    fn swap(&self, e: &Env, si: LPSwap) -> i128 {
        let token_client = TokenClient::new(e, &si.in_token);
        token_client.transfer(&si.to, &si.step.pool, &si.amount);

        let swap_client = SoroswapClient::new(e, &si.step.pool);
        let reserves = &swap_client.get_reserves();

        let amount_out = calc_soroswap_amount_out(si.amount, reserves, si.step.bi == 0);

        if si.step.bi == 1 {
            swap_client.swap(&0, &amount_out, &si.to);
        } else if si.step.bi == 0 {
            swap_client.swap(&amount_out, &0, &si.to);
        } else {
            panic_with_error!(e, BrokerError::InvalidPath);
        };

        amount_out
    }
}

const SOROSWAP_FEE: i128 = 30;
const SOROSWAP_FEEM: i128 = 10_000;

// Estimate amount_out for SoroSwap LPs
pub fn calc_soroswap_amount_out(amount_in: i128, reserves: &(i128, i128), reverse: bool) -> i128 {
    let reserve_x: i128;
    let reserve_y: i128;
    if reverse {
        reserve_x = reserves.1;
        reserve_y = reserves.0;
    } else {
        reserve_x = reserves.0;
        reserve_y = reserves.1;
    }

    let fee = checked_ceiling_div(amount_in.checked_mul(SOROSWAP_FEE).unwrap(), SOROSWAP_FEEM);
    let amount_in_less_fee = amount_in.checked_sub(fee).unwrap();
    let numerator = amount_in_less_fee.checked_mul(reserve_y).unwrap();
    let denominator = reserve_x.checked_add(amount_in_less_fee).unwrap();

    numerator.checked_div(denominator).unwrap()
}

fn checked_ceiling_div(x: i128, y: i128) -> i128 {
    //copied from SoroSwap source code
    let result = x.checked_div(y).unwrap();
    if x % y != 0 {
        result.checked_add(1).unwrap()
    } else {
        result
    }
}
