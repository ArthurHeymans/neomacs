//! Case-table (case-table-p, unicode upcase/downcase/capitalize, case-fold
//! search), buffer modification ticks / modified-p / swap-text / new-buffer-
//! name, and marker/overlay edge (other-buffer marker, evaporate, overlay
//! changes, remove-overlays, marker in narrowed region); plus the custom
//! case-syntax-pair divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn ct_case_fold_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "Hello WORLD")
  (let ((case-fold-search t)) (goto-char (point-min))
    (list (re-search-forward "world" nil t)
          (progn (goto-char (point-min)) (search-forward "HELLO" nil t)))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: set-case-syntax-pair on a custom case table is ignored by upcase/downcase, which use the standard mapping regardless (a custom { <-> } pairing has no effect; GNU honors it)."]
fn divergence_case_table_custom_pair() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((tbl (copy-case-table (standard-case-table))))
    (set-case-syntax-pair ?{ ?} tbl)
    (with-current-buffer (current-buffer)
      (set-case-table tbl)
      (list (upcase ?}) (downcase ?{) (upcase "a}c") (downcase "A{C")))))"##,
    );
}

#[test]
fn ct_case_table_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (case-table-p (standard-case-table))
        (case-table-p (current-case-table))
        (char-equal ?a ?A))"##,
    );
}

#[test]
fn ct_upcase_downcase_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (upcase "ßæ") (downcase "ÆÇ") (capitalize "ﬁle")
        (upcase ?ﬀ) (upcase-initials "hello-world test"))"##,
    );
}

#[test]
fn bk_buffer_swap_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((b1 (generate-new-buffer " neo-bsw1-xxx")) (b2 (generate-new-buffer " neo-bsw2-xxx")))
  (with-current-buffer b1 (insert "AAA"))
  (with-current-buffer b2 (insert "BBB"))
  (with-current-buffer b1 (buffer-swap-text b2))
  (prog1 (list (with-current-buffer b1 (buffer-string)) (with-current-buffer b2 (buffer-string)))
    (kill-buffer b1) (kill-buffer b2)))"##,
    );
}

#[test]
fn bk_gen_buffer_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((b (generate-new-buffer "neo-gbn-xxx")))
  (prog1 (list (generate-new-buffer-name "neo-gbn-xxx")
               (string-prefix-p "neo-gbn-xxx" (generate-new-buffer-name "neo-gbn-xxx")))
    (kill-buffer b)))"##,
    );
}

#[test]
fn bk_modified_p_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "x")
  (let ((m1 (buffer-modified-p)))
    (set-buffer-modified-p nil)
    (let ((m2 (buffer-modified-p)))
      (restore-buffer-modified-p t)
      (list m1 m2 (buffer-modified-p)))))"##,
    );
}

#[test]
fn bk_modified_ticks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (let ((t0 (buffer-modified-tick)))
    (insert "abc")
    (let ((t1 (buffer-modified-tick)))
      (insert "def")
      (list (> t1 t0) (> (buffer-modified-tick) t1) (buffer-chars-modified-tick)
            (> (buffer-chars-modified-tick) 0)))))"##,
    );
}

#[test]
fn mo_marker_in_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "0123456789")
  (let ((m (copy-marker 8)))
    (narrow-to-region 2 6)
    (prog1 (list (marker-position m) (= m 8))
      (widen))))"##,
    );
}

#[test]
fn mo_marker_other_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((b (generate-new-buffer " neo-mob-xxx")))
  (with-current-buffer b (insert "0123456789"))
  (let ((m (make-marker)))
    (set-marker m 4 b)
    (prog1 (list (marker-position m) (eq (marker-buffer m) b)
                 (progn (set-marker m nil) (marker-position m)))
      (kill-buffer b))))"##,
    );
}

#[test]
fn mo_overlay_changes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "0123456789")
  (let ((o1 (make-overlay 2 4)) (o2 (make-overlay 6 8)))
    (list (next-overlay-change 1) (next-overlay-change 4)
          (previous-overlay-change 9) (length (overlays-in 1 11)))))"##,
    );
}

#[test]
fn mo_overlay_evaporate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "0123456789")
  (let ((ov (make-overlay 3 6)))
    (overlay-put ov 'evaporate t)
    (delete-region 3 6)
    (list (overlay-buffer ov) (overlay-start ov))))"##,
    );
}

#[test]
fn mo_remove_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "0123456789")
  (make-overlay 1 3) (make-overlay 4 6)
  (let ((ov3 (make-overlay 7 9))) (overlay-put ov3 'keep t))
  (remove-overlays (point-min) (point-max) 'keep nil)
  (list (length (overlays-in (point-min) (point-max)))))"##,
    );
}
