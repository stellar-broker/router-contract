# Sushi swaps through StellarBroker

The existing pool mock pays 99% of the input. This example builds StellarBroker from this checkout and uses [Kanatoko](https://crates.io/crates/kanatoko) to run it against the Sushi XLM/USDC pool at two recorded mainnet ledgers. The expected amount and final pool price come from Sushi’s own `quote_exact_input`.

| Swap | Mainnet ledger | Pool quote | Trader received |
| --- | ---: | ---: | ---: |
| 1 XLM → USDC | 65534398 | 0.1406480 USDC | 0.1406480 USDC |
| 1 USDC → XLM | 65534393 | 7.0673427 XLM | 7.0673427 XLM |

For each direction, the test asks for one unit more than the quote. The broker rejects with error 32712. Trader, broker and pool balances, `slot0`, liquidity and oracle hints match their values before the call. A retry at the quote settles with the quoted final price and zero tokens left at the broker. The broker fee is zero and account authorization is mocked locally.

[Read the Rust assertions](tests/sushi.rs#L109).

## Reproduce locally

Use Rust 1.95.0 with the `wasm32v1-none` target and Stellar CLI 27. Run these commands from the repo root. Build the broker from this checkout first.

```sh
RUSTUP_TOOLCHAIN=1.95.0 stellar contract build --locked --out-dir integration-tests/sushi/wasm
```

The [frozen ledger entries](fixtures/) record three contract code hashes. Fetch those bytes once with Stellar CLI, using a mainnet RPC provider of your choice. Set `STELLAR_RPC_URL` to its endpoint and run the following from the same directory. The [pool address and interface](https://github.com/sushi-labs/sushiswap/blob/074482e2c0752cf48f984e216088c87bed2b83af/packages/stellar/contract-bindings/pool/README.md) are also published by Sushi.

```sh
export STELLAR_NETWORK_PASSPHRASE='Public Global Stellar Network ; September 2015'
stellar contract fetch --wasm-hash 003710b383f9da7d650a7f719a7be479110266427817ebbed61d924505fcd7c7 --out-file integration-tests/sushi/wasm/sushi_pool.wasm
stellar contract fetch --wasm-hash 9f94c577ea24c2f71cbb0c964c9c9d321e02448d1fe946020afb18974b1c2602 --out-file integration-tests/sushi/wasm/9f94c577.wasm
stellar contract fetch --wasm-hash aba81900bc82d24a6f5b66a6fee775bc51543f91dc89e824bd96ffcf3879dba1 --out-file integration-tests/sushi/wasm/aba81900.wasm
cargo +1.95.0 fetch --locked --manifest-path integration-tests/sushi/Cargo.toml
cargo +1.95.0 test --locked --offline --manifest-path integration-tests/sushi/Cargo.toml
```

The [fixture reader](tests/fixture/mod.rs) matches each local binary to the SHA-256 in the ledger and writes a temporary snapshot for Kanatoko. Both swaps then run offline. The test uses the same frozen entries on every run.
