use kanatoko::soroban_env_host::xdr::ScError;
use kanatoko::soroban_sdk::{token::TokenClient, vec, Address, Bytes, Vec, U256};
use kanatoko::{mainnet, ScenarioFork};

mod broker {
    use kanatoko::soroban_sdk;
    soroban_sdk::contractimport!(file = "wasm/stellarbroker.wasm");
}

// Generated clients retain the deployed contract's argument lists.
#[allow(clippy::too_many_arguments)]
mod sushi {
    use kanatoko::soroban_sdk;
    // The deployed spec refers to this U256 alias in an auxiliary quote type.
    type SqrtPriceX96 = soroban_sdk::U256;
    soroban_sdk::contractimport!(file = "wasm/sushi_pool.wasm");
}

const POOL: &str = "CCR2CH4GQVCZHG7CHFVMNANCK45CU5DVKXZIIITDZQAU3CEJZ7RQH2MQ";
const XLM: &str = "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA";
const USDC: &str = "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75";
const AMOUNT: i128 = 10_000_000;

mod fixture;

struct Setup {
    trader: Address,
    broker: Address,
    pool: Address,
    input: Address,
    output: Address,
    input_index: u32,
}

fn setup(fork: &ScenarioFork<'_>, zero_for_one: bool) -> Setup {
    fork.mock_all_auths();
    let pool = fork.contract(POOL);
    let client = sushi::Client::new(fork.env(), &pool);
    assert!(client.is_initialized());
    assert!(!client.is_paused());
    assert!(client.liquidity() > 0);
    let xlm = fork.contract(XLM);
    let usdc = fork.contract(USDC);
    assert_eq!(client.token0(), xlm);
    assert_eq!(client.token1(), usdc);
    let trader = fork.local_account("broker-sushi-trader");
    fork.fund_local_account(&trader, 1_000_000_000);
    fork.invoke::<()>(&usdc, "trust", (&trader,));
    fork.invoke::<()>(&usdc, "set_authorized", (&trader, true));
    fork.invoke::<()>(&usdc, "mint", (&trader, 1_000_000_000_i128));
    let admin = fork.local_account("broker-sushi-admin");
    let broker = fork.deploy(broker::WASM, ());
    fork.invoke::<()>(&broker, "init", (&admin, &usdc));
    fork.invoke::<()>(
        &broker,
        "enable_protocol",
        (broker::Protocol::Sushiswap, true),
    );
    let (input, output) = if zero_for_one {
        (xlm, usdc)
    } else {
        (usdc, xlm)
    };
    Setup {
        trader,
        broker,
        pool,
        input,
        output,
        input_index: u32::from(!zero_for_one),
    }
}

fn balances(fork: &ScenarioFork<'_>, p: &Setup) -> [i128; 6] {
    let a = TokenClient::new(fork.env(), &p.input);
    let b = TokenClient::new(fork.env(), &p.output);
    [
        a.balance(&p.trader),
        b.balance(&p.trader),
        a.balance(&p.broker),
        b.balance(&p.broker),
        a.balance(&p.pool),
        b.balance(&p.pool),
    ]
}

fn routes(fork: &ScenarioFork<'_>, p: &Setup, minimum: i128) -> Vec<broker::Route> {
    vec![
        fork.env(),
        broker::Route {
            amount: AMOUNT,
            min: minimum,
            estimated: minimum,
            path: vec![
                fork.env(),
                broker::PathStep {
                    protocol: broker::Protocol::Sushiswap,
                    asset: p.output.clone(),
                    pool: p.pool.clone(),
                    si: p.input_index,
                    bi: 1 - p.input_index,
                    ext: Bytes::new(fork.env()),
                },
            ],
        },
    ]
}

fn scenario(fork: &ScenarioFork<'_>, zero_for_one: bool, captured_quote: i128) {
    let p = setup(fork, zero_for_one);
    let pool = sushi::Client::new(fork.env(), &p.pool);
    let before = balances(fork, &p);
    assert_eq!(&before[2..4], &[0, 0]);
    let slot_before = pool.slot0();
    let liquidity_before = pool.liquidity();
    let hints_before = pool.get_oracle_hints();
    let empty = Vec::<broker::PathStep>::new(fork.env());
    let limit = if zero_for_one {
        U256::from_parts(fork.env(), 0, 0, 0, 4295128740)
    } else {
        U256::from_parts(
            fork.env(),
            0,
            4294805859,
            17280870778742802505,
            6743328256752651557,
        )
    };
    let quoted = pool.quote_exact_input(&zero_for_one, &AMOUNT, &limit);
    let output = if zero_for_one {
        -quoted.amount1
    } else {
        -quoted.amount0
    };
    assert_eq!(
        if zero_for_one {
            quoted.amount0
        } else {
            quoted.amount1
        },
        AMOUNT
    );
    assert_eq!(output, captured_quote, "quote from the recorded Sushi pool");
    assert_eq!(balances(fork, &p), before);
    assert_eq!(pool.slot0(), slot_before);

    let failure = fork
        .try_invoke::<Vec<i128>>(
            &p.broker,
            "swap",
            (
                &p.input,
                routes(fork, &p, output + 1),
                &p.trader,
                0_u32,
                0_u32,
                &empty,
            ),
        )
        .unwrap_err();
    assert_eq!(failure.error, Some(ScError::Contract(32712)));
    assert_eq!(balances(fork, &p), before);
    assert_eq!(pool.slot0(), slot_before);
    assert_eq!(pool.liquidity(), liquidity_before);
    assert_eq!(pool.get_oracle_hints(), hints_before);

    let result: Vec<i128> = fork.invoke(
        &p.broker,
        "swap",
        (
            &p.input,
            routes(fork, &p, output),
            &p.trader,
            0_u32,
            0_u32,
            &empty,
        ),
    );
    assert_eq!(result, vec![fork.env(), AMOUNT, output, 0_i128]);
    assert_eq!(
        balances(fork, &p),
        [
            before[0] - AMOUNT,
            before[1] + output,
            0,
            0,
            before[4] + AMOUNT,
            before[5] - output
        ]
    );
    let sqrt_after = pool.slot0().sqrt_price_x96;
    assert_eq!(sqrt_after, quoted.sqrt_price_x96);
    if zero_for_one {
        assert!(sqrt_after < slot_before.sqrt_price_x96);
    } else {
        assert!(sqrt_after > slot_before.sqrt_price_x96);
    }
    let direction = if zero_for_one {
        "XLM -> USDC"
    } else {
        "USDC -> XLM"
    };
    println!("{direction}: input {AMOUNT}, pool quote {captured_quote}, broker output {output} (raw units)");
}

#[test]
fn sushi_xlm_to_usdc_checks_exact_output_rollback_and_retry() {
    let snapshot = fixture::restore(include_str!("../fixtures/xlm-to-usdc.template.json"));
    mainnet()
        .cache(snapshot.path())
        .offline()
        .run(|fork| scenario(fork, true, 1_406_480))
        .unwrap();
}

#[test]
fn sushi_usdc_to_xlm_checks_exact_output_rollback_and_retry() {
    let snapshot = fixture::restore(include_str!("../fixtures/usdc-to-xlm.template.json"));
    mainnet()
        .cache(snapshot.path())
        .offline()
        .run(|fork| scenario(fork, false, 70_673_427))
        .unwrap();
}
