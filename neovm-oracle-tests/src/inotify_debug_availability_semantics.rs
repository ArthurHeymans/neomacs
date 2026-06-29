//! Oracle parity tests for GNU inotify debug helper availability.
//!
//! GNU implements `inotify-watch-list` and `inotify-allocated-p` in
//! `src/inotify.c` only under `INOTIFY_DEBUG`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_inotify_debug_helpers_follow_gnu_build_feature_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (fboundp 'inotify-watch-list)
 (condition-case err
     (inotify-watch-list)
   (error (cons (car err) (cdr err))))
 (fboundp 'inotify-allocated-p)
 (condition-case err
     (inotify-allocated-p)
   (error (cons (car err) (cdr err)))))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK (nil (void-function inotify-watch-list) nil (void-function inotify-allocated-p))""#
        ]],
    );
}
