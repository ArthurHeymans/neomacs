//! Strict combo oracle probes, batch 323: keyboard macro execution.
//! execute-kbd-macro over a vector of events, kmacro-start/end/execute,
//! and kmacro-counter insertion.
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_execute_kbd_macro_vector_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-temp-buffer
  (execute-kbd-macro (vconcat "hello"))
  (list (buffer-string)
        (point)
        (length (buffer-string))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_kmacro_counter_call_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'kmacro)
(with-temp-buffer
  (setq kmacro-counter 0)
  (setq last-kbd-macro (vconcat "A" [?\C-x ?\C-k ?\C-i] "B"))
  (execute-kbd-macro last-kbd-macro)
  (list (> (length (buffer-string)) 0)
        (buffer-string)
        kmacro-counter))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_execute_kbd_macro_keys_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-temp-buffer
  (execute-kbd-macro (vconcat "ab") 3)
  (list (buffer-string)
        (= (length (buffer-string)) 6)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
