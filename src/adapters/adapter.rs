use super::{
    aqua_constant::AquaConstantAdapter, aqua_stable::AquaStableAdapter, comet::CometAdapter,
    phoenix::PhoenixAdapter, soroswap::SoroswapAdapter,
};
use crate::extensions::env_extensions::EnvExtensions;
use crate::types::protocol::Protocol;
use soroban_sdk::{panic_with_error, Env};
use crate::types::error::BrokerError;
use crate::types::swapinfo::LPSwap;

// Standard interface for all LP protocol adapters
pub trait AdapterTrait {
    // Executes the swap directly through LP contract
    fn swap(&self, env: &Env, swap: LPSwap) -> i128;
}

// Resolve contract adapter for a given protocol
pub fn swap_adapter(e: &Env, protocol: Protocol, si: LPSwap) -> i128 {
    //protocol should be enabled
    if !e.is_protocol_enabled(&protocol) {
        panic_with_error!(&e, BrokerError::ProtocolDisabled);
    }
    //match by protocol
    match protocol {
        Protocol::AquaConstant => AquaConstantAdapter.swap(e, si),
        Protocol::AquaStable => AquaStableAdapter.swap(e, si),
        Protocol::Soroswap => SoroswapAdapter.swap(e, si),
        Protocol::Comet => CometAdapter.swap(e, si),
        Protocol::Phoenix => PhoenixAdapter.swap(e, si),
    }
}
