use super::{aqua_constant, aqua_stable, comet, phoenix, soroswap};
use crate::storage;
use crate::types::{error::BrokerError, protocol::Protocol, swapinfo::LPSwap};
use soroban_sdk::{panic_with_error, Env};

// Standard interface for all LP protocol adapters
pub trait AdapterTrait {
    // Executes the swap directly through LP contract
    fn swap(&self, env: &Env, swap: LPSwap) -> i128;
}

// Resolve contract adapter for a given protocol
pub fn swap_adapter(e: &Env, protocol: Protocol, si: LPSwap) -> i128 {
    //protocol should be enabled
    if !storage::is_protocol_enabled(&e, &protocol) {
        panic_with_error!(&e, BrokerError::ProtocolDisabled);
    }
    //match by protocol
    match protocol {
        Protocol::AquaConstant => aqua_constant::AquaConstantAdapter.swap(e, si),
        Protocol::AquaStable => aqua_stable::AquaStableAdapter.swap(e, si),
        Protocol::Soroswap => soroswap::SoroswapAdapter.swap(e, si),
        Protocol::Comet => comet::CometAdapter.swap(e, si),
        Protocol::Phoenix => phoenix::PhoenixAdapter.swap(e, si),
    }
}
