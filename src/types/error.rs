use soroban_sdk::contracterror;

// Standard contract errors
#[contracterror]
#[repr(i16)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrokerError {
    // Caller is not allowed to execute this function
    Unauthorized = 32_700,
    // Contract has not been initialized yet
    NotInitialized = 32_701,
    // Cannot initialize the contract for the second time
    AlreadyInitialized = 32_702,
    // Protocol participating in the swap has been disabled
    ProtocolDisabled = 32_710,
    // Malformed swap route path
    InvalidPath = 32_711,
    // Requested quote can not be executed
    Unfeasible = 32_712,
    // LP protocol charged more than projected
    Misconduct = 32_713
}
