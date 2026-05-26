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
