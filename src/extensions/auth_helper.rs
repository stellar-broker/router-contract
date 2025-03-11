use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    symbol_short, Address, Env, IntoVal, Vec,
};

// Add authorization for the current contract to call the transfer function of the selling token
//
// # Arguments
//
// * `env` - The environment
// * `pool` - The pool address
// * `token` - The address of the token to sell
// * `amount` - The amount of token to sell
pub fn add_transfer_auth(env: &Env, pool: &Address, token: &Address, amount: i128) {
    let invocation = InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: token.clone(),
            fn_name: symbol_short!("transfer"),
            args: Vec::from_array(
                &env,
                [
                    env.current_contract_address().to_val(),
                    pool.clone().to_val(),
                    amount.into_val(env),
                ],
            ),
        },
        sub_invocations: Vec::new(&env),
    });

    env.authorize_as_current_contract(Vec::from_array(env, [invocation]));
}

// Add authorization for the current contract to call the approve function of the token
//
// # Arguments
//
// * `env` - The environment
// * `pool` - The pool address
// * `token` - The address of the token to approve
// * `max_amount` - The maximum amount to approve
pub fn add_approve_auth(env: &Env, pool: &Address, token: &Address, max_amount: i128) {
    let approve_invocation = InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: token.clone(),
            fn_name: symbol_short!("approve"),
            args: Vec::from_array(
                &env,
                [
                    env.current_contract_address().to_val(),
                    pool.clone().to_val(),
                    max_amount.into_val(env),
                    ((env.ledger().sequence() / 100000 + 1) * 100000).into_val(env),
                ],
            ),
        },
        sub_invocations: Vec::new(&env),
    });

    env.authorize_as_current_contract(Vec::from_array(env, [approve_invocation]));
}
