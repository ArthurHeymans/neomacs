use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_src_edit_switches_indent_writeback_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-src)
  (with-temp-buffer
    (org-mode)
    (insert "* Code\n")
    (insert "#+begin_src emacs-lisp -n -r :results value :exports both\n")
    (insert "  (let ((x 1))\n")
    (insert "    (+ x 2))\n")
    (insert "#+end_src\n")
    (goto-char (point-min))
    (search-forward "(let")
    (let ((before-info (org-babel-get-src-block-info)))
      (org-edit-src-code)
      (let ((edit-mode major-mode)
            (edit-before (buffer-substring-no-properties
                          (point-min) (point-max))))
        (goto-char (point-max))
        (insert ";; tail\n")
        (org-edit-src-exit)
        (goto-char (point-min))
        (search-forward "begin_src")
        (let ((element (org-element-at-point)))
          (list (nth 0 before-info)
                (nth 1 before-info)
                (cdr (assq :exports (nth 2 before-info)))
                edit-mode
                edit-before
                (org-element-property :switches element)
                (org-element-property :parameters element)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_babel_noweb_expand_export_processing_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-exp)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (insert "#+PROPERTY: header-args:emacs-lisp :results value replace\n")
    (insert "#+NAME: helper\n")
    (insert "#+begin_src emacs-lisp\n")
    (insert "(defun helper (x) (+ x 10))\n")
    (insert "#+end_src\n\n")
    (insert "#+begin_src emacs-lisp :noweb yes :exports both\n")
    (insert "<<helper>>\n")
    (insert "(helper 5)\n")
    (insert "#+end_src\n")
    (goto-char (point-min))
    (search-forward ":noweb")
    (let* ((info (org-babel-get-src-block-info))
           (expanded (org-babel-expand-src-block))
           (hash (org-babel-sha1-hash info :export))
           (exported-code (let ((org-babel-exp-reference-buffer (current-buffer)))
                            (org-babel-exp-code info 'block))))
      (list (cdr (assq :noweb (nth 2 info)))
            hash
            (not (null (string-match-p "(defun helper" expanded)))
            expanded
            exported-code))))"##,
    );
}

#[test]
fn org_babel_inline_and_block_result_replace_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: calc\n")
    (insert "#+begin_src emacs-lisp :results value replace drawer\n")
    (insert "(list 1 2 3)\n")
    (insert "#+end_src\n\n")
    (insert "Inline src_emacs-lisp[:results raw replace]{(+ 4 5)} end.\n")
    (let ((org-confirm-babel-evaluate nil))
      (org-babel-execute-buffer))
    (let ((after-first
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "(list 1 2 3)")
      (replace-match "(list 3 2 1)" t t)
      (goto-char (point-min))
      (search-forward "(+ 4 5)")
      (replace-match "(* 2 7)" t t)
      (org-babel-execute-buffer)
      (list after-first
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_src_preserve_indentation_save_abort_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-src)
  (with-temp-buffer
    (let ((org-src-preserve-indentation t)
          (org-edit-src-content-indentation 4)
          (org-src-window-setup 'current-window))
      (org-mode)
      (insert "* Code\n")
      (insert "#+begin_src emacs-lisp\n")
      (insert "    (message \"one\")\n")
      (insert "      (message \"two\")\n")
      (insert "#+end_src\n")
      (goto-char (point-min))
      (search-forward "one")
      (let (edit-before edit-after-save)
        (org-edit-src-code)
        (setq edit-before (buffer-substring-no-properties
                           (point-min) (point-max)))
        (goto-char (point-max))
        (insert "  (message \"saved\")\n")
        (org-edit-src-save)
        (setq edit-after-save
              (with-current-buffer (marker-buffer org-src--beg-marker)
                (buffer-substring-no-properties (point-min) (point-max))))
        (insert "  (message \"aborted\")\n")
        (org-edit-src-abort)
        (list edit-before
              edit-after-save
              (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_edit_special_example_and_src_modes_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-src)
  (with-temp-buffer
    (let ((org-src-window-setup 'current-window)
          (org-edit-fixed-width-region-mode 'fundamental-mode))
      (org-mode)
      (insert "* Mixed\n")
      (insert "#+begin_example\n")
      (insert "example line\n")
      (insert "#+end_example\n\n")
      (insert ": fixed\n: width\n\n")
      (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
      (let (example-mode example-text fixed-mode fixed-text src-mode)
        (goto-char (point-min))
        (search-forward "example line")
        (org-edit-special)
        (setq example-mode major-mode
              example-text (buffer-substring-no-properties
                            (point-min) (point-max)))
        (goto-char (point-max))
        (insert "example added\n")
        (org-edit-src-exit)
        (goto-char (point-min))
        (search-forward "fixed")
        (org-edit-special)
        (setq fixed-mode major-mode
              fixed-text (buffer-substring-no-properties
                          (point-min) (point-max)))
        (goto-char (point-max))
        (insert "third\n")
        (org-edit-src-exit)
        (goto-char (point-min))
        (search-forward "(+ 1 2)")
        (org-edit-special)
        (setq src-mode major-mode)
        (org-edit-src-abort)
        (list example-mode
              example-text
              fixed-mode
              fixed-text
              src-mode
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_babel_update_body_remove_result_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: calc\n")
    (insert "#+begin_src emacs-lisp :results value replace\n")
    (insert "(+ 1 2)\n")
    (insert "#+end_src\n")
    (insert "#+RESULTS: calc\n: 3\n\n")
    (insert "Inline src_emacs-lisp[:results raw replace]{(+ 2 3)} {{{results(=5=)}}}.\n")
    (goto-char (point-min))
    (search-forward "(+ 1 2)")
    (let ((info-before (org-babel-get-src-block-info)))
      (org-babel-update-block-body "(let ((x 4))\n  (+ x 6))")
      (let ((after-update
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-remove-result)
        (goto-char (point-min))
        (search-forward "src_emacs-lisp")
        (org-babel-remove-inline-result)
        (list (nth 1 info-before)
              after-update
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}
