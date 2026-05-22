//! Divergence tests: window, frame, display, and face attributes.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_window_basic_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((w (selected-window)))
  (list (windowp w)
        (window-live-p w)
        (window-buffer w)
        (window-point w)
        (window-start w)
        (window-valid-p w)))"#,
    );
}

#[test]
fn divergence_window_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((w (selected-window)))
  (list (> (window-total-width w) 0)
        (> (window-total-height w) 0)
        (> (window-body-width w) 0)
        (> (window-body-height w) 0)
        (>= (window-hscroll w) 0)))"#,
    );
}

#[test]
fn divergence_window_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((w (selected-window)))
  (list (consp (window-edges w))
        (consp (window-inside-edges w))
        (length (window-edges w))))"#,
    );
}

#[test]
fn divergence_window_configuration() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((cfg (current-window-configuration)))
  (list (window-configuration-p cfg)
        (window-configuration-frame cfg)))"#,
    );
}

#[test]
fn divergence_frame_basic_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((f (selected-frame)))
  (list (framep f)
        (frame-live-p f)
        (frame-visible-p f)
        (stringp (frame-parameter f 'name))
        (frame-parameter f 'minibuffer)))"#,
    );
}

#[test]
fn divergence_frame_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((f (selected-frame)))
  (list (consp (frame-parameters f))
        (assq 'name (frame-parameters f))
        (assq 'foreground-color (frame-parameters f))
        (assq 'background-color (frame-parameters f))))"#,
    );
}

#[test]
fn divergence_face_attribute_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (facep 'default)
  (facep 'bold)
  (facep 'nonexistent-face-xyz)
  (consp (face-all-attributes 'default (selected-frame))))"#,
    );
}

#[test]
fn divergence_face_attribute_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (face-attribute 'default :family (selected-frame))
  (face-attribute 'default :height (selected-frame))
  (face-attribute 'default :weight (selected-frame)))"#,
    );
}

#[test]
fn divergence_display_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "before")
  (put-text-property (point-min) (point-max) 'display '(image :type xpm))
  (list (get-text-property (point-min) 'display)
        (buffer-string)))"#,
    );
}

#[test]
fn divergence_invisible_text_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "visible")
  (insert (propertize "hidden" 'invisible t))
  (insert "visible2")
  (list (get-text-property 7 'invisible (current-buffer))
        (get-text-property 9 'invisible (current-buffer))
        (get-text-property 15 'invisible (current-buffer))
        (length (buffer-string))))"#,
    );
}

#[test]
fn divergence_line_beginning_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "line1\nline2\nline3")
  (goto-char 1)
  (list (line-beginning-position)
        (line-end-position)
        (progn (forward-line 1) (line-beginning-position))
        (progn (forward-line 1) (line-beginning-position))
        (line-number-at-pos)))"#,
    );
}

#[test]
fn divergence_point_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World")
  (list (point-min)
        (point-max)
        (point)
        (bobp)
        (eobp)
        (progn (goto-char 5) (point))
        (bolp)
        (eolp)))"#,
    );
}
