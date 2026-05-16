//! Oracle parity tests for GNU base64 primitive semantics.
//!
//! GNU implements these primitives in `src/fns.c`.  In particular,
//! `Fbase64_decode_string` accepts `(STRING &optional BASE64URL IGNORE-INVALID)`;
//! URL-safe decoding is selected by the second argument and invalid input is
//! ignored only when the third argument is non-nil.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_base64_decode_string_url_and_ignore_invalid_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (base64url-encode-string "a+b/" t)
 (base64-decode-string (base64url-encode-string "a+b/" t) t)
 (condition-case err
     (base64-decode-string "!!!!")
   (error (list (car err) (cdr err))))
 (base64-decode-string "!!!!" nil t))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
