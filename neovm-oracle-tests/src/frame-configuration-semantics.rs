//! Oracle parity tests for GNU `subr.el` `frame-configuration-p`.

use super::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn oracle_prop_frame_configuration_p_shape_contract() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el intentionally treats any cons whose car is
    // `frame-configuration` as a frame configuration, including dotted pairs.
    let form = r#"(mapcar #'frame-configuration-p
        (list nil
              t
              'frame-configuration
              '(frame-configuration)
              '(frame-configuration . payload)
              '(x frame-configuration)
              [frame-configuration]))"#;
    assert_oracle_parity(form);
}
