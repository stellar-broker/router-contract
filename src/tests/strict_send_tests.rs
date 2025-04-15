extern crate std;
use crate::tests::swap_test_context::{amount, setup};
use crate::types::protocol::Protocol;
use crate::types::route::Route;
use crate::types::step::PathStep;
use soroban_sdk::Vec;

#[test]
fn swap_usdc_eurc() {
    let ctx = setup();
    ctx.fund_trader(&ctx.usdc, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: amount(70),
            estimated: amount(70),
            path: ctx.path([
                ctx.step(&ctx.usdc_xlm_pool, ctx.xlm.clone()),
                ctx.step(&ctx.xlm_eurc_pool, ctx.eurc.clone()),
            ]),
        }],
    );

    ctx.broker_client.swap(
        &ctx.usdc,
        &swaps,
        &ctx.trader,
        &150,
        &10,
        &ctx.path([
            ctx.step(&ctx.xlm_eurc_pool, ctx.xlm.clone()),
            ctx.step(&ctx.usdc_xlm_pool, ctx.usdc.clone()),
        ]),
    );

    ctx.check_contract_fee_balance(32826388);
    ctx.check_trader_balance(&ctx.eurc, 800679106);
    ctx.check_trader_balance(&ctx.usdc, 0);
    ctx.check_no_interim_leftovers();
    ctx.withdraw_fees();
}

#[test]
fn swap_eurc_usdc() {
    let ctx = setup();
    ctx.fund_trader(&ctx.eurc, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(55),
            min: amount(60),
            estimated: amount(60),
            path: ctx.path([
                ctx.step(&ctx.xlm_eurc_pool, ctx.xlm.clone()),
                ctx.step(&ctx.usdc_xlm_pool, ctx.usdc.clone()),
            ]),
        }],
    );

    ctx.broker_client
        .swap(&ctx.eurc, &swaps, &ctx.trader, &300, &0, &ctx.path([]));

    ctx.check_contract_fee_balance(16790041);
    ctx.check_trader_balance(&ctx.eurc, amount(45));
    ctx.check_trader_balance(&ctx.usdc, 639176764);
    ctx.check_no_interim_leftovers();
    ctx.withdraw_fees();
}

#[test]
fn swap_xlm_eurc() {
    let ctx = setup();
    ctx.fund_trader(&ctx.xlm, amount(1000));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(730),
            min: amount(58),
            estimated: amount(60),
            path: ctx.path([
                ctx.step(&ctx.usdc_xlm_pool, ctx.usdc.clone()),
                ctx.step(&ctx.usdc_eurc_pool, ctx.eurc.clone()),
            ]),
        }],
    );

    ctx.broker_client.swap(
        &ctx.xlm,
        &swaps,
        &ctx.trader,
        &300,
        &0,
        &ctx.path([ctx.step(&ctx.usdc_eurc_pool, ctx.usdc.clone())]),
    );

    ctx.check_contract_fee_balance(1665767);
    ctx.check_trader_balance(&ctx.xlm, amount(270));
    ctx.check_trader_balance(&ctx.eurc, 603248700);
    ctx.check_no_interim_leftovers();
    ctx.withdraw_fees();
}

#[test]
fn swap_no_fees() {
    // result < estimated, no fee charged
    let ctx = setup();
    ctx.fund_trader(&ctx.xlm, amount(1000));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(730),
            min: amount(58),
            estimated: amount(70),
            path: ctx.path([
                ctx.step(&ctx.usdc_xlm_pool, ctx.usdc.clone()),
                ctx.step(&ctx.usdc_eurc_pool, ctx.eurc.clone()),
            ]),
        }],
    );

    ctx.broker_client.swap(
        &ctx.xlm,
        &swaps,
        &ctx.trader,
        &300,
        &0,
        &ctx.path([ctx.step(&ctx.usdc_eurc_pool, ctx.usdc.clone())]),
    );

    ctx.check_contract_fee_balance(0);
    ctx.check_trader_balance(&ctx.xlm, amount(270));
    ctx.check_trader_balance(&ctx.eurc, 604641000);
    ctx.check_no_interim_leftovers();
    ctx.withdraw_fees();
}

#[test]
#[should_panic(expected = "32712")]
fn swap_fail_less_min() {
    // result < min, unfeasible
    let ctx = setup();
    ctx.fund_trader(&ctx.xlm, amount(1000));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(730),
            min: amount(68),
            estimated: amount(70),
            path: ctx.path([
                ctx.step(&ctx.usdc_xlm_pool, ctx.usdc.clone()),
                ctx.step(&ctx.usdc_eurc_pool, ctx.eurc.clone()),
            ]),
        }],
    );

    ctx.broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &300, &0, &ctx.path([]));
}

#[test]
#[should_panic(expected = "32711")]
fn swap_fail_invalid_step() {
    // result < min, unfeasible
    let ctx = setup();
    ctx.fund_trader(&ctx.xlm, amount(1000));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(730),
            min: amount(68),
            estimated: amount(70),
            path: ctx.path([PathStep {
                protocol: Protocol::Soroswap,
                asset: ctx.usdc.clone(),
                pool: ctx.xlm_eurc_pool.clone(),
                si: 0,
                bi: 2,
            }]),
        }],
    );

    ctx.broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &300, &0, &ctx.path([]));
}

#[test]
#[should_panic(expected = "32710")]
fn swap_fail_protocol_disabled() {
    // result < min, unfeasible
    let ctx = setup();
    ctx.fund_trader(&ctx.xlm, amount(1000));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(730),
            min: amount(68),
            estimated: amount(70),
            path: ctx.path([PathStep {
                protocol: Protocol::Phoenix,
                asset: ctx.usdc.clone(),
                pool: ctx.xlm_eurc_pool.clone(),
                si: 0,
                bi: 2,
            }]),
        }],
    );

    ctx.broker_client.swap(
        &ctx.xlm,
        &swaps,
        &ctx.trader,
        &300,
        &0,
        &ctx.path([]),
    );
}
