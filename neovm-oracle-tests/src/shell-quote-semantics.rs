//! Oracle parity tests for GNU `subr.el` `shell-quote-argument`.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_prop_gnu_subr_shell_quote_argument_posix_contracts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el:shell-quote-argument POSIX quoting leaves only POSIX
    // filename characters unescaped, maps empty string to "''", and replaces
    // embedded newlines with the shell-safe quoted newline sequence.
    let form = r#"(mapcar
 (lambda (s)
   (list s
         (shell-quote-argument s t)
         (shell-quote-argument s)))
 (list ""
       "plain"
       "has space"
       "quote'single"
       "dollar$semi;pipe|"
       "line\nbreak"
       "[glob]*?"
       "back\\slash"
       "two\\\\slashes"
       "tab\tchar"
       "ümlaut"))"#;
    assert_oracle_parity_with_bootstrap(form);
}
