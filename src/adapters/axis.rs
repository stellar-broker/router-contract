use soroban_sdk::{contract, contractclient, contracttype, panic_with_error, token, xdr::FromXdr, Address, Env, Vec};

use super::adapter::AdapterTrait;
use crate::types::{error::BrokerError, swapinfo::LPSwap};

// AXIS trade direction
#[contracttype]
#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TradeDirection {
    // Sell a fixed amount of asset
    Sell = 1,
    // Buy a fixed amount of asset
    #[allow(dead_code)]
    Buy = 2,
}

// AXIS swap route hop
#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub struct TradeStep {
    // Asset to buy at this step
    pub asset: Address,
    // Maker order IDs to match
    pub orders: Vec<u128>,
}

// AXIS in-call allowance grant
#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub struct Approval {
    // Token the allowance is granted on
    pub asset: Address,
    // Absolute allowance amount
    pub amount: i128,
    // Ledger sequence the allowance lives until
    pub live_until: u32,
}

#[contractclient(name = "AxisClient")]
#[allow(dead_code)]
pub trait AxisTrait {
    fn swap(
        e: Env,
        direction: TradeDirection,
        trader: Address,
        selling: Address,
        selling_amount: i128,
        buying_amount: i128,
        path: Vec<TradeStep>,
        approve: Option<Approval>,
    ) -> (i128, i128);
}

#[contract]
pub struct AxisAdapter;

impl AdapterTrait for AxisAdapter {
    fn swap(&self, env: &Env, si: LPSwap) -> i128 {
        //step pool holds the AXIS contract address
        let axis = &si.step.pool;
        //step ext holds the maker order IDs to match (Vec<u128>)
        if si.step.ext.is_empty() {
            panic_with_error!(env, BrokerError::InvalidPath);
        }
        let orders = Vec::<u128>::from_xdr(env, &si.step.ext)
            .unwrap_or_else(|_| panic_with_error!(env, BrokerError::InvalidPath));
        //single-hop AXIS swap over the orders listed by the router
        let path = Vec::from_array(
            env,
            [TradeStep {
                asset: si.step.asset.clone(),
                orders,
            }],
        );

        //allow AXIS to collect the selling amount from the broker until the end of the current ledger
        token::Client::new(env, &si.in_token).approve(
            &si.to,
            axis,
            &si.amount,
            &env.ledger().sequence(),
        );

        //sell the whole amount, AXIS leaves the sub-unit input it cannot use with the broker
        let (_, bought) = AxisClient::new(env, axis).swap(
            &TradeDirection::Sell,
            &si.to,
            &si.in_token,
            &si.amount,
            &1,
            &path,
            &None,
        );
        bought
    }
}
