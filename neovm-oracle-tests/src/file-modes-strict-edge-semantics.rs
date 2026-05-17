//! Oracle parity tests for GNU file mode edge semantics.
//!
//! GNU implements `file-modes` and `set-file-modes` in `src/fileio.c`.
//! Their optional FLAG handling goes through `symlink_nofollow_flag`, where
//! any non-nil flag means nofollow; it is not restricted to the symbol
//! `nofollow`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_modes_non_nil_flag_means_nofollow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((dir (make-temp-file "neomacs-oracle-file-modes-" t))
       (target (expand-file-name "target" dir))
       (link (expand-file-name "link" dir)))
  (unwind-protect
      (progn
        (write-region "x" nil target nil 'silent)
        (set-file-modes target #o600)
        (make-symbolic-link target link)
        (list
         (logand (file-modes target) #o7777)
         (file-modes (expand-file-name "missing" dir))
         ;; GNU fileio.c:symlink_nofollow_flag treats any non-nil flag
         ;; as nofollow, so t and an arbitrary symbol must match 'nofollow.
         (= (file-modes link t)
            (file-modes link 'nofollow))
         (= (file-modes link 'anything-non-nil)
            (file-modes link 'nofollow))
         ;; The ordinary follow path still sees the target mode.
         (logand (file-modes link nil) #o7777)
         (condition-case err
             (file-modes)
           (error (list (car err) (cdr err))))
         (condition-case err
             (file-modes target 'nofollow 'extra)
           (error (list (car err) (cdr err))))))
    (ignore-errors (delete-file link))
    (ignore-errors (delete-file target))
    (ignore-errors (delete-directory dir))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
