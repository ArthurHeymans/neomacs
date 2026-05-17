//! Oracle parity tests for GNU `file-attributes` edge semantics.
//!
//! GNU implements `file-attributes` in `src/dired.c`.  It condition-catches
//! `expand-file-name`, returns nil if expansion fails or the file does not
//! exist, follows GNU's ID-FORMAT rule for uid/gid representation, and
//! `file-attributes-lessp` compares the car strings of directory entries.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_file_attributes_shape_id_format_missing_bad_filename_and_lessp_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((dir (make-temp-file "neomacs-oracle-fileattrs-" t)))
  (unwind-protect
      (progn
        (make-directory (expand-file-name "sub" dir))
        (with-temp-file (expand-file-name "alpha.txt" dir)
          (insert "alpha"))
        (let* ((file (expand-file-name "alpha.txt" dir))
               (sub (expand-file-name "sub" dir))
               (missing (expand-file-name "missing" dir))
               (int-attrs (file-attributes file 'integer))
               (string-attrs (file-attributes file 'string))
               (dir-attrs (file-attributes sub 'integer)))
          (list
           (list
            (nth 0 int-attrs)
            (integerp (nth 1 int-attrs))
            (integerp (nth 2 int-attrs))
            (integerp (nth 3 int-attrs))
            (consp (nth 4 int-attrs))
            (consp (nth 5 int-attrs))
            (consp (nth 6 int-attrs))
            (nth 7 int-attrs)
            (substring (nth 8 int-attrs) 0 1)
            (nth 9 int-attrs)
            (integerp (nth 10 int-attrs))
            (integerp (nth 11 int-attrs))
            (length int-attrs))
           (list
            (or (stringp (nth 2 string-attrs)) (integerp (nth 2 string-attrs)))
            (or (stringp (nth 3 string-attrs)) (integerp (nth 3 string-attrs))))
           (list (nth 0 dir-attrs)
                 (substring (nth 8 dir-attrs) 0 1)
                 (length dir-attrs))
           (file-attributes missing 'integer)
           (file-attributes nil)
           (condition-case err
               (file-attributes 42)
             (error (list (car err) (cdr err))))
           (condition-case err
               (file-attributes (string ?a 0 ?b))
             (error (list (car err) (cdr err))))
           (file-attributes-lessp (cons "alpha" int-attrs)
                                  (cons "beta" int-attrs))
           (file-attributes-lessp (cons "beta" int-attrs)
                                  (cons "alpha" int-attrs))
           (condition-case err
               (file-attributes-lessp "alpha" "beta")
             (error (list (car err) (cdr err)))))))
    (delete-directory dir t)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
