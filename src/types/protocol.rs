use soroban_sdk::contracttype;

// LP protocol type
#[contracttype]
#[repr(i8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Protocol {
    AquaConstant = 0,
    AquaStable = 1,
    Soroswap = 2,
    Comet = 3,
    Phoenix = 4,
}
