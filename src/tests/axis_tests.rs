extern crate std;
use crate::tests::mock_axis_contract::{MockAxisContract, MockAxisContractClient, MockOrder};
use crate::tests::swap_test_context::{amount, setup, StrictSendTestContext};
use crate::types::{protocol::Protocol, route::Route, step::PathStep};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    xdr::ToXdr,
    Address, Bytes, Vec,
};

struct AxisTestContext {
    ctx: StrictSendTestContext<'static>,
    axis: Address,
    axis_client: MockAxisContractClient<'static>,
}

impl AxisTestContext {
    // Place a maker order selling `amount` of `selling` for `buying`
    fn add_order(&self, id: u128, selling: &Address, buying: &Address, amount: i128, price: i128) {
        let env = &self.ctx.env;
        let maker = Address::generate(env);
        StellarAssetClient::new(env, selling).mint(&maker, &amount);
        TokenClient::new(env, selling).approve(
            &maker,
            &self.axis,
            &amount,
            &(env.ledger().sequence() + 1000),
        );
        self.axis_client.add_order(
            &id,
            &MockOrder {
                owner: maker,
                selling: selling.clone(),
                buying: buying.clone(),
                amount,
                price,
            },
        );
    }

    // AXIS hop buying `buying` from the listed maker orders
    fn step<const N: usize>(&self, buying: &Address, orders: [u128; N]) -> PathStep {
        PathStep {
            protocol: Protocol::Axis,
            asset: buying.clone(),
            pool: self.axis.clone(),
            si: 0,
            bi: 0,
            ext: Vec::from_array(&self.ctx.env, orders).to_xdr(&self.ctx.env),
        }
    }

    fn balance(&self, asset: &Address, account: &Address) -> i128 {
        TokenClient::new(&self.ctx.env, asset).balance(account)
    }

    // Neither the broker nor AXIS keeps traded assets (the broker fee balance is checked separately)
    fn check_no_leftovers(&self) {
        let ctx = &self.ctx;
        assert_eq!(self.balance(&ctx.xlm, &ctx.broker), 0);
        assert_eq!(self.balance(&ctx.eurc, &ctx.broker), 0);
        for asset in [&ctx.xlm, &ctx.usdc, &ctx.eurc] {
            assert_eq!(self.balance(asset, &self.axis), 0);
        }
    }
}

fn setup_axis() -> AxisTestContext {
    let ctx = setup();
    let axis = ctx.env.register(MockAxisContract, ());
    let axis_client = MockAxisContractClient::new(&ctx.env, &axis);
    ctx.broker_client.enable_protocol(&Protocol::Axis, &true);
    AxisTestContext {
        ctx,
        axis,
        axis_client,
    }
}

// AXIS order price given in tenths (31 = 3.1 `buying` per 1 `selling`), 18 decimals
fn price(tenths: i128) -> i128 {
    tenths * 10i128.pow(17)
}

#[test]
fn axis_single_hop() {
    let t = setup_axis();
    let ctx = &t.ctx;
    t.add_order(1, &ctx.usdc, &ctx.xlm, amount(10), price(30));
    t.add_order(2, &ctx.usdc, &ctx.xlm, amount(1000), price(31));
    ctx.fund_trader(&ctx.xlm, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: amount(32),
            estimated: 325_000_000,
            path: ctx.path([t.step(&ctx.usdc, [1, 2])]),
        }],
    );

    let result = ctx
        .broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &100, &10, &ctx.path([]));
    //the broker approves AXIS and trades as its own invoker, only the trader signs
    let auths = ctx.env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, ctx.trader);
    //order 1 is filled in full, the partial fill of order 2 leaves 1 stroop of XLM unspent
    ctx.check_result(result, 999_999_999, 322_467_742, 3_338_709);
    ctx.check_trader_balance(&ctx.xlm, 1);
    ctx.check_trader_balance(&ctx.usdc, 322_467_742);
    ctx.check_contract_fee_balance(3_338_709);
    assert_eq!(t.axis_client.order_amount(&1), 0);
    assert_eq!(t.axis_client.order_amount(&2), amount(1000) - 225_806_451);
    t.check_no_leftovers();
    ctx.withdraw_fees();
}

#[test]
fn axis_consecutive_hops() {
    let t = setup_axis();
    let ctx = &t.ctx;
    t.add_order(1, &ctx.usdc, &ctx.xlm, amount(1000), price(37));
    t.add_order(2, &ctx.eurc, &ctx.usdc, amount(1000), price(21));
    ctx.fund_trader(&ctx.xlm, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: amount(12),
            estimated: 128_000_000,
            path: ctx.path([t.step(&ctx.usdc, [1]), t.step(&ctx.eurc, [2])]),
        }],
    );

    let result = ctx
        .broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &0, &0, &ctx.path([]));
    //every hop is a separate AXIS swap, like hops of any other protocol
    assert_eq!(t.axis_client.swaps(), 2);
    //each hop leaves 1 stroop of its input it can't use: the unspent XLM of the first hop is
    //refunded to the trader, the USDC (the fee token) of the second one stays in the broker
    ctx.check_result(result, 999_999_999, 128_700_128, 0);
    ctx.check_trader_balance(&ctx.xlm, 1);
    ctx.check_trader_balance(&ctx.eurc, 128_700_128);
    assert_eq!(t.balance(&ctx.xlm, &ctx.broker), 0);
    ctx.check_contract_fee_balance(1);
    assert_eq!(t.axis_client.order_amount(&1), amount(1000) - 270_270_270);
    assert_eq!(t.axis_client.order_amount(&2), amount(1000) - 128_700_128);
    assert_eq!(t.balance(&ctx.eurc, &ctx.broker), 0);
    for asset in [&ctx.xlm, &ctx.usdc, &ctx.eurc] {
        assert_eq!(t.balance(asset, &t.axis), 0);
    }
}

#[test]
fn axis_with_amm_hop() {
    let t = setup_axis();
    let ctx = &t.ctx;
    t.add_order(1, &ctx.usdc, &ctx.xlm, amount(10), price(30));
    t.add_order(2, &ctx.usdc, &ctx.xlm, amount(1000), price(31));
    ctx.fund_trader(&ctx.xlm, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: 260_000_000,
            estimated: 265_000_000,
            path: ctx.path([
                t.step(&ctx.usdc, [1, 2]),
                ctx.step(&ctx.usdc_eurc_pool, ctx.eurc.clone()),
            ]),
        }],
    );

    let result = ctx.broker_client.swap(
        &ctx.xlm,
        &swaps,
        &ctx.trader,
        &100,
        &10,
        &ctx.path([ctx.step(&ctx.usdc_eurc_pool, ctx.usdc.clone())]),
    );
    assert_eq!(t.axis_client.swaps(), 1);
    ctx.check_result(result, 999_999_999, 267_414_211, 3_919_322);
    ctx.check_trader_balance(&ctx.xlm, 1);
    ctx.check_trader_balance(&ctx.eurc, 267_414_211);
    ctx.check_contract_fee_balance(3_919_322);
    t.check_no_leftovers();
    ctx.withdraw_fees();
}

#[test]
fn axis_after_amm_hop_leaves_fee_token_dust() {
    let t = setup_axis();
    let ctx = &t.ctx;
    t.add_order(1, &ctx.eurc, &ctx.usdc, amount(1000), price(21));
    ctx.fund_trader(&ctx.xlm, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: 46_000_000,
            estimated: 47_000_000,
            path: ctx.path([
                ctx.step(&ctx.usdc_xlm_pool, ctx.usdc.clone()),
                t.step(&ctx.eurc, [1]),
            ]),
        }],
    );

    let result = ctx.broker_client.swap(
        &ctx.xlm,
        &swaps,
        &ctx.trader,
        &100,
        &10,
        &ctx.path([ctx.step(&ctx.usdc_eurc_pool, ctx.usdc.clone())]),
    );
    ctx.check_result(result, amount(100), 46_953_387, 624_912);
    ctx.check_trader_balance(&ctx.xlm, 0);
    ctx.check_trader_balance(&ctx.eurc, 46_953_387);
    //AXIS sells 99_699_002 of the 99_699_003 USDC stroops bought from the AMM,
    //the unspent stroop stays with the broker on top of the charged fee
    ctx.check_contract_fee_balance(624_912 + 1);
    t.check_no_leftovers();
}

#[test]
fn axis_dust_refund() {
    let t = setup_axis();
    let ctx = &t.ctx;
    //a costly base unit, like a token with few decimals: 2000 XLM stroops per EURC stroop
    t.add_order(1, &ctx.eurc, &ctx.xlm, 1_000_000, price(20_000));
    ctx.fund_trader(&ctx.xlm, 500_001_999);

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: 500_001_999,
            min: 250_000,
            estimated: 250_000,
            path: ctx.path([t.step(&ctx.eurc, [1])]),
        }],
    );

    let result = ctx
        .broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &0, &0, &ctx.path([]));
    //1_999 stroops cannot buy another EURC stroop and are refunded to the trader
    ctx.check_result(result, 500_000_000, 250_000, 0);
    ctx.check_trader_balance(&ctx.xlm, 1_999);
    ctx.check_trader_balance(&ctx.eurc, 250_000);
    ctx.check_contract_fee_balance(0);
    t.check_no_leftovers();
}

#[test]
fn axis_selling_fee_token() {
    let t = setup_axis();
    let ctx = &t.ctx;
    t.add_order(1, &ctx.eurc, &ctx.usdc, amount(1000), price(23));
    ctx.fund_trader(&ctx.usdc, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: amount(42),
            estimated: amount(43),
            path: ctx.path([t.step(&ctx.eurc, [1])]),
        }],
    );

    let result = ctx.broker_client.swap(
        &ctx.usdc,
        &swaps,
        &ctx.trader,
        &100,
        &10,
        &ctx.path([ctx.step(&ctx.usdc_eurc_pool, ctx.usdc.clone())]),
    );
    //the unspent USDC stroop is told apart from the fee converted to USDC and refunded
    ctx.check_result(result, 999_999_999, 429_956_522, 5_773_928);
    ctx.check_trader_balance(&ctx.usdc, 1);
    ctx.check_trader_balance(&ctx.eurc, 429_956_522);
    ctx.check_contract_fee_balance(5_773_928);
    t.check_no_leftovers();
    ctx.withdraw_fees();
}

#[test]
#[should_panic(expected = "32710")]
fn axis_disabled() {
    let t = setup_axis();
    let ctx = &t.ctx;
    ctx.broker_client.enable_protocol(&Protocol::Axis, &false);
    t.add_order(1, &ctx.usdc, &ctx.xlm, amount(1000), price(31));
    ctx.fund_trader(&ctx.xlm, amount(100));

    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: amount(32),
            estimated: amount(32),
            path: ctx.path([t.step(&ctx.usdc, [1])]),
        }],
    );

    ctx.broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &0, &0, &ctx.path([]));
}

// AXIS hop whose `ext` doesn't hold a Vec<u128> of order IDs
fn swap_with_ext(ext: impl Fn(&AxisTestContext) -> Bytes) {
    let t = setup_axis();
    let ctx = &t.ctx;
    t.add_order(1, &ctx.usdc, &ctx.xlm, amount(1000), price(31));
    ctx.fund_trader(&ctx.xlm, amount(100));

    let mut step = t.step(&ctx.usdc, [1]);
    step.ext = ext(&t);
    let swaps = Vec::from_array(
        &ctx.env,
        [Route {
            amount: amount(100),
            min: amount(32),
            estimated: amount(32),
            path: ctx.path([step]),
        }],
    );

    ctx.broker_client
        .swap(&ctx.xlm, &swaps, &ctx.trader, &0, &0, &ctx.path([]));
}

#[test]
#[should_panic(expected = "32711")]
fn axis_ext_wrong_type() {
    swap_with_ext(|t| 5u32.to_xdr(&t.ctx.env));
}

#[test]
#[should_panic(expected = "32711")]
fn axis_ext_empty() {
    swap_with_ext(|t| Bytes::new(&t.ctx.env));
}
