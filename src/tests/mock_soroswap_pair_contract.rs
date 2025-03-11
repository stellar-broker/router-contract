use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, Error, Vec};

#[contract]
pub struct MockSoroswapPairContract;

fn checked_ceiling_div(a: i128, divisor: i128) -> Option<i128> {
    let result = a.checked_div(divisor)?;
    if a % divisor != 0 {
        result.checked_add(1)
    } else {
        Some(result)
    }
}

#[contractimpl]
impl MockSoroswapPairContract {
    pub fn init(e: Env, tokens: Vec<Address>, reserves: Vec<u128>) {
        e.storage().instance().set(&"tokens", &tokens);
        e.storage().instance().set(&"reserves", &reserves);
    }

    pub fn token_0(e: Env) -> Address {
        let tokens:Vec<Address> = e.storage().instance().get(&"tokens").unwrap();
        tokens.get(0).unwrap()
    }

    pub fn token_1(e: Env) -> Address {
        let tokens:Vec<Address> = e.storage().instance().get(&"tokens").unwrap();
        tokens.get(1).unwrap()
    }

    pub fn get_reserves(e: Env) -> (i128, i128) {
        let reserves: Vec<u128> = e.storage().instance().get(&"reserves").unwrap();
        (reserves.get(0).unwrap() as i128, reserves.get(1).unwrap() as i128)
    }

    pub fn swap(
        e: Env,
        amount_0_out: i128,
        amount_1_out: i128,
        to: Address,
    ) -> Result<(), Error> {
    
        let reserves: Vec<u128> = e.storage().instance().get(&"reserves").unwrap();

        let reserve_0 = reserves.get(0).unwrap() as i128;
        let reserve_1 = reserves.get(1).unwrap() as i128;
    
        if amount_0_out == 0 && amount_1_out == 0 {
            return Err(Error::from_contract_error(102));
        }
        if amount_0_out < 0 || amount_1_out < 0 {
            return Err(Error::from_contract_error(109));
        }
        if amount_0_out >= reserve_0 || amount_1_out >= reserve_1 {
            return Err(Error::from_contract_error(110));
        }

        let tokens:Vec<Address> = e.storage().instance().get(&"tokens").unwrap();

        let token_0 = tokens.get(0).unwrap();
        let token_1 = tokens.get(1).unwrap();

        if to == token_0 || to == token_1 {
            return Err(Error::from_contract_error(111));
        }

        let token_0_client = TokenClient::new(&e, &token_0);
        let token_1_client = TokenClient::new(&e, &token_1);


        if amount_0_out > 0 {
            token_0_client.transfer(&e.current_contract_address(), &to, &amount_0_out);
        }
        if amount_1_out > 0 {
            token_1_client.transfer(&e.current_contract_address(), &to, &amount_1_out);
        }

        let (balance_0, balance_1) = (token_0_client.balance(&e.current_contract_address()), token_1_client.balance(&e.current_contract_address()));

        let amount_0_in = if balance_0 > reserve_0.checked_sub(amount_0_out).unwrap() {
            balance_0.checked_sub(reserve_0.checked_sub(amount_0_out).unwrap()).unwrap()
        } else {
            0
        };
        let amount_1_in = if balance_1 > reserve_1.checked_sub(amount_1_out).unwrap() {
            balance_1.checked_sub(reserve_1.checked_sub(amount_1_out).unwrap()).unwrap()
        } else {
            0
        };

        if amount_0_in == 0 && amount_1_in == 0 {
            return Err(Error::from_contract_error(112));
        }
        if amount_0_in < 0 || amount_1_in < 0 {
            return Err(Error::from_contract_error(113));
        }
        
        let fee_0 = checked_ceiling_div(amount_0_in.checked_mul(3).unwrap(), 1000).unwrap();
        let fee_1 = checked_ceiling_div(amount_1_in.checked_mul(3).unwrap(), 1000).unwrap();

        let balance_0_minus_fee = balance_0.checked_sub(fee_0).unwrap();
        let balance_1_minus_fee = balance_1.checked_sub(fee_1).unwrap();

        if balance_0_minus_fee.checked_mul(balance_1_minus_fee).unwrap() <
            reserve_0.checked_mul(reserve_1).unwrap() {
            return Err(Error::from_contract_error(114));
        }

        e.storage().instance().set(&"reserves", &Vec::from_array(&e, [balance_0 as u128, balance_1 as u128]));      

        Ok(())
    }
}