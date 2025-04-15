extern crate std;
use crate::tests::swap_test_context::{amount, fake_asset};
use crate::{
    tests::mock_malicious_lp_contract::{MaliciousLPContract, MaliciousLPContractClient},
    types::{protocol::Protocol, route::Route, step::PathStep},
    StellarBroker, StellarBrokerClient,
};
use soroban_sdk::{testutils::Address as _, token::StellarAssetClient, Address, Env, Vec};

#[test]
#[should_panic(expected = "32713")]
fn attempt_intermediate_asset_fee_drain() {
    let env = Env::default();

    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let usdc = fake_asset(&env, &issuer);
    let eurc = fake_asset(&env, &issuer);
    let xlm = fake_asset(&env, &issuer);

    let usdc_asset_client = StellarAssetClient::new(&env, &usdc);
    let eurc_asset_client = StellarAssetClient::new(&env, &eurc);
    let xlm_asset_client = StellarAssetClient::new(&env, &xlm);

    //init broker
    let admin = Address::generate(&env);
    let broker_address = env.register(StellarBroker, ());
    let broker_client = StellarBrokerClient::new(&env, &broker_address);
    broker_client.init(&admin, &usdc);
    let broker_accumulated_usdc_fees = amount(1000000);
    usdc_asset_client.mint(&broker_address, &broker_accumulated_usdc_fees);

    //enable protocols
    broker_client.enable_protocol(&Protocol::Comet, &true);

    //init fake contract
    let lp_address = env.register(MaliciousLPContract, ());
    let lp_client = MaliciousLPContractClient::new(&env, &lp_address);
    // final asset is EURC, and the amount to steal is the accumulated USDC fees
    lp_client.init(&eurc, &broker_accumulated_usdc_fees);
    // fund the fake contract with 1 EURC
    // this token will be sent to the broker as the final swap result
    eurc_asset_client.mint(&lp_address, &1);

    //init client address
    let trader = Address::generate(&env);
    //fund it
    xlm_asset_client.mint(&trader, &1);

    // 1 XLM -> USDC -> EURC
    let xlm_eurc_swaps = Vec::from_array(
        &env,
        [Route {
            amount: 1i128,
            min: 0,
            estimated: 0,
            path: Vec::from_array(
                &env,
                [
                    PathStep {
                        protocol: Protocol::Comet,
                        asset: usdc.clone(),
                        pool: lp_address.clone(),
                        si: 0,
                        bi: 0,
                    },
                    PathStep {
                        protocol: Protocol::Comet,
                        asset: eurc.clone(),
                        pool: lp_address.clone(),
                        si: 0,
                        bi: 0,
                    },
                ],
            ),
        }],
    );

    // execute swap
    broker_client.swap(
        &xlm,
        &xlm_eurc_swaps,
        &trader,
        &0,
        &0,
        &Vec::from_array(&env, []),
    );
}
