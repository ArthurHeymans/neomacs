//! Default key-binding divergence probes.
//!
//! Probes whether Neomacs' default global-map key bindings match GNU's.
//! Each test lists lookup-key results for a group of common default bindings;
//! differences surface which default bindings diverge (missing, or bound to a
//! different command).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_db_cursor_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map "\C-a")
      (lookup-key global-map "\C-e")
      (lookup-key global-map "\C-f")
      (lookup-key global-map "\C-b")
      (lookup-key global-map "\C-n")
      (lookup-key global-map "\C-p")
      (lookup-key global-map "\M-f")
      (lookup-key global-map "\M-b"))
"##,
    );
}

#[test]
fn div_db_editing_kill_yank() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map "\C-d")
      (lookup-key global-map "\C-k")
      (lookup-key global-map "\C-y")
      (lookup-key global-map "\C-w")
      (lookup-key global-map "\M-w")
      (lookup-key global-map "\M-d")
      (lookup-key (current-global-map) "\C-_"))
"##,
    );
}

#[test]
fn div_db_search_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map "\C-s")
      (lookup-key global-map "\C-r")
      (lookup-key global-map "\M-%")
      (lookup-key global-map [?\M-%]))
"##,
    );
}

#[test]
fn div_db_scroll_buffer_navigation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map "\C-v")
      (lookup-key global-map "\M-v")
      (lookup-key global-map "\M-<")
      (lookup-key global-map "\M->")
      (lookup-key global-map "\C-xb")
      (lookup-key global-map "\C-xk")
      (lookup-key global-map "\C-x\C-f")
      (lookup-key global-map "\C-x\C-s"))
"##,
    );
}

#[test]
fn div_db_prefix_and_misc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map "\C-x")
      (lookup-key global-map "\C-c")
      (lookup-key global-map "\C-h")
      (lookup-key global-map "\M-x")
      (lookup-key global-map "\C-g")
      (lookup-key global-map "\C-u")
      (lookup-key global-map "\C-x\C-c"))
"##,
    );
}

#[test]
fn div_db_self_insert_and_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map "a")
      (lookup-key global-map " ")
      (lookup-key global-map "\r")
      (lookup-key global-map "\t")
      (lookup-key global-map "\d")
      (lookup-key global-map "\e"))
"##,
    );
}

#[test]
fn div_db_function_and_arrow_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (lookup-key global-map [left])
      (lookup-key global-map [right])
      (lookup-key global-map [up])
      (lookup-key global-map [down])
      (lookup-key global-map [home])
      (lookup-key global-map [end])
      (lookup-key global-map [prior])
      (lookup-key global-map [next]))
"##,
    );
}

#[test]
fn div_db_ctl_x_map_common() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ctlx (lookup-key global-map "\C-x")))
  (list (lookup-key ctlx "o")
        (lookup-key ctlx "0")
        (lookup-key ctlx "1")
        (lookup-key ctlx "2")
        (lookup-key ctlx "s")
        (lookup-key ctlx "i")))
"##,
    );
}
