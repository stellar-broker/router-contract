use crate::adapters::soroswap::calc_soroswap_amount_out;

#[test]
fn get_soroswap_amount_out_test() {
    let reserves = (190104976848, 198442923346);
    let mut amount = calc_soroswap_amount_out(18920, &reserves, false);
    assert_eq!(amount, 19690);
    amount = calc_soroswap_amount_out(19690, &reserves, true);
    assert_eq!(amount, 18805);
}