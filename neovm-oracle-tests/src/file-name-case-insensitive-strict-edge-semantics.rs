//! Oracle parity tests for GNU file-name case-sensitivity probing.
//!
//! GNU implements `file-name-case-insensitive-p` in `src/fileio.c`.  Missing
//! paths are not immediate errors: GNU walks upward to an existing parent and
//! reports that filesystem's case behavior, returning nil if it cannot decide.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_name_case_insensitive_existing_and_missing_paths() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((root (make-temp-file "neomacs-oracle-file-name-case-" t))
       (sub (expand-file-name "SubDir" root))
       (file (expand-file-name "MixedName.txt" sub))
       (missing-child (expand-file-name "Missing/Child.txt" sub)))
  (unwind-protect
      (progn
        (make-directory sub)
        (write-region "x" nil file nil 'silent)
        (list
         ;; Existing files and directories should use the same filesystem
         ;; probe result.
         (file-name-case-insensitive-p root)
         (file-name-case-insensitive-p sub)
         (file-name-case-insensitive-p file)
         ;; Missing descendants walk upward to the existing parent.
         (file-name-case-insensitive-p missing-child)
         (equal (file-name-case-insensitive-p missing-child)
                (file-name-case-insensitive-p sub))
         ;; Relative names are expanded before probing.
         (let ((default-directory sub))
           (equal (file-name-case-insensitive-p "MixedName.txt")
                  (file-name-case-insensitive-p file)))
         (condition-case err
             (file-name-case-insensitive-p)
           (error (list (car err) (cdr err))))
         (condition-case err
             (file-name-case-insensitive-p file 'extra)
           (error (list (car err) (cdr err))))
         (condition-case err
             (file-name-case-insensitive-p 42)
           (error (list (car err) (cdr err))))))
    (ignore-errors (delete-directory root t))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
