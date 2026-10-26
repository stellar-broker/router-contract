#![allow(dead_code)]
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, token::TokenClient, Address, Env,
    Error, Vec,
};

use crate::adapters::axis::{Approval, TradeDirection, TradeStep};

// fixed-point price precision (18 decimals)
const PRECISION: i128 = 1_000_000_000_000_000_000;
// AXIS error codes
const INVALID_MATCH: u32 = 704;
const NOT_FILLED: u32 = 709;

// Maker order: sells `amount` of `selling` for `buying` at `price` (`buying` per 1 `selling`, 18 decimals)
#[contracttype]
#[derive(Clone)]
pub struct MockOrder {
    pub owner: Address,
    pub selling: Address,
    pub buying: Address,
    pub amount: i128,
    pub price: i128,
}

// Planned maker fill
#[contracttype]
#[derive(Clone)]
pub struct MockFill {
    pub id: u128,
    pub sold: i128,
    pub bought: i128,
}

// Simplified AXIS order book that follows the `swap` planning and settlement rules:
// a forward pass sells the whole input, a backward pass fixes exact hop amounts,
// makers deliver to the contract with `transfer_from`, the trader pays the first hop's makers
// with `transfer_from` against its allowance, and the contract pays the later hops' makers
#[contract]
pub struct MockAxisContract;

#[contractimpl]
impl MockAxisContract {
    // Store a maker order, the maker must approve the contract to collect its amount
    pub fn add_order(e: Env, id: u128, order: MockOrder) {
        e.storage().instance().set(&id, &order);
    }

    // Amount left on a maker order
    pub fn order_amount(e: Env, id: u128) -> i128 {
        get_order(&e, id).map_or(0, |order| order.amount)
    }

    // Number of executed swaps
    pub fn swaps(e: Env) -> u32 {
        e.storage().instance().get(&"swaps").unwrap_or(0)
    }

    pub fn swap(
        e: Env,
        direction: TradeDirection,
        trader: Address,
        selling: Address,
        selling_amount: i128,
        buying_amount: i128,
        path: Vec<TradeStep>,
        approve: Option<Approval>,
    ) -> (i128, i128) {
        trader.require_auth();
        //only what the broker uses is supported
        if direction != TradeDirection::Sell || approve.is_some() {
            panic!("unsupported swap arguments");
        }
        if path.is_empty() || selling_amount <= 0 || buying_amount <= 0 {
            panic_with_error!(&e, Error::from_contract_error(INVALID_MATCH));
        }

        //forward pass: every hop sells the whole output of the previous hop
        let mut amount = selling_amount;
        for i in 0..path.len() {
            amount = plan_sell(&e, &hop_selling(&selling, &path, i), &path.get_unchecked(i), amount);
        }
        let bought = amount;
        if bought < buying_amount {
            panic_with_error!(&e, Error::from_contract_error(NOT_FILLED));
        }

        //backward pass: every hop buys exactly what the next hop sells
        let mut plans: Vec<Vec<MockFill>> = Vec::new(&e);
        for i in (0..path.len()).rev() {
            let fills = plan_buy(&e, &hop_selling(&selling, &path, i), &path.get_unchecked(i), amount);
            amount = fills.iter().map(|fill| fill.sold).sum();
            plans.push_front(fills);
        }
        let sold = amount;
        if sold > selling_amount {
            panic_with_error!(&e, Error::from_contract_error(NOT_FILLED));
        }

        //settle hops front to back
        let axis = e.current_contract_address();
        for i in 0..path.len() {
            let selling_client = TokenClient::new(&e, &hop_selling(&selling, &path, i));
            let buying_client = TokenClient::new(&e, &path.get_unchecked(i).asset);
            for fill in plans.get_unchecked(i).iter() {
                let mut order = get_order(&e, fill.id).unwrap();
                //maker delivers to the contract
                buying_client.transfer_from(&axis, &order.owner, &axis, &fill.bought);
                //the trader pays the first hop, the contract pays later hops
                if i == 0 {
                    selling_client.transfer_from(&axis, &trader, &order.owner, &fill.sold);
                } else {
                    selling_client.transfer(&axis, &order.owner, &fill.sold);
                }
                order.amount -= fill.bought;
                e.storage().instance().set(&fill.id, &order);
            }
        }
        //forward the last hop's output to the trader
        TokenClient::new(&e, &path.last_unchecked().asset).transfer(&axis, &trader, &bought);

        let swaps = Self::swaps(e.clone()) + 1;
        e.storage().instance().set(&"swaps", &swaps);
        (sold, bought)
    }
}

fn get_order(e: &Env, id: u128) -> Option<MockOrder> {
    e.storage().instance().get(&id)
}

// Asset sold at hop `i`
fn hop_selling(selling: &Address, path: &Vec<TradeStep>, i: u32) -> Address {
    if i == 0 {
        selling.clone()
    } else {
        path.get_unchecked(i - 1).asset
    }
}

// Listed orders that sell the step asset for `selling`
fn matching_orders(e: &Env, selling: &Address, step: &TradeStep) -> Vec<(u128, MockOrder)> {
    let mut orders = Vec::new(e);
    for id in step.orders.iter() {
        if let Some(order) = get_order(e, id) {
            if order.amount > 0 && order.selling == step.asset && &order.buying == selling {
                orders.push_back((id, order));
            }
        }
    }
    orders
}

// Sell `amount` into the step orders, returns the bought amount
fn plan_sell(e: &Env, selling: &Address, step: &TradeStep, amount: i128) -> i128 {
    let mut left = amount;
    let mut bought = 0;
    let mut max_price = 0;
    for (_, order) in matching_orders(e, selling, step).iter() {
        if left == 0 {
            break;
        }
        let cost = mul_div_ceil(order.amount, order.price, PRECISION);
        let (fill_sold, fill_bought) = if left >= cost {
            (cost, order.amount)
        } else {
            let fill_bought = left * PRECISION / order.price;
            (mul_div_ceil(fill_bought, order.price, PRECISION), fill_bought)
        };
        if fill_sold == 0 || fill_bought == 0 {
            continue;
        }
        left -= fill_sold;
        bought += fill_bought;
        max_price = max_price.max(order.price);
    }
    //a hop must buy something and may leave only input too small to buy one more base unit
    if bought == 0 || left * PRECISION / max_price > 0 {
        panic_with_error!(e, Error::from_contract_error(NOT_FILLED));
    }
    bought
}

// Buy exactly `amount` from the step orders
fn plan_buy(e: &Env, selling: &Address, step: &TradeStep, amount: i128) -> Vec<MockFill> {
    let mut left = amount;
    let mut fills = Vec::new(e);
    for (id, order) in matching_orders(e, selling, step).iter() {
        if left == 0 {
            break;
        }
        let bought = left.min(order.amount);
        let sold = mul_div_ceil(bought, order.price, PRECISION);
        fills.push_back(MockFill { id, sold, bought });
        left -= bought;
    }
    if left > 0 {
        panic_with_error!(e, Error::from_contract_error(NOT_FILLED));
    }
    fills
}

fn mul_div_ceil(a: i128, b: i128, denominator: i128) -> i128 {
    let product = a.checked_mul(b).unwrap();
    let result = product / denominator;
    if product % denominator != 0 {
        result + 1
    } else {
        result
    }
}
