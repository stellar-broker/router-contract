extern crate std;
use crate::tests::mock_aqua_pool_contract::{get_aqua_amount_out, MockAquaPoolContractClient};
use crate::tests::swap_test_context::{amount, setup};
use crate::types::protocol::Protocol;
use crate::types::route::Route;
use soroban_sdk::Vec;

#[test]
fn aqua_concentrated_swap_test() {
    let ctx = setup();
    ctx.broker_client
        .enable_protocol(&Protocol::AquaConcentrated, &true);

    let pool_client = MockAquaPoolContractClient::new(&ctx.env, &ctx.usdc_xlm_pool);
    let reserves = pool_client.get_reserves();
    let reserve_in = reserves.get(0).unwrap() as i128; // usdc reserve
    let reserve_out = reserves.get(1).unwrap() as i128; // xlm reserve

    let sell_amount = amount(100);
    ctx.fund_trader(&ctx.usdc, sell_amount);

    let expected_out = get_aqua_amount_out(
        &ctx.env,
        sell_amount,
        &(reserve_in, reserve_out),
        30,
        10_000,
    );

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: sell_amount,
            min: 1,
            estimated: expected_out,
            path: ctx.path([ctx.step_with_protocol(
                &ctx.usdc_xlm_pool,
                ctx.xlm.clone(),
                Protocol::AquaConcentrated,
            )]),
        }],
    );

    let result = ctx
        .broker_client
        .swap(&ctx.usdc, &swaps, &ctx.trader, &0, &0, &ctx.path([]));

    ctx.check_trader_balance(&ctx.xlm, expected_out);
    ctx.check_trader_balance(&ctx.usdc, 0);
    ctx.check_contract_fee_balance(0);
    ctx.check_no_interim_leftovers();
    ctx.check_result(result, sell_amount, expected_out, 0);
}

#[test]
#[should_panic(expected = "32710")]
fn aqua_concentrated_disabled_test() {
    let ctx = setup();
    let sell_amount = amount(100);
    ctx.fund_trader(&ctx.usdc, sell_amount);

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: sell_amount,
            min: 1,
            estimated: 1,
            path: ctx.path([ctx.step_with_protocol(
                &ctx.usdc_xlm_pool,
                ctx.xlm.clone(),
                Protocol::AquaConcentrated,
            )]),
        }],
    );

    ctx.broker_client
        .swap(&ctx.usdc, &swaps, &ctx.trader, &0, &0, &ctx.path([]));
}

#[test]
fn sushiswap_swap_test() {
    let ctx = setup();
    ctx.broker_client
        .enable_protocol(&Protocol::Sushiswap, &true);

    // trader sells usdc (token0) -> zero_for_one (si = 0)
    let sell_amount = amount(100);
    ctx.fund_trader(&ctx.usdc, sell_amount);
    let expected_out = sell_amount * 99 / 100;

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: sell_amount,
            min: 1,
            estimated: expected_out,
            path: ctx.path([ctx.step(&ctx.usdc_xlm_sushi_pool, ctx.xlm.clone())]),
        }],
    );

    let result = ctx
        .broker_client
        .swap(&ctx.usdc, &swaps, &ctx.trader, &0, &0, &ctx.path([]));

    ctx.check_trader_balance(&ctx.xlm, expected_out);
    ctx.check_trader_balance(&ctx.usdc, 0);
    ctx.check_contract_fee_balance(0);
    ctx.check_no_interim_leftovers();
    ctx.check_result(result, sell_amount, expected_out, 0);
}
