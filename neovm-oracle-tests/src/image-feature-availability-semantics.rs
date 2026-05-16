//! Oracle parity tests for GNU image feature-gated primitives.
//!
//! GNU registers `imagemagick-types` in `src/image.c` only under
//! `HAVE_IMAGEMAGICK`, and `lookup-image` only under `GLYPH_DEBUG`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_image_primitives_follow_gnu_build_feature_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (fboundp 'imagemagick-types)
 (condition-case err
     (imagemagick-types)
   (error (cons (car err) (cdr err))))
 (fboundp 'lookup-image)
 (condition-case err
     (lookup-image nil)
   (error (cons (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
