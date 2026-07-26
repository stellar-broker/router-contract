# StellarBroker router smart contract

This contract facilitates multi-hop swaps prepared by [StellarBroker Router service](https://stellar.broker/)
for Stellar liquidity pool protocols.

Supported protocols:

- [Aquarius](https://aqua.network)
- [SoroSwap](https://soroswap.finance)
- [Comet](https://github.com/CometDEX/comet-contracts-v1)
- [Phoenix](https://www.phoenix-hub.io)
- [SushiSwap](https://www.sushi.com/)

## Contract compilation

### Prerequisites

- Ensure you have Rust installed and set up on your local machine. Follow the 
[official guide here](https://www.rust-lang.org/tools/install).
- Install [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli)

### Compilation

Checkout this repository, navigate to the repo root directory, and run build command:

```shell
stellar contract build
```

### Generate JS bindings

```shell
stellar contract bindings typescript --wasm target\wasm32v1-none\release\stellarbroker.wasm --overwrite --output-dir bindings/
```

## Initialization

After deployment, run `init()` and then enable each protocol: 

```shell
# init, provide admin account address and USDC contract address for fee token
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- init --admin <ADMIN_G...> --fee_token <FEE_TOKEN_C...>

# enable protocols
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol AquaConstant --enabled true
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol AquaStable --enabled true
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol Soroswap --enabled true
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol Comet --enabled true
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol Phoenix --enabled true
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol AquaConcentrated --enabled true
stellar contract invoke --id <DEPLOYED_ADDRESS> --source <admin> --network mainnet -- enable_protocol --protocol Sushiswap --enabled true
```