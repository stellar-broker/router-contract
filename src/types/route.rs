use soroban_sdk::{Vec, contracttype};

use super::step::PathStep;

// Swap route descriptor
#[derive(Clone, Eq, PartialEq)]
#[contracttype]
pub struct Route {
    // Route execution path
    pub path: Vec<PathStep>,
    // Selling amount
    pub amount: i128,
    // Min buying amount
    pub min: i128,
    // Estimated buying amount
    pub estimated: i128,
}
