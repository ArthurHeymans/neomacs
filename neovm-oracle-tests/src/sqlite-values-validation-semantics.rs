//! Oracle parity tests for GNU SQLite VALUES validation semantics.
//!
//! GNU implements this in `src/sqlite.c`: `sqlite-execute` and
//! `sqlite-select` signal `sqlite-error` for malformed VALUES arguments and
//! unsupported bind value types.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_sqlite_values_validation_signals_sqlite_error_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (sqlite-available-p)
 (condition-case err
     (let ((db (sqlite-open)))
       (sqlite-execute db "select ?" 9))
   (error (cons (car err) (cdr err))))
 (condition-case err
     (let ((db (sqlite-open)))
       (sqlite-select db "select ?" 9))
   (error (cons (car err) (cdr err))))
 (condition-case err
     (let ((db (sqlite-open)))
       (sqlite-execute db "select ?" (vector (cons 1 2))))
   (error (cons (car err) (cdr err)))))
"#;

    assert_oracle_parity(form);
}
