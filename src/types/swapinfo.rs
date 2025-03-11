use soroban_sdk::Address;
use crate::types::step::PathStep;

// Normalized LP atomic swap descriptor
#[derive(Clone, Eq, PartialEq)]
pub struct LPSwap {
    pub step: PathStep,
    // Selling token address
    pub in_token: Address,
    // Address to receive swapped tokens
    pub to: Address,
    // Amount of tokens to sell
    pub amount: i128
}