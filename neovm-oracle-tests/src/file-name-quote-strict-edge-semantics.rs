//! Oracle parity tests for GNU file name quote helpers.
//!
//! GNU implements `file-name-quoted-p`, `file-name-quote`, and
//! `file-name-unquote` in `lisp/files.el`.  For local names these helpers are
//! exact string operations around the `/:` prefix, including the special
//! unquote result for the bare `/:` name.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_name_quote_unquote_and_quoted_p_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (mapcar (lambda (name)
           (list name
                 (file-name-quoted-p name)
                 (file-name-quoted-p name t)
                 (file-name-quote name)
                 (file-name-quote name t)
                 (file-name-unquote name)
                 (file-name-unquote name t)))
         '("" "/" "/:" "/:/" "/:/tmp/a" "/tmp/a"
           "relative" "/::literal" "/:/already/quoted"))
 (condition-case err
     (file-name-quoted-p 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-quote 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-unquote 42)
   (error (list (car err) (cdr err))))
 (condition-case err
     (file-name-quote "x" nil nil)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
