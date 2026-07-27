use expect_test::expect;

use super::{assert_ansilove_parity, assert_ansilove_signal_parity};

#[test]
fn temporary_directory_initialization_creates_a_real_writable_directory_and_is_idempotent() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name "ansilove-init" temporary-file-directory))
        (ansilove-temporary-directory (file-name-as-directory directory)))
  (when (file-exists-p directory)
    (delete-directory directory t))
  (unwind-protect
      (list
       (file-exists-p directory)
       (ansilove--init-temporary-directory)
       (file-directory-p directory)
       (file-writable-p directory)
       (ansilove--init-temporary-directory)
       (file-directory-p directory))
    (delete-directory directory t)))"##;
    let expect = expect!["OK (nil nil t t nil t)"];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn temporary_directory_initialization_reports_the_exact_unwritable_directory() {
    let elisp_form = r##"(let ((ansilove-temporary-directory
       (file-name-as-directory
        (expand-file-name "ansilove-read-only" temporary-file-directory))))
  (cl-letf (((symbol-function 'file-exists-p) (lambda (_file) t))
            ((symbol-function 'file-writable-p) (lambda (_file) nil))
            ((symbol-function 'make-directory)
             (lambda (&rest _arguments)
               (error "must not create an existing directory"))))
    (ansilove--init-temporary-directory)))"##;
    let expect = expect![[
        r#"ERR (user-error "Fatal error: The directory [ORACLE-TMPDIR]/ansilove-read-only/ is not writable!")"#
    ]];
    assert_ansilove_signal_parity(elisp_form, expect);
}

#[test]
fn cleanup_removes_png_and_txt_recursively_while_preserving_unrelated_files() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name "ansilove-clean" temporary-file-directory))
        (nested (expand-file-name "nested/deeper" directory))
        (ansilove-temporary-directory (file-name-as-directory directory)))
  (make-directory nested t)
  (dolist (entry
           '(("root.png" . "png")
             ("root.txt" . "text")
             ("root.ans" . "ansi")
             ("nested/deeper/image.png" . "nested png")
             ("nested/deeper/input.txt" . "nested text")
             ("nested/deeper/notes.md" . "keep")))
    (with-temp-file (expand-file-name (car entry) directory)
      (insert (cdr entry))))
  (unwind-protect
      (list
       (sort
        (mapcar
         (lambda (file) (file-relative-name file directory))
         (directory-files-recursively directory ".*" nil))
        #'string-lessp)
       (ansilove-clean-temporary-directory)
       (sort
        (mapcar
         (lambda (file) (file-relative-name file directory))
         (directory-files-recursively directory ".*" nil))
        #'string-lessp)
       (with-temp-buffer
         (insert-file-contents (expand-file-name "root.ans" directory))
         (buffer-string))
       (with-temp-buffer
         (insert-file-contents (expand-file-name "nested/deeper/notes.md" directory))
         (buffer-string)))
    (delete-directory directory t)))"##;
    let expect = expect![[
        r#"OK (("nested/deeper/image.png" "nested/deeper/input.txt" "nested/deeper/notes.md" "root.ans" "root.png" "root.txt") ("[ORACLE-TMPDIR]/ansilove-clean/nested/deeper/image.png" "[ORACLE-TMPDIR]/ansilove-clean/nested/deeper/input.txt" "[ORACLE-TMPDIR]/ansilove-clean/root.png" "[ORACLE-TMPDIR]/ansilove-clean/root.txt") ("nested/deeper/notes.md" "root.ans") "ansi" "keep")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn cleanup_of_a_missing_directory_emits_a_warning_without_creating_it() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name "ansilove-clean-missing" temporary-file-directory))
        (ansilove-temporary-directory (file-name-as-directory directory))
        messages)
  (when (file-exists-p directory)
    (delete-directory directory t))
  (cl-letf (((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (push (apply #'format format-string arguments) messages))))
    (list
     (ansilove-clean-temporary-directory)
     (nreverse messages)
     (file-exists-p directory))))"##;
    let expect = expect![[
        r#"OK (#1=("Warning: The directory [ORACLE-TMPDIR]/ansilove-clean-missing/ does not exist.") #1# nil)"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}
