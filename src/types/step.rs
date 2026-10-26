use soroban_sdk::{contracttype, Address, Bytes};

use super::protocol::Protocol;

#[derive(Clone, Eq, PartialEq)]
#[contracttype]
pub struct PathStep {
    // Protocol type
    pub protocol: Protocol,
    // Buying asset address
    pub asset: Address,
    // LP contract address
    pub pool: Address,
    // Selling asset index
    pub si: u32,
    // Buying asset index
    pub bi: u32,
    // Protocol-specific parameters, XDR-encoded ScVal decoded by the adapter (empty for protocols without parameters)
    pub ext: Bytes,
}