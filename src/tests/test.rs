extern crate std;
use crate::adapters::soroswap::calc_soroswap_amount_out;
use crate::{
    tests::{
        mock_aqua_pool_contract::{MockAquaPoolContract, MockAquaPoolContractClient},
        mock_soroswap_pair_contract::{MockSoroswapPairContract, MockSoroswapPairContractClient},
    },
    types::{protocol::Protocol, route::Route, step::PathStep},
    StellarBroker, StellarBrokerClient,
};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env, Vec,
};

#[test]
fn strict_send_test() {
    let env = Env::default();

    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let usdc = fake_asset(&env, &issuer);
    let xlm = fake_asset(&env, &issuer);
    let eurc = fake_asset(&env, &issuer);

    let usdc_asset_client = StellarAssetClient::new(&env, &usdc);
    let usdc_client = TokenClient::new(&env, &usdc);
    let xlm_client = StellarAssetClient::new(&env, &xlm);
    let eurc_asset_client = StellarAssetClient::new(&env, &eurc);
    let eurc_client = TokenClient::new(&env, &eurc);

    //init aqua pool
    let aqua_pool_address = env.register(MockAquaPoolContract, ());
    let aqua_pool_client = MockAquaPoolContractClient::new(&env, &aqua_pool_address);
    aqua_pool_client.init(
        &Vec::from_array(&env, [usdc.clone(), xlm.clone()]),
        &Vec::from_array(&env, [amount(1000000), amount(1000000)]),
    );
    usdc_asset_client.mint(&aqua_pool_address, &(amount(1000000) as i128));
    xlm_client.mint(&aqua_pool_address, &(amount(1000000) as i128));

    //init soroswap pool
    let soroswap_pool_address = env.register(MockSoroswapPairContract, ());
    let soroswap_pool_client = MockSoroswapPairContractClient::new(&env, &soroswap_pool_address);
    soroswap_pool_client.init(
        &Vec::from_array(&env, [xlm.clone(), eurc.clone()]),
        &Vec::from_array(&env, [amount(1000000), amount(1000000)]),
    );
    xlm_client.mint(&soroswap_pool_address, &(amount(1000000) as i128));
    eurc_asset_client.mint(&soroswap_pool_address, &(amount(1000000) as i128));

    //init broker
    let admin = Address::generate(&env);
    let broker_address = env.register(StellarBroker, ());
    let broker_client = StellarBrokerClient::new(&env, &broker_address);
    broker_client.init(&admin, &usdc);

    //enable protocols
    broker_client.enable_protocol(&Protocol::AquaConstant, &true);
    broker_client.enable_protocol(&Protocol::Soroswap, &true);

    //init client address
    let trader = Address::generate(&env);
    //fund it
    usdc_asset_client.mint(&trader, &10i128.pow(10));

    let usdc_eurc_swaps = Vec::from_array(
        &env,
        [Route {
            amount: amount(100) as i128,
            min: amount(80) as i128,
            estimated: amount(80) as i128,
            path: Vec::from_array(
                &env,
                [
                    PathStep {
                        protocol: Protocol::AquaConstant,
                        asset: xlm.clone(),
                        pool: aqua_pool_address.clone(),
                        si: 0,
                        bi: 1,
                    },
                    PathStep {
                        protocol: Protocol::Soroswap,
                        asset: eurc.clone(),
                        pool: soroswap_pool_address.clone(),
                        si: 0,
                        bi: 1,
                    },
                ],
            ),
        }],
    );

    let usdc_eurc_fee_path = &Vec::from_array(
        &env,
        [
            PathStep {
                protocol: Protocol::Soroswap,
                asset: xlm.clone(),
                pool: soroswap_pool_address.clone(),
                si: 1,
                bi: 0,
            },
            PathStep {
                protocol: Protocol::AquaConstant,
                asset: usdc.clone(),
                pool: aqua_pool_address.clone(),
                si: 1,
                bi: 0,
            },
        ],
    );

    //execute swap
    broker_client.swap(
        &usdc,
        &usdc_eurc_swaps,
        &trader,
        &150,
        &10,
        usdc_eurc_fee_path,
    );

    let mut contract_fees_balance = usdc_client.balance(&broker_address);
    assert_eq!(contract_fees_balance, 38791186i128);

    let eurc_swap_balance = eurc_client.balance(&trader);
    assert_eq!(eurc_swap_balance, 954801099i128);

    assert_eq!(usdc_client.balance(&trader), 9000000000i128);

    //reverse swap

    let eurc_usdc_swaps = Vec::from_array(
        &env,
        [Route {
            amount: eurc_swap_balance,
            min: amount(60) as i128,
            estimated: amount(60) as i128,
            path: Vec::from_array(
                &env,
                [
                    PathStep {
                        protocol: Protocol::Soroswap,
                        asset: xlm.clone(),
                        pool: soroswap_pool_address.clone(),
                        si: 1,
                        bi: 0,
                    },
                    PathStep {
                        protocol: Protocol::AquaConstant,
                        asset: usdc.clone(),
                        pool: aqua_pool_address.clone(),
                        si: 1,
                        bi: 0,
                    },
                ],
            ),
        }],
    );

    let eurc_usdc_fee_path = &Vec::from_array(&env, []);

    //execute swap
    broker_client.swap(
        &eurc,
        &eurc_usdc_swaps,
        &trader,
        &300,
        &0,
        eurc_usdc_fee_path,
    );

    contract_fees_balance = usdc_client.balance(&broker_address);
    assert_eq!(contract_fees_balance, 143570350i128);

    assert_eq!(eurc_client.balance(&trader), 0);

    assert_eq!(usdc_client.balance(&trader), 9844484716i128);

    broker_client.withdraw(&Address::generate(&env), &usdc, &contract_fees_balance);
}

#[test]
fn get_soroswap_amount_out_test() {
    let reserves = (190104976848, 198442923346);
    let mut amount = calc_soroswap_amount_out(18920, &reserves, false);
    assert_eq!(amount, 19690);
    amount = calc_soroswap_amount_out(19690, &reserves, true);
    assert_eq!(amount, 18805);
}

fn amount(amount: u128) -> u128 {
    amount * 10u128.pow(7)
}

fn fake_asset(env: &Env, issuer: &Address) -> Address {
    env.register_stellar_asset_contract_v2(issuer.clone())
        .address()
}
