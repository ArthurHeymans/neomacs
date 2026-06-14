//! Overlay & text-property (propertize) divergence probes.
//!
//! Probes overlay lifecycle (make/delete/move), overlay-put/get, overlays-at/in
//! and priority ordering, before/after-string, evaporate, overlay-lists; and
//! text properties (propertize, put/get/set/add/remove-text-properties,
//! next/previous-property-change, stickiness). Areas where a display-metadata
//! reimplementation commonly diverges from GNU.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- propertize -------------------------------------------------------------

#[test]
fn div_prop_propertize_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r#"(propertize "hello" 'face 'bold 'font-lock-face 'keyword)"#);
}

#[test]
fn div_prop_propertize_text_properties_at() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s (propertize "x" 'a 1 'b 2)))
  (list s (text-properties-at 0 s) (length s)))
"#,
    );
}

#[test]
fn div_prop_set_text_properties_add_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abcdef")
  (set-text-properties 1 3 (list 'face 'bold))
  (add-text-properties 3 5 (list 'font-lock-face 'keyword))
  (let ((p1 (text-properties-at 1))
        (p3 (text-properties-at 3))
        (p5 (text-properties-at 5)))
    (remove-text-properties 1 6 '(face))
    (list p1 p3 p5 (text-properties-at 1) (text-properties-at 3))))
"#,
    );
}

#[test]
fn div_prop_next_previous_property_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abcdefgh")
  (put-text-property 2 4 'p 'v)
  (list (next-property-change 1)
        (next-single-property-change 1 'p)
        (previous-property-change 6)
        (previous-single-property-change 6 'p)))
"#,
    );
}

#[test]
fn div_prop_property_search_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abcdefgh")
  (put-text-property 3 6 'face 'bold)
  (list (text-property-any 1 8 'face 'bold)
        (text-property-not-all 1 8 'face 'bold)
        (text-property-any 1 3 'face 'bold)))
"#,
    );
}

#[test]
fn div_prop_stickiness_explicit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 2 4 'rear-sticky nil)
  (put-text-property 2 4 'front-sticky t)
  (list (get-text-property 2 'rear-sticky)
        (get-text-property 2 'front-sticky)
        (text-properties-at 2)))
"#,
    );
}

#[test]
fn div_prop_insert_inherits_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Inserting at a boundary inherits neighboring rear/front-sticky props.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 1 4 'face 'bold)
  (goto-char 3)
  (insert "X")
  (list (get-text-property 3 'face) (get-text-property 4 'face)))
"#,
    );
}

// --- overlay lifecycle & properties -----------------------------------------

#[test]
fn div_ov_make_put_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello world")
  (let ((ov (make-overlay 2 5)))
    (overlay-put ov 'face 'bold)
    (overlay-put ov 'priority 5)
    (list (overlayp ov)
          (overlay-start ov) (overlay-end ov)
          (overlay-get ov 'face)
          (overlay-get ov 'priority)
          (overlay-buffer ov))))
"#,
    );
}

#[test]
fn div_ov_move_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello world")
  (let ((ov (make-overlay 2 5)))
    (move-overlay ov 6 9)
    (let ((s1 (overlay-start ov)) (e1 (overlay-end ov)))
      (delete-overlay ov)
      (list s1 e1 (overlays-at 3) (overlays-at 7)))))
"#,
    );
}

#[test]
fn div_ov_priority_ordering_at() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // overlays-at returns overlays sorted by priority (highest first).
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello world")
  (let ((o1 (make-overlay 2 5)) (o2 (make-overlay 3 6)) (o3 (make-overlay 4 7)))
    (overlay-put o1 'priority 1)
    (overlay-put o2 'priority 3)
    (overlay-put o3 'priority 2)
    (list (length (overlays-at 4))
          (length (overlays-in 2 6))
          (mapcar (lambda (o) (overlay-get o 'priority)) (overlays-at 4)))))
"#,
    );
}

#[test]
fn div_ov_before_after_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello")
  (let ((ov (make-overlay 2 4)))
    (overlay-put ov 'after-string ">>")
    (overlay-put ov 'before-string "<<")
    (list (overlay-get ov 'after-string)
          (overlay-get ov 'before-string))))
"#,
    );
}

#[test]
fn div_ov_lists_and_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello world")
  (make-overlay 2 5)
  (make-overlay 7 9)
  (let ((ls (overlay-lists)))
    (list (length (car ls)) (length (cdr ls))
          (length (overlays-in 1 20)))))
"#,
    );
}

#[test]
fn div_ov_overlay_at_endpoints() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello world")
  (let ((ov (make-overlay 3 6)))
    (list (member ov (overlays-at 3))
          (member ov (overlays-at 5))
          (member ov (overlays-at 6))
          (overlays-in 6 9))))
"#,
    );
}

#[test]
fn div_ov_evaporate_on_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // An overlay with evaporate=t and zero length should be deleted.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello")
  (let ((ov (make-overlay 3 3)))
    (overlay-put ov 'evaporate t)
    (list (overlay-start ov) (overlay-end ov) (overlayp ov)
          (progn (delete-region 3 4) (overlay-start ov)))))
"#,
    );
}

#[test]
fn div_ov_priority_default_and_neg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "hello world")
  (let ((o1 (make-overlay 2 5)) (o2 (make-overlay 2 5)))
    (overlay-put o1 'priority -5)
    (overlay-put o2 'priority 0)
    (mapcar (lambda (o) (overlay-get o 'priority)) (overlays-at 3))))
"#,
    );
}
