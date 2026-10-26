extern crate std;
use crate::{
    tests::mock_aqua_pool_contract::{MockAquaPoolContract, MockAquaPoolContractClient},
    tests::mock_soroswap_pair_contract::{
        MockSoroswapPairContract, MockSoroswapPairContractClient,
    },
    tests::mock_sushiswap_pool_contract::{MockSushiPoolContract, MockSushiPoolContractClient},
    types::{protocol::Protocol, step::PathStep},
    StellarBroker, StellarBrokerClient,
};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Bytes, Env, Vec,
};

pub struct StrictSendTestContext<'a> {
    pub env: Env,
    pub usdc: Address,
    pub xlm: Address,
    pub eurc: Address,
    pub usdc_asset_client: StellarAssetClient<'a>,
    pub usdc_client: TokenClient<'a>,
    pub xlm_asset_client: StellarAssetClient<'a>,
    pub xlm_client: TokenClient<'a>,
    pub eurc_asset_client: StellarAssetClient<'a>,
    pub eurc_client: TokenClient<'a>,
    pub usdc_xlm_pool: Address,
    pub usdc_eurc_pool: Address,
    pub xlm_eurc_pool: Address,
    pub usdc_xlm_sushi_pool: Address,
    pub broker: Address,
    pub broker_client: StellarBrokerClient<'a>,
    pub trader: Address,
}

impl StrictSendTestContext<'_> {
    pub fn check_trader_balance(&self, asset: &Address, expected: i128) {
        let balance = self.get_token_client(asset).balance(&self.trader);
        assert_eq!(balance, expected);
    }

    pub fn check_contract_fee_balance(&self, expected: i128) {
        let balance = self.get_token_client(&self.usdc).balance(&self.broker);
        assert_eq!(balance, expected);
    }

    pub fn check_no_interim_leftovers(&self) {
        assert_eq!(self.get_token_client(&self.eurc).balance(&self.broker), 0);
        assert_eq!(self.get_token_client(&self.xlm).balance(&self.broker), 0);
    }

    // Checks the [sold, bought, fee] tuple returned by `swap`
    pub fn check_result(
        &self,
        result: Vec<i128>,
        expected_sold: i128,
        expected_bought: i128,
        expected_fee: i128,
    ) {
        assert_eq!(result.get(0).unwrap(), expected_sold);
        assert_eq!(result.get(1).unwrap(), expected_bought);
        assert_eq!(result.get(2).unwrap(), expected_fee);
    }

    pub fn fund_trader(&self, asset: &Address, amount: i128) {
        self.get_asset_client(asset).mint(&self.trader, &amount);
    }

    pub fn withdraw_fees(&self) {
        let dest = Address::generate(&self.env);
        let fee_balance = self.get_token_client(&self.usdc).balance(&self.broker);
        self.broker_client.withdraw(&dest, &self.usdc, &fee_balance);
        assert_eq!(self.get_token_client(&self.usdc).balance(&self.broker), 0);
        assert_eq!(
            self.get_token_client(&self.usdc).balance(&dest),
            fee_balance
        );
    }

    pub fn step(&self, pool: &Address, buying: Address) -> PathStep {
        let mut protocol: Protocol = Protocol::AquaConstant;
        let mut si: u32 = 0;
        let mut bi: u32 = 0;
        if pool == &self.xlm_eurc_pool {
            protocol = Protocol::Soroswap;
            if buying == self.xlm {
                si = 1;
            } else {
                bi = 1;
            }
        } else if pool == &self.usdc_eurc_pool {
            if buying == self.usdc {
                si = 1;
            } else {
                bi = 1;
            }
        } else if pool == &self.usdc_xlm_pool {
            if buying == self.usdc {
                si = 1;
            } else {
                bi = 1;
            }
        } else if pool == &self.usdc_xlm_sushi_pool {
            protocol = Protocol::Sushiswap;
            if buying == self.usdc {
                si = 1;
            } else {
                bi = 1;
            }
        } else {
            panic!("Unknown pool")
        }
        PathStep {
            protocol,
            asset: buying,
            pool: pool.clone(),
            si,
            bi,
            ext: Bytes::new(&self.env),
        }
    }

    // Same as `step`, but overrides the inferred protocol tag. Useful for pools whose swap ABI is
    // shared across protocols — e.g. routing the constant-product `usdc_xlm_pool` as
    // `AquaConcentrated`, which the concentrated adapter drives through the identical `swap` call.
    pub fn step_with_protocol(
        &self,
        pool: &Address,
        buying: Address,
        protocol: Protocol,
    ) -> PathStep {
        let mut step = self.step(pool, buying);
        step.protocol = protocol;
        step
    }

    pub fn path<const N: usize>(&self, steps: [PathStep; N]) -> Vec<PathStep> {
        Vec::from_array(&self.env, steps)
    }

    fn get_token_client(&self, asset: &Address) -> &TokenClient<'_> {
        if asset == &self.xlm {
            return &self.xlm_client;
        }
        if asset == &self.usdc {
            return &self.usdc_client;
        }
        if asset == &self.eurc {
            return &self.eurc_client;
        }
        panic!("Unknown asset")
    }

    fn get_asset_client(&self, asset: &Address) -> &StellarAssetClient<'_> {
        if asset == &self.xlm {
            return &self.xlm_asset_client;
        }
        if asset == &self.usdc {
            return &self.usdc_asset_client;
        }
        if asset == &self.eurc {
            return &self.eurc_asset_client;
        }
        panic!("Unknown asset")
    }
}

pub fn setup() -> StrictSendTestContext<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let usdc = fake_asset(&env, &issuer);
    let xlm = fake_asset(&env, &issuer);
    let eurc = fake_asset(&env, &issuer);

    let usdc_asset_client = StellarAssetClient::new(&env, &usdc);
    let usdc_client = TokenClient::new(&env, &usdc);
    let xlm_asset_client = StellarAssetClient::new(&env, &xlm);
    let xlm_client = TokenClient::new(&env, &xlm);
    let eurc_asset_client = StellarAssetClient::new(&env, &eurc);
    let eurc_client = TokenClient::new(&env, &eurc);

    //init aqua pools
    let usdc_xlm_pool = env.register(MockAquaPoolContract, ());
    let usdc_xlm_pool_client = MockAquaPoolContractClient::new(&env, &usdc_xlm_pool);
    usdc_xlm_pool_client.init(
        &Vec::from_array(&env, [usdc.clone(), xlm.clone()]),
        &Vec::from_array(&env, [amount(1000000) as u128, amount(10000000) as u128]),
    );
    usdc_asset_client.mint(&usdc_xlm_pool, &(amount(1000000)));
    xlm_asset_client.mint(&usdc_xlm_pool, &(amount(10000000)));

    let usdc_eurc_pool = env.register(MockAquaPoolContract, ());
    let usdc_eurc_pool_client = MockAquaPoolContractClient::new(&env, &usdc_eurc_pool);
    usdc_eurc_pool_client.init(
        &Vec::from_array(&env, [usdc.clone(), eurc.clone()]),
        &Vec::from_array(&env, [amount(12000000) as u128, amount(10000000) as u128]),
    );
    usdc_asset_client.mint(&usdc_eurc_pool, &(amount(12000000)));
    eurc_asset_client.mint(&usdc_eurc_pool, &(amount(10000000)));

    //init soroswap pool
    let xlm_eurc_pool = env.register(MockSoroswapPairContract, ());
    let xlm_eurc_pool_client = MockSoroswapPairContractClient::new(&env, &xlm_eurc_pool);
    xlm_eurc_pool_client.init(
        &Vec::from_array(&env, [xlm.clone(), eurc.clone()]),
        &Vec::from_array(&env, [amount(12000000) as u128, amount(1000000) as u128]),
    );
    xlm_asset_client.mint(&xlm_eurc_pool, &(amount(12000000)));
    eurc_asset_client.mint(&xlm_eurc_pool, &(amount(1000000)));

    //init sushiswap pool (token0 = usdc, token1 = xlm), funded both ways so it can swap either direction
    let usdc_xlm_sushi_pool = env.register(MockSushiPoolContract, ());
    let usdc_xlm_sushi_pool_client = MockSushiPoolContractClient::new(&env, &usdc_xlm_sushi_pool);
    usdc_xlm_sushi_pool_client.init(&usdc, &xlm);
    usdc_asset_client.mint(&usdc_xlm_sushi_pool, &(amount(1000000)));
    xlm_asset_client.mint(&usdc_xlm_sushi_pool, &(amount(1000000)));

    //init broker
    let admin = Address::generate(&env);
    let broker = env.register(StellarBroker, ());
    let broker_client = StellarBrokerClient::new(&env, &broker);
    broker_client.init(&admin, &usdc);

    //enable protocols
    broker_client.enable_protocol(&Protocol::AquaConstant, &true);
    broker_client.enable_protocol(&Protocol::Soroswap, &true);

    //init client address
    let trader = Address::generate(&env);

    StrictSendTestContext {
        env,
        usdc,
        eurc,
        xlm,
        usdc_asset_client,
        usdc_client,
        xlm_asset_client,
        xlm_client,
        eurc_asset_client,
        eurc_client,
        usdc_xlm_pool,
        usdc_eurc_pool,
        xlm_eurc_pool,
        usdc_xlm_sushi_pool,
        broker,
        broker_client,
        trader,
    }
}

pub fn fake_asset(env: &Env, issuer: &Address) -> Address {
    env.register_stellar_asset_contract_v2(issuer.clone())
        .address()
}

pub fn amount(amount: i128) -> i128 {
    amount * 10i128.pow(7)
}
