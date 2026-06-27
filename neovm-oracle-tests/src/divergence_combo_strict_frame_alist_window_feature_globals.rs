//! Strict combo oracle probes, batch 24: frame alists (default/initial/
//! minibuffer), window-system and system-type, buffer-local-variable set,
//! featurep/feature list, and global ring/mark state.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_f9_frame_alist_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (length default-frame-alist)
      (assq 'menu-bar-lines default-frame-alist)
      (assq 'tool-bar-lines default-frame-alist)
      (length initial-frame-alist)
      minibuffer-frame-alist)
"##,
    );
}

#[test]
fn div_f9_window_system_and_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list window-system
      (condition-case err (window-system-version) (error (car err)))
      system-type
      (framep (selected-frame)))
"##,
    );
}

#[test]
fn div_f9_buffer_local_variables_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((bl (buffer-local-variables)))
    (list (> (length bl) 10)
          (assq 'fill-column bl)
          (local-variable-p 'fill-column)
          (local-variable-p 'buffer-file-name)
          (assq 'buffer-read-only bl))))
"##,
    );
}

#[test]
fn div_f9_featurep_and_feature_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (featurep 'emacs)
      (member 'emacs features)
      (> (length features) 50)
      (featurep 'png)
      (featurep 'jpeg)
      (featurep 'svg)
      (featurep 'rlimit))
"##,
    );
}

#[test]
fn div_f9_featurep_x() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK t
    // Neomacs:   OK nil
    // (featurep 'x) is t in this GNU Emacs build (compiled with X) but nil in
    // Neomacs; Neomacs does not advertise the `x' feature symbol, so Elisp
    // that guards GUI behavior on (featurep 'x) diverges.
    assert_oracle_parity(
        r##"
(featurep 'x)
"##,
    );
}

#[test]
fn div_f9_global_ring_and_mode_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (default-value 'mark-ring)
      global-mark-ring
      kill-ring-yank-pointer
      (default-value 'global-mode-string)))
"##,
    );
}

#[test]
fn div_f9_standard_alists_and_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (length auto-mode-alist)
      (length interpreter-mode-alist)
      (assq "\\.el\\'" auto-mode-alist)
      (length minor-mode-map-alist)
      (consp (default-value 'write-file-functions)))
"##,
    );
}
