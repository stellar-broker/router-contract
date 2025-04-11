use crate::{storage, types};
use soroban_sdk::{auth, panic_with_error, symbol_short, Address, Env, IntoVal, Vec};

// Panic if current user is not admin
pub fn require_admin(e: &Env) {
    let admin = storage::get_admin(&e);
    if admin.is_none() {
        panic_with_error!(e, types::error::BrokerError::Unauthorized);
    }
    admin.unwrap().require_auth()
}

// Add authorization for the current contract to call the transfer function of the selling token
pub fn add_transfer_auth(env: &Env, pool: &Address, token: &Address, amount: i128) {
    let invocation = auth::InvokerContractAuthEntry::Contract(auth::SubContractInvocation {
        context: auth::ContractContext {
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

// Add authorization for the current contract to call the approve function of the selling token
pub fn add_approve_auth(env: &Env, pool: &Address, token: &Address, max_amount: i128) {
    let approve_invocation =
        auth::InvokerContractAuthEntry::Contract(auth::SubContractInvocation {
            context: auth::ContractContext {
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
