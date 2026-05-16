//! Oracle parity tests for GNU `remember-mouse-glyph` argument semantics.
//!
//! GNU implements this in `src/xdisp.c`: it decodes a live window-system frame
//! and checks X/Y are fixnums before returning glyph extents.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_remember_mouse_glyph_validates_frame_and_coordinates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (remember-mouse-glyph "not-a-frame" nil nil)
   (error (cons (car err) (cdr err))))
 (condition-case err
     (remember-mouse-glyph (selected-frame) nil nil)
   (error (cons (car err) (cdr err))))
 (condition-case err
     (remember-mouse-glyph (selected-frame) 0 nil)
   (error (cons (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
