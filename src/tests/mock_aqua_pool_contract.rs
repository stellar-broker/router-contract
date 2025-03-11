#![allow(dead_code)]
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token::TokenClient, Address, Env, Error, Vec, U256,
};

use soroban_fixed_point_math::SorobanFixedPoint;

// Returns the amount out for a given input amount
//
// # Arguments
//
// * `amount_in` - The input amount
// * `reserves` - The reserves of the pool. Tuple where the first element is reserve of in token and the second element is reserve of out token
// * `fee` - The fee
// * `fee_multiplier` - The fee multiplier
//
// # Returns
//
// * The amount out
pub fn get_aqua_amount_out(
    env: &Env,
    amount_in: i128,
    reserves: &(i128, i128),
    fee: i128,
    fee_multiplier: i128,
) -> i128 {
    let result = (amount_in as u128).fixed_mul_floor(
        env,
        &(reserves.1 as u128),
        &((reserves.0 + amount_in) as u128),
    );
    let fee = result.fixed_mul_ceil(&env, &(fee as u128), &(fee_multiplier as u128));
    (result - fee) as i128
}

#[contract]
pub struct MockAquaPoolContract;

const FEE: u128 = 30;
const FEE_MULTIPLIER: u128 = 10_000;

#[contractimpl]
impl MockAquaPoolContract {
    pub fn init(e: Env, tokens: Vec<Address>, reserves: Vec<u128>) {
        e.storage().instance().set(&"tokens", &tokens);
        e.storage().instance().set(&"reserves", &reserves);
    }

    pub fn get_reserves(e: Env) -> Vec<u128> {
        e.storage().instance().get(&"reserves").unwrap()
    }

    pub fn swap(
        e: Env,
        user: Address,
        in_idx: u32,
        out_idx: u32,
        in_amount: u128,
        out_min: u128,
    ) -> u128 {
        user.require_auth();

        if in_idx == out_idx {
            panic_with_error!(&e, Error::from_contract_error(2007));
        }

        if in_idx > 1 {
            panic_with_error!(&e, Error::from_contract_error(2008));
        }

        if out_idx > 1 {
            panic_with_error!(&e, Error::from_contract_error(2009));
        }

        if in_amount == 0 {
            panic_with_error!(e, Error::from_contract_error(2018));
        }

        let reserves: Vec<u128> = e.storage().instance().get(&"reserves").unwrap();
        let reserve_a = reserves.get(0).unwrap();
        let reserve_b = reserves.get(1).unwrap();

        let reserves = Vec::from_array(&e, [reserve_a, reserve_b]);
        let tokens: Vec<Address> = e.storage().instance().get(&"tokens").unwrap();

        let reserve_sell = reserves.get(in_idx).unwrap();
        let reserve_buy = reserves.get(out_idx).unwrap();
        if reserve_sell == 0 || reserve_buy == 0 {
            panic_with_error!(&e, Error::from_contract_error(2010));
        }

        let out = get_aqua_amount_out(
            &e,
            in_amount as i128,
            &(reserve_sell as i128, reserve_buy as i128),
            FEE as i128,
            FEE_MULTIPLIER as i128,
        ) as u128;

        if out < out_min {
            panic_with_error!(&e, Error::from_contract_error(2006));
        }

        // Transfer the amount being sold to the contract
        let sell_token = tokens.get(in_idx).unwrap();
        let sell_token_client = TokenClient::new(&e, &sell_token);
        sell_token_client.transfer(&user, &e.current_contract_address(), &(in_amount as i128));

        if in_idx == 0 {
            e.storage().instance().set(
                &"reserves",
                &Vec::from_array(&e, [reserve_a + in_amount, reserve_b]),
            );
        } else {
            e.storage().instance().set(
                &"reserves",
                &Vec::from_array(&e, [reserve_a, reserve_b + in_amount]),
            );
        }

        let reserves: Vec<u128> = e.storage().instance().get(&"reserves").unwrap();
        let new_reserve_a = reserves.get(0).unwrap();
        let new_reserve_b = reserves.get(1).unwrap();

        // residue_numerator and residue_denominator are the amount that the invariant considers after
        // deducting the fee, scaled up by FEE_MULTIPLIER to avoid fractions
        let residue_numerator = FEE_MULTIPLIER - FEE;
        let residue_denominator = U256::from_u128(&e, FEE_MULTIPLIER);

        let new_invariant_factor = |reserve: u128, old_reserve: u128, out: u128| {
            if reserve - old_reserve > out {
                residue_denominator
                    .mul(&U256::from_u128(&e, old_reserve))
                    .add(
                        &(U256::from_u128(&e, residue_numerator)
                            .mul(&U256::from_u128(&e, reserve - old_reserve - out))),
                    )
            } else {
                residue_denominator
                    .mul(&U256::from_u128(&e, old_reserve))
                    .add(&residue_denominator.mul(&U256::from_u128(&e, reserve)))
                    .sub(&(residue_denominator.mul(&U256::from_u128(&e, old_reserve + out))))
            }
        };

        let (out_a, out_b) = if out_idx == 0 { (out, 0) } else { (0, out) };

        let new_inv_a = new_invariant_factor(new_reserve_a, reserve_a, out_a);
        let new_inv_b = new_invariant_factor(new_reserve_b, reserve_b, out_b);
        let old_inv_a = residue_denominator.mul(&U256::from_u128(&e, reserve_a));
        let old_inv_b = residue_denominator.mul(&U256::from_u128(&e, reserve_b));

        if new_inv_a.mul(&new_inv_b) < old_inv_a.mul(&old_inv_b) {
            panic_with_error!(&e, Error::from_contract_error(2004));
        }
        if out_idx == 0 {
            TokenClient::new(&e, &tokens.get(out_idx).unwrap()).transfer(
                &e.current_contract_address(),
                &user,
                &(out_a as i128),
            );
            e.storage().instance().set(
                &"reserves",
                &Vec::from_array(&e, [reserve_a - out, new_reserve_b]),
            );
        } else {
            TokenClient::new(&e, &tokens.get(out_idx).unwrap()).transfer(
                &e.current_contract_address(),
                &user,
                &(out_b as i128),
            );
            e.storage().instance().set(
                &"reserves",
                &Vec::from_array(&e, [new_reserve_a, reserve_b - out]),
            );
        }

        out
    }
}
