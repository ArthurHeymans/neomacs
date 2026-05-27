use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_babel_lob_ingest_call_execute_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-lob)
  (require 'ob-emacs-lisp)
  (let* ((root (make-temp-file "org-lob" t))
         (lib (expand-file-name "library.org" root))
         (org-babel-library-of-babel nil))
    (unwind-protect
        (progn
          (with-temp-file lib
            (insert "#+PROPERTY: header-args:emacs-lisp :results value replace\n")
            (insert "#+NAME: add-pair\n")
            (insert "#+begin_src emacs-lisp :var x=1 y=2\n")
            (insert "(list :sum (+ x y) :product (* x y))\n")
            (insert "#+end_src\n\n")
            (insert "#+NAME: decorate\n")
            (insert "#+begin_src emacs-lisp :var label=\"n\" values='(1 2)\n")
            (insert "(format \"%s=%S\" label values)\n")
            (insert "#+end_src\n"))
          (let ((ingested (org-babel-lob-ingest lib)))
            (with-temp-buffer
              (org-mode)
              (insert "#+CALL: add-pair(x=6,y=7) :results value drawer replace\n\n")
              (insert "Prefix call_decorate[:results raw](label=\"nums\", values='(3 4)) suffix.\n")
              (let ((org-confirm-babel-evaluate nil)
                    (org-babel-default-lob-header-args '((:exports . "results"))))
                (goto-char (point-min))
                (let ((call-info (org-babel-lob-get-info)))
                  (org-babel-lob-execute-maybe)
                  (goto-char (point-min))
                  (search-forward "call_decorate")
                  (let ((inline-info (org-babel-lob-get-info)))
                    (org-babel-lob-execute-maybe)
                    (list ingested
                          (sort (mapcar (lambda (cell)
                                          (symbol-name (car cell)))
                                        org-babel-library-of-babel)
                                #'string<)
                          (nth 0 call-info)
                          (nth 1 call-info)
                          (cdr (assq :results (nth 2 call-info)))
                          (nth 0 inline-info)
                          (cdr (assq :results (nth 2 inline-info)))
                          (buffer-substring-no-properties
                           (point-min) (point-max)))))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_babel_ref_remote_table_headline_index_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-ref)
  (require 'ob-emacs-lisp)
  (require 'org-id)
  (let* ((root (make-temp-file "org-ref" t))
         (remote (expand-file-name "remote.org" root))
         (org-id-locations-file (expand-file-name ".org-id-locations" root))
         (org-id-track-globally t))
    (unwind-protect
        (progn
          (with-temp-file remote
            (insert "#+NAME: matrix\n")
            (insert "| row | a | b |\n")
            (insert "|-----+---+---|\n")
            (insert "| r1  | 1 | 2 |\n")
            (insert "| r2  | 3 | 4 |\n\n")
            (insert "* Remote headline\n")
            (insert ":PROPERTIES:\n:CUSTOM_ID: remote-head\n:END:\n")
            (insert "First body line.\nSecond body line.\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+NAME: local\n")
            (insert "| name | score |\n| ann | 5 |\n| bob | 8 |\n\n")
            (insert "#+begin_src emacs-lisp :var cell=local[2,1] :var row=local[1,*] :results value\n")
            (insert "(list cell row)\n")
            (insert "#+end_src\n")
            (let ((org-confirm-babel-evaluate nil))
              (goto-char (point-min))
              (search-forward "begin_src")
              (let ((info (org-babel-get-src-block-info)))
                (list
                 (assq :var (nth 2 info))
                 (org-babel-ref-resolve "local[1,*]")
                 (org-babel-ref-resolve "local[,1]")
                 (org-babel-ref-resolve
                  (concat remote ":matrix[2,1]"))
                 (org-babel-ref-resolve
                  (concat remote ":matrix[1:2,1:2]"))
                 (substring-no-properties
                  (org-babel-ref-resolve "remote-head")))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_babel_sbe_table_formula_literal_header_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (require 'ob-core)
  (require 'ob-table)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: combine\n")
    (insert "#+begin_src emacs-lisp :var label=\"\" :var n=0 :results value\n")
    (insert "(format \"%s:%s\" label (* n n))\n")
    (insert "#+end_src\n\n")
    (insert "| label | n | result |\n")
    (insert "|-------+---+--------|\n")
    (insert "| alpha | 3 |        |\n")
    (insert "| beta  | 4 |        |\n")
    (insert "#+TBLFM: $3='(org-sbe \"combine\" (label $$1) (n $2))\n")
    (let ((org-confirm-babel-evaluate nil))
      (goto-char (point-min))
      (search-forward "alpha")
      (org-table-recalculate-buffer-tables)
      (let ((after-first (buffer-substring-no-properties
                          (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "beta")
        (org-table-next-field)
        (delete-char 1)
        (insert "5")
        (org-table-recalculate-buffer-tables)
        (list after-first
              (org-babel-table-truncate-at-newline "line1\nline2")
              (org-babel-table-truncate-at-newline "single")
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_babel_local_call_table_cache_inline_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-lob)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (let ((org-confirm-babel-evaluate nil)
          (org-babel-default-lob-header-args
           '((:exports . "results") (:results . "replace"))))
      (org-mode)
      (insert "#+PROPERTY: header-args:emacs-lisp :results value replace\n")
      (insert "#+NAME: nums\n")
      (insert "| item | n |\n")
      (insert "|------+---|\n")
      (insert "| a    | 2 |\n")
      (insert "| b    | 5 |\n\n")
      (insert "#+NAME: shape\n")
      (insert "#+begin_src emacs-lisp :var rows=nums factor=1 :cache yes\n")
      (insert "(mapcar (lambda (row)\n")
      (insert "          (let ((label (car row))\n")
      (insert "                (n (string-to-number (cadr row))))\n")
      (insert "            (list label n (* factor n))))\n")
      (insert "        rows)\n")
      (insert "#+end_src\n\n")
      (insert "#+CALL: shape(rows=nums[2:3,*], factor=10) :results value table replace :cache yes\n\n")
      (insert "Inline call_shape[:results raw replace](rows=nums[2:2,*], factor=3) end.\n")
      (let (call-info call-noeval call-pos call-read inline-info inline-read
            after-first after-second no-info)
        (goto-char (point-min))
        (search-forward "#+CALL")
        (setq call-info (org-babel-lob-get-info)
              call-noeval (org-babel-lob-get-info nil t))
        (org-babel-execute-maybe)
        (setq call-pos (org-babel-where-is-src-block-result nil call-info))
        (goto-char call-pos)
        (forward-line 1)
        (setq call-read (org-babel-read-result))
        (setq after-first
              (buffer-substring-no-properties (point-min) (point-max)))
        (goto-char (point-min))
        (search-forward "call_shape")
        (setq inline-info (org-babel-lob-get-info))
        (org-babel-execute-maybe)
        (setq inline-read
              (save-excursion
                (goto-char (point-min))
                (search-forward "Inline")
                (buffer-substring-no-properties
                 (line-beginning-position) (line-end-position))))
        (goto-char (point-min))
        (search-forward "| b")
        (search-forward "5")
        (replace-match "7" t t)
        (goto-char (point-min))
        (search-forward "#+CALL")
        (org-babel-execute-maybe)
        (setq after-second
              (buffer-substring-no-properties (point-min) (point-max)))
        (goto-char (point-max))
        (setq no-info (org-babel-lob-get-info))
        (list (nth 0 call-info)
              (nth 1 call-info)
              (assq :var (nth 2 call-info))
              (assq :cache (nth 2 call-info))
              (assq :results (nth 2 call-info))
              (assq :exports (nth 2 call-info))
              (nth 4 call-info)
              (nth 5 call-info)
              (assq :var (nth 2 call-noeval))
              call-read
              after-first
              (nth 0 inline-info)
              (assq :var (nth 2 inline-info))
              (assq :results (nth 2 inline-info))
              inline-read
              after-second
              no-info)))))"##,
    );
}
