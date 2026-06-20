//! Strong combo-complex-67/68 oracle tests — final comprehensive
//! probes: babel with ob-ref resolution chains, element with
//! org-element-create for keyword/types, org-export-to-file
//! with various backends, org-agenda prefix formatting, org-table
//! iterate stability, org-babel with org-babel-insert-result,
//! org-cycle with org-cycle-separator-lines, org-element with
//! affiliated on keyword elements, org-store-link with agenda,
//! and org-capture with template finalization.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn combo67_babel_ref_resolution_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'ob-emacs-lisp)
  (require 'ob-ref)
  (let ((org-confirm-babel-evaluate nil))
    (insert "#+name: step1\n| 5 |\n\n")
    (insert "#+name: step2\n| 3 |\n\n")
    (insert "#+begin_src emacs-lisp :results value :var a=step1 :var b=step2\n")
    (insert "(+ (car (car a)) (car (car b)))\n")
    (insert "#+end_src\n")
    (let ((r '()))
      (goto-char (point-min))
      (search-forward "#+begin_src emacs-lisp")
      (push (org-babel-execute-src-block) r)
      (push (list :result-count (length (org-element-map (org-element-parse-buffer) 'result #'identity))) r)
      (nreverse r))))"##,
    );
}

#[test]
fn combo67_element_create_for_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'org-element)
  (let* (;; create various keyword-like elements
         (kw (org-element-create 'keyword '(:key "TITLE" :value "My Title")))
         (affiliated (org-element-create 'keyword '(:key "CAPTION" :value "Fig 1")))
         (data (org-element-create 'org-data nil
                 (org-element-create 'section nil
                   kw
                   (org-element-create 'paragraph nil "Body.")
                   affiliated
                   (org-element-create 'table '(:type org)
                     (org-element-create 'table-row '(:type standard)
                       (org-element-create 'table-cell nil "X"))))))
         (interpreted (substring-no-properties (org-element-interpret-data data)))
         (r '()))
    (push (list :keyword-type (org-element-type kw)) r)
    (push (list :keyword-key (org-element-property :key kw)) r)
    (push (list :keyword-value (org-element-property :value kw)) r)
    (push (list :interpreted-length (> (length interpreted) 10)) r)
    ;; reparse
    (let ((reparsed (with-temp-buffer (org-mode) (insert interpreted)
                      (goto-char (point-min)) (org-element-parse-buffer))))
      (push (list :re-keywords (length (org-element-map reparsed 'keyword #'identity))) r)
      (push (list :re-tables (length (org-element-map reparsed 'table #'identity))) r))
    (nreverse r)))"##,
    );
}

#[test]
fn combo67_export_to_file_various_backends() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (require 'ox)
  (let ((tmpbase (make-temp-file "org-export-" nil)))
    (insert "* Export\nBody.\n")
    (let ((r '()))
      (dolist (pair '(("ascii" . ".txt") ("html" . ".html") ("latex" . ".tex")))
        (let ((outfile (concat tmpbase (cdr pair))))
          (condition-case nil
              (let ((backend (intern (car pair))))
                (org-export-to-file backend outfile)
                (push (list (intern (concat ":" (car pair) "-written"))
                            (file-exists-p outfile)) r)
                (condition-case nil (delete-file outfile) (error nil)))
            (error (push (list (intern (concat ":" (car pair) "-error")) t) r)))))
      (condition-case nil (delete-file tmpbase) (error nil))
      (nreverse r))))"##,
    );
}

#[test]
fn combo67_agenda_prefix_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-agenda)
  (list
   :prefix-fbound (boundp 'org-agenda-prefix-format)
   :default-prefix (when (boundp 'org-agenda-prefix-format)
                     org-agenda-prefix-format)
   :tags-prefix (when (boundp 'org-agenda-prefix-format)
                  (assq 'tags org-agenda-prefix-format))
   :todo-prefix (when (boundp 'org-agenda-prefix-format)
                  (assq 'todo org-agenda-prefix-format))
   ))"##,
    );
}

#[test]
fn combo67_table_iterate_stability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |\n")
  (insert "#+TBLFM: @>$1=vsum(@2..@-1)::$2=$1+0\n")
  (let ((r '()))
    (goto-char (point-min))
    ;; iterate to stability
    (condition-case nil
        (progn (org-table-iterate)
               (push (list :after-iterate (buffer-string)) r)
               (push (list :sum (org-table-get "@>$1" nil)) r)
               (push (list :to-lisp (org-table-to-lisp)) r))
      (error (push (list :iterate-error t) r)))
    (nreverse r)))"##,
    );
}

#[test]
fn combo67_cycle_separator_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (list
   :separator-lines-fbound (boundp 'org-cycle-separator-lines)
   :separator-lines (when (boundp 'org-cycle-separator-lines)
                      org-cycle-separator-lines)
   :empty-lines-before-fbound (boundp 'org-cycle-separator-lines)
   ))"##,
    );
}

#[test]
fn combo67_element_affiliated_on_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+CAPTION: result caption\n")
  (insert "#+NAME: result-name\n")
  (insert "#+RESULTS:\n: result\n#+END:\n")
  (let ((r '()))
    (let* ((tree (org-element-parse-buffer))
           (keywords (org-element-map tree 'keyword #'identity)))
      (push (list :keyword-count (length keywords)) r)
      (push (list :keyword-keys (mapcar (lambda (k) (org-element-property :key k)) keywords)) r)
      (push (list :keyword-values (mapcar (lambda (k) (org-element-property :value k)) keywords)) r))
    (nreverse r)))"##,
    );
}

#[test]
fn combo67_table_formula_arithmetic_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 0 | 5 |   |\n| 2 | 0 |   |\n")
  (insert "#+TBLFM: $3=$2/$1\n")
  (let ((r '()))
    (goto-char (point-min))
    (condition-case e
        (progn (org-table-recalculate t) (org-table-align)
               (push (list :after-recalc (buffer-string)) r))
      (error (push (list :recalc-error (car e)) r)))
    ;; to-lisp still works even with errors
    (goto-char (point-min))
    (push (list :to-lisp (org-table-to-lisp)) r)
    (nreverse r)))"##,
    );
}

#[test]
fn combo67_babel_insert_result() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (list
   :insert-result-fbound (fboundp 'org-babel-insert-result)
   ;; org-babel-result-end
   :result-end-fbound (fboundp 'org-babel-result-end)
   ))"##,
    );
}

#[test]
fn combo67_org_property_inheritance_settings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (list
   :use-prop-inherit-fbound (boundp 'org-use-property-inheritance)
   :use-prop-inherit (when (boundp 'org-use-property-inheritance)
                       org-use-property-inheritance)
   :prop-inherit-var-bound (boundp 'org-use-property-inheritance)
   ))"##,
    );
}
