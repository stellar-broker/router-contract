#![no_std]

mod adapters;
mod auth;
mod storage;
mod tests;
mod types;

use adapters::adapter::swap_adapter;
use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, BytesN, Env, Vec};
use types::{error, protocol, route::Route, step::PathStep, swapinfo::LPSwap};

#[contract]
pub struct StellarBroker;

#[contractimpl]
impl StellarBroker {
    // Initialize contract
    //
    // # Arguments
    //
    // * `admin` - Admin account address
    // * `fee_token` - Fee token address
    //
    // # Panics
    //
    // Panics if the contract is already initialized
    pub fn init(e: Env, admin: Address, fee_token: Address) {
        admin.require_auth();
        // Write settings
        storage::init_settings(&e, &admin, &fee_token);
        // Extend TTL
        storage::bump_instance(&e, 1);
    }

    // Enable/disable specific LP protocol
    //
    // # Arguments
    //
    // * `protocol` - Protocol to enable/disable
    // * `enabled` - Whether the protocol should be enabled or disabled
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    // Panics if the caller is not the admin
    pub fn enable_protocol(e: Env, protocol: protocol::Protocol, enabled: bool) {
        auth::require_admin(&e);
        storage::set_protocol_enabled(&e, &protocol, enabled);
    }

    // Update the contract's WASM hash
    //
    // # Arguments
    //
    // * `wasm_hash` - New WASM hash
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    // Panics if the caller is not the admin
    pub fn update_contract(e: Env, wasm_hash: BytesN<32>) {
        auth::require_admin(&e);
        e.deployer().update_current_contract_wasm(wasm_hash)
    }

    // Perform token swaps following router instructions
    //
    // # Arguments
    //
    // * `selling` - Selling token address
    // * `routes` - Chained swap routes
    // * `trader` - Address of the trader account
    // * `vfee` - Variable fee charged from actual savings (in ‰)
    // * `ffee` - Fixed fee charged from total swap amount (in ‰)
    // * `fpath` - Fee conversion path
    //
    // # Panics
    //
    // Panics if the contract is not initialized,
    // Panics if the caller doesn't match the trader address
    // Panics if the trader does not have enough balance to perform the swap
    // Panics if the swap is unfeasible
    //
    // # Returns
    //
    // * A vector containing sold/bought amounts and charged fee
    pub fn swap(
        e: Env,
        selling: Address,
        routes: Vec<Route>,
        trader: Address,
        vfee: u32,
        ffee: u32,
        fpath: Vec<PathStep>,
    ) -> Vec<i128> {
        //require authentication
        trader.require_auth();
        //bump only if TTL < 5 days
        storage::bump_instance(&e, 5);

        let broker = e.current_contract_address();
        //estimated bought amount
        let mut estimated: i128 = 0;
        //actual bought amount
        let mut bought: i128 = 0;

        //retrieve buying asset, planned amount to sell, and min amount to receive
        let buying = get_buying_asset(&e, &routes);
        let fee_token = storage::get_fee_token(&e).unwrap();
        let (selling_amount, min_buying_amount) = estimate_routes(&routes);

        //init token clients for sold/bought tokens
        let selling_token_client = token::Client::new(&e, &selling);
        let buying_token_client = token::Client::new(&e, &buying);
        let fee_token_client = token::Client::new(&e, &fee_token);
        let fee_balance_before = fee_token_client.balance(&broker);

        //transfer selling asset to contract address to avoid missing trustline errors for the trader
        selling_token_client.transfer(&trader, &broker, &selling_amount);

        //make balances snapshot before swap
        let selling_balance_before = selling_token_client.balance(&broker);
        let buying_balance_before = buying_token_client.balance(&broker);

        //process chained swaps for each route
        for route in routes.iter() {
            let swap_result = perform_route_swap(&e, &route, &selling, &broker);
            //sum actual bought amounts
            bought = bought.checked_add(swap_result).unwrap();
            //sum total estimated amounts
            estimated = estimated.checked_add(route.estimated).unwrap();
        }

        //calculate trader profit based on estimated
        let profit = calc_profit(estimated, bought);

        let mut selling_balance_after = 0i128;
        let mut buying_balance_after = 0i128;
        let mut received_fee = 0i128;

        //charged fee = profit fee + fixed fee
        let fee = calc_fee(profit, vfee) + calc_fee(bought, ffee);
        //process fees
        if fee > 0 {
            //deduct fee from the execution result
            bought = bought.checked_sub(fee).unwrap();
            if fee_token == buying {
                received_fee = fee;
                //swap buying asset equals ref fee asset - deduct the fee from the balance variable
                buying_balance_after = -fee;
            } else {
                //convert charged fee to ref fee tokens
                received_fee = swap_fee(&e, &buying, fee, fpath, &broker);
                //adjust balance variable in case if selling asset equals ref fee asset
                if fee_token == selling {
                    selling_balance_after = -received_fee;
                }
            }
        }

        //verify that exactly selling_amount of tokens were deducted from the balance
        selling_balance_after = selling_balance_after
            .checked_add(selling_token_client.balance(&broker))
            .unwrap();
        let actual_sold = selling_balance_before.checked_sub(selling_balance_after);
        if actual_sold.unwrap() != selling_amount {
            panic_with_error!(e, error::BrokerError::Misconduct);
        }

        //verify that received at least min_buying_amount of tokens after the swap
        buying_balance_after = buying_balance_after
            .checked_add(buying_token_client.balance(&broker))
            .unwrap();
        let actual_bought = buying_balance_after.checked_sub(buying_balance_before);
        if actual_bought.unwrap() < min_buying_amount {
            panic_with_error!(e, error::BrokerError::Unfeasible);
        }

        //transfer bought tokens minus fee to the trader account
        buying_token_client.transfer(&broker, &trader, &bought);

        //verify that fee token balance is correct
        let actual_fee = fee_token_client
            .balance(&broker)
            .checked_sub(fee_balance_before)
            .unwrap();
        if actual_fee != received_fee || actual_fee < 0 {
            panic_with_error!(e, error::BrokerError::Misconduct);
        }

        //return result as array
        Vec::from_array(&e, [selling_amount, bought, received_fee])
    }

    // Withdraw accumulated fees from contract balance
    //
    // # Arguments
    //
    // * `dest` - Destination account address
    // * `token` - Token address to withdraw
    // * `amount` - Amount of tokens to withdraw
    //
    // # Panics
    //
    // Panics if the caller is not admin
    pub fn withdraw(e: Env, dest: Address, token: Address, amount: i128) {
        //check admin auth
        auth::require_admin(&e);
        //extend TTL if less than 10 days TTL left
        storage::bump_instance(&e, 10);
        //transfer tokens from the contract balance
        let token_client = token::Client::new(&e, &token);
        token_client.transfer(&e.current_contract_address(), &dest, &amount);
    }
}

// Execute chained swap based on provided route
fn perform_route_swap(e: &Env, swap: &Route, selling: &Address, to: &Address) -> i128 {
    //current amount = initial selling amount
    let mut amount = swap.amount;
    //current token = initial selling token address
    let mut in_token = selling.clone();
    //iterate and execute swap path steps
    for path_step in swap.path.iter() {
        let buying = path_step.asset.clone();
        let protocol = path_step.protocol.clone();
        let swap_info = LPSwap {
            step: path_step,
            in_token,
            amount,
            to: to.clone(),
        };
        //execute the swap, set current amount = swapped amount
        amount = swap_adapter(&e, protocol, swap_info);
        //current token = bought token address
        in_token = buying;
    }
    //return result amount
    amount
}

// Calculate variable fee based on the difference between actual and estimated swap amounts
fn calc_profit(estimated: i128, actual: i128) -> i128 {
    //calculate the difference
    let difference = actual.checked_sub(estimated);
    //no variable fee charge if no profit
    if difference.is_none() || difference.unwrap() <= 0 {
        return 0;
    }
    difference.unwrap()
}

// Calculate fee amount based on the percentage
fn calc_fee(amount: i128, share: u32) -> i128 {
    amount
        .checked_mul(share as i128)
        .unwrap()
        .checked_div(1000) //share specified in ‰
        .unwrap()
}

// Convert charged fee to ref fee tokens
fn swap_fee(e: &Env, selling: &Address, fee: i128, path: Vec<PathStep>, broker: &Address) -> i128 {
    //skip for zero fee
    if fee == 0 {
        return 0;
    }
    //build fee route
    let fee_route = Route {
        path,
        amount: fee,
        estimated: 1,
        min: 1,
    };
    //convert fee to the ref fee tokens
    perform_route_swap(&e, &fee_route, &selling, &broker)
}

// Retrieve the target token and the total amounts
fn estimate_routes(routes: &Vec<Route>) -> (i128, i128) {
    let mut total_selling: i128 = 0;
    let mut min_buying: i128 = 0;

    //sum total selling assets and min projected bought amount
    for swap in routes.iter() {
        total_selling = total_selling.checked_add(swap.amount).unwrap();
        min_buying = min_buying.checked_add(swap.min).unwrap();
    }
    //return token_out, total selling amount and min buying amount
    (total_selling, min_buying)
}

// Retrieve last token address from the path (buying token address)
fn get_buying_asset(e: &Env, routes: &Vec<Route>) -> Address {
    let mut asset: Option<Address> = None;
    //check every route
    for route in routes.iter() {
        let last_step = route.path.last();
        if last_step.is_none() {
            //zero length path
            panic_with_error!(&e, error::BrokerError::Unfeasible);
        }
        //retrieve the asset
        let route_asset = Some(last_step.unwrap().asset.clone());
        //assign the token variable
        if asset.is_none() {
            asset = route_asset;
        } else if route_asset != asset {
            //each route should have the same buying token
            panic_with_error!(&e, error::BrokerError::Unfeasible);
        }
    }
    //at least one is required
    if asset.is_none() {
        panic_with_error!(&e, error::BrokerError::Unfeasible);
    }
    asset.unwrap()
}
