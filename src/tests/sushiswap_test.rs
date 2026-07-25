use crate::adapters::sushiswap::{max_sqrt_ratio, min_sqrt_ratio};
use soroban_sdk::{Bytes, Env};

#[test]
fn max_sqrt_ratio_encodes_correctly() {
    // MAX_SQRT_RATIO - 1 as a 32-byte big-endian array
    let env = Env::default();
    let expected = Bytes::from_array(
        &env,
        &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 253, 137, 99, 239, 209, 252, 106, 80, 100,
            136, 73, 93, 149, 29, 82, 99, 152, 141, 37,
        ],
    );
    assert_eq!(max_sqrt_ratio(&env).to_be_bytes(), expected);
}

#[test]
fn min_sqrt_ratio_encodes_correctly() {
    // MIN_SQRT_RATIO + 1 = 4295128740 as a 32-byte big-endian array
    let env = Env::default();
    let expected = Bytes::from_array(
        &env,
        &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 2, 118, 164,
        ],
    );
    assert_eq!(min_sqrt_ratio(&env).to_be_bytes(), expected);
}