# StellarBroker router smart contract

This contract facilitates multi-hop swaps prepared by [StellarBroker Router service](https://stellar.broker/)
for Stellar liquidity pool protocols.

Supported protocols:

- [Aquarius](https://aqua.network)
- [SoroSwap](https://soroswap.finance)
- [Comet](https://github.com/CometDEX/comet-contracts-v1)
- [Phoenix](https://www.phoenix-hub.io)

## Contract compilation

### Prerequisites

- Ensure you have Rust installed and set up on your local machine. Follow the 
[official guide here](https://www.rust-lang.org/tools/install).
- Install [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli)

### Compilation

Checkout this repository, navigate to the repo root directory, and run build command:

```
stellar contract build
```