//! Oracle parity tests for GNU file access and ownership helper semantics.
//!
//! GNU implements `access-file` and `file-accessible-directory-p` in
//! `src/fileio.c`, while `file-ownership-preserved-p` is Lisp in
//! `lisp/files.el` layered on `file-attributes`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_accessible_access_file_and_ownership_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((dir (make-temp-file "neomacs-oracle-file-access-" t)))
  (unwind-protect
      (progn
        (make-directory (expand-file-name "sub" dir))
        (with-temp-file (expand-file-name "alpha.txt" dir)
          (insert "alpha"))
        (let ((file (expand-file-name "alpha.txt" dir))
              (sub (expand-file-name "sub" dir))
              (missing (expand-file-name "missing" dir)))
          (list
           (access-file file "Reading alpha")
           (file-accessible-directory-p dir)
           (file-accessible-directory-p (file-name-as-directory dir))
           (file-accessible-directory-p sub)
           (file-accessible-directory-p file)
           (file-accessible-directory-p missing)
           (file-accessible-directory-p "")
           (file-ownership-preserved-p file)
           (file-ownership-preserved-p file t)
           (file-ownership-preserved-p missing)
           (condition-case err
               (access-file missing "Reading missing")
             (error (list (car err) (cadr err))))
           (condition-case err
               (access-file file 42)
             (error (list (car err) (cdr err))))
           (condition-case err
               (file-accessible-directory-p 42)
             (error (list (car err) (cdr err))))
           (condition-case err
               (file-ownership-preserved-p 42)
             (error (list (car err) (cdr err)))))))
    (delete-directory dir t)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
