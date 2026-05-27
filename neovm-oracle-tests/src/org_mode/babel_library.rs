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

#[test]
fn org_babel_lob_noweb_export_result_replacement_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-lob)
  (require 'ob-emacs-lisp)
  (require 'ox-html)
  (require 'ox-ascii)
  (let* ((root (make-temp-file "org-lob-export" t))
         (lib (expand-file-name "lib.org" root))
         (org-babel-library-of-babel nil)
         (org-confirm-babel-evaluate nil)
         (org-export-use-babel t)
         (org-export-with-broken-links t))
    (unwind-protect
        (progn
          (with-temp-file lib
            (insert "#+PROPERTY: header-args:emacs-lisp :results value replace\n")
            (insert "#+NAME: helper-lines\n")
            (insert "#+begin_src emacs-lisp :var prefix=\"x\" :var rows='((\"a\" 1))\n")
            (insert "(mapcar (lambda (row)\n")
            (insert "          (format \"%s:%s=%s\" prefix (car row) (cadr row)))\n")
            (insert "        rows)\n")
            (insert "#+end_src\n\n")
            (insert "#+NAME: wrap-lines\n")
            (insert "#+begin_src emacs-lisp :var title=\"T\" :var rows='((\"a\" 1)) :noweb yes\n")
            (insert "(cons title (helper-lines prefix=title rows=rows))\n")
            (insert "#+end_src\n"))
          (org-babel-lob-ingest lib)
          (with-temp-buffer
            (org-mode)
            (insert "#+TITLE: LOB Export\n")
            (insert "#+NAME: data\n")
            (insert "| key | n |\n")
            (insert "|-----+---|\n")
            (insert "| a   | 2 |\n")
            (insert "| b   | 3 |\n\n")
            (insert "* Calls\n")
            (insert "#+CALL: wrap-lines(title=\"Run\", rows=data[2:3,*]) :results value list replace :exports both\n\n")
            (insert "Inline call_helper-lines[:results raw replace](prefix=\"I\", rows=data[2:2,*]) done.\n")
            (let (call-info inline-info after-call after-inline html ascii ast)
              (goto-char (point-min))
              (search-forward "#+CALL")
              (setq call-info (org-babel-lob-get-info))
              (org-babel-execute-maybe)
              (setq after-call
                    (buffer-substring-no-properties
                     (point-min) (point-max)))
              (goto-char (point-min))
              (search-forward "call_helper-lines")
              (setq inline-info (org-babel-lob-get-info))
              (org-babel-execute-maybe)
              (setq after-inline
                    (buffer-substring-no-properties
                     (point-min) (point-max)))
              (setq ast
                    (org-element-map (org-element-parse-buffer)
                        '(babel-call inline-babel-call plain-list item table)
                      (lambda (el)
                        (list (org-element-type el)
                              (org-element-property :call el)
                              (org-element-property :arguments el)
                              (org-element-property :begin el)
                              (org-element-property :end el)))))
              (setq html
                    (replace-regexp-in-string
                     "org[[:alnum:]]+"
                     "org-id"
                     (org-export-as 'html nil nil t '(:with-toc nil))))
              (setq ascii
                    (let ((org-ascii-charset 'utf-8))
                      (org-export-as 'ascii nil nil t
                                     '(:with-toc nil))))
              (list (sort (mapcar (lambda (cell)
                                    (symbol-name (car cell)))
                                  org-babel-library-of-babel)
                          #'string<)
                    (nth 0 call-info)
                    (assq :var (nth 2 call-info))
                    (assq :results (nth 2 call-info))
                    (nth 0 inline-info)
                    (assq :var (nth 2 inline-info))
                    (assq :results (nth 2 inline-info))
                    after-call
                    after-inline
                    ast
                    (mapcar (lambda (needle)
                              (not (null (string-match-p needle html))))
                            '("LOB Export" "Run" "Run:a=2" "Run:b=3"
                              "I:a=2"))
                    (mapcar (lambda (needle)
                              (not (null (string-match-p needle ascii))))
                            '("LOB Export" "Run:a=2" "Run:b=3" "I:a=2"))
                    html
                    ascii))))
      (when (file-directory-p root) (delete-directory root t)))))"##,
    );
}

#[test]
fn org_babel_execute_buffer_call_inline_remove_combo() {
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
      (insert "| key | n |\n")
      (insert "|-----+---|\n")
      (insert "| a   | 2 |\n")
      (insert "| b   | 4 |\n\n")
      (insert "#+NAME: rows->table\n")
      (insert "#+begin_src emacs-lisp :var rows=nums[2:3,*] :var factor=1 :results value table replace\n")
      (insert "(mapcar (lambda (row)\n")
      (insert "          (list (car row)\n")
      (insert "                (string-to-number (cadr row))\n")
      (insert "                (* factor (string-to-number (cadr row)))))\n")
      (insert "        rows)\n")
      (insert "#+end_src\n\n")
      (insert "#+NAME: scalar\n")
      (insert "#+begin_src emacs-lisp :var label=\"x\" :var n=0 :results value replace\n")
      (insert "(format \"%s:%s\" label (* n n))\n")
      (insert "#+end_src\n\n")
      (insert "* Calls\n")
      (insert "#+CALL: rows->table(rows=nums[2:3,*], factor=3) :results value table replace\n\n")
      (insert "Inline call_scalar[:results value replace](label=\"sq\", n=5) and ")
      (insert "call_scalar[:results value replace](label=\"cube\", n=3) done.\n")
      (let (mapped before after-first call-table inline-line removed-line
            after-edit after-remove executable-types parsed-results)
        (org-babel-map-call-lines nil
          (let ((ctx (org-element-context)))
            (push (list (org-element-type ctx)
                        (org-element-property :call ctx)
                        (org-element-property :arguments ctx)
                        (org-element-property :inside-header ctx)
                        (org-element-property :end-header ctx)
                        (org-element-property :begin ctx)
                        (org-element-property :end ctx))
                  mapped)))
        (setq before (buffer-substring-no-properties (point-min) (point-max)))
        (goto-char (point-min))
        (org-babel-execute-buffer)
        (setq after-first
              (buffer-substring-no-properties (point-min) (point-max)))
        (goto-char (point-min))
        (search-forward "#+CALL")
        (setq call-table
              (save-excursion
                (goto-char (org-babel-where-is-src-block-result))
                (forward-line 1)
                (org-babel-read-result)))
        (goto-char (point-min))
        (search-forward "Inline")
        (setq inline-line
              (buffer-substring-no-properties
               (line-beginning-position) (line-end-position)))
        (search-forward "call_scalar")
        (org-babel-remove-inline-result)
        (setq removed-line
              (buffer-substring-no-properties
               (line-beginning-position) (line-end-position)))
        (goto-char (point-min))
        (search-forward "| b")
        (search-forward "4")
        (replace-match "6" t t)
        (goto-char (point-min))
        (search-forward "#+CALL")
        (org-babel-lob-execute-maybe)
        (setq after-edit
              (buffer-substring-no-properties (point-min) (point-max)))
        (org-babel-map-executables nil
          (push (org-element-type (org-element-context)) executable-types))
        (setq parsed-results
              (org-element-map (org-element-parse-buffer)
                  '(babel-call inline-babel-call macro table)
                (lambda (el)
                  (list (org-element-type el)
                        (org-element-property :call el)
                        (org-element-property :key el)
                        (org-element-property :value el)
                        (org-element-property :begin el)
                        (org-element-property :end el)))))
        (goto-char (point-min))
        (search-forward "#+CALL")
        (org-babel-remove-result nil t)
        (setq after-remove
              (buffer-substring-no-properties (point-min) (point-max)))
        (list (nreverse mapped)
              before
              after-first
              call-table
              inline-line
              removed-line
              after-edit
              (sort (mapcar #'symbol-name executable-types) #'string<)
              parsed-results
              after-remove)))))"##,
    );
}

#[test]
fn org_babel_session_dir_noweb_file_tangle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (require 'ob-tangle)
  (let* ((root (make-temp-file "org-babel-sess" t))
         (src-file (expand-file-name "work.org" root))
         (tangle-out (expand-file-name "out.el" root))
         (dir-file (expand-file-name "dirprobe.txt" root))
         (org-confirm-babel-evaluate nil)
         (org-babel-default-header-args
          '((:results . "output replace")
            (:exports . "results"))))
    (unwind-protect
        (progn
          (with-temp-file src-file
            (insert "#+PROPERTY: header-args:emacs-lisp :results value replace\n\n")
            (insert "#+NAME: helper\n")
            (insert "#+begin_src emacs-lisp :var n=1\n")
            (insert "(list :n n :square (* n n))\n")
            (insert "#+end_src\n\n")
            (insert "#+NAME: use-noweb\n")
            (insert "#+begin_src emacs-lisp :noweb yes :results value replace\n")
            (insert "(let ((base (<<helper(n=3)>>)))\n")
            (insert "  (list :base base :doubled (* 2 (plist-get base :n))))\n")
            (insert "#+end_src\n\n")
            (insert "#+begin_src emacs-lisp :tangle " tangle-out " :noweb yes\n")
            (insert ";; tangled helper\n")
            (insert "<<helper(n=5)>>\n")
            (insert "#+end_src\n\n")
            (insert "#+begin_src emacs-lisp :dir " root " :results value replace\n")
            (insert "(expand-file-name \"probe.txt\")\n")
            (insert "#+end_src\n\n"))
          (with-current-buffer (find-file-noselect src-file)
            (org-mode)
            (let ((noweb-result nil)
                  (dir-result nil)
                  (tangle-files nil)
                  (tangle-content nil)
                  (all-results nil))
              (goto-char (point-min))
              (search-forward "use-noweb")
              (org-babel-execute-src-block)
              (setq noweb-result
                    (org-babel-read-result))
              (goto-char (point-min))
              (search-forward "expand-file-name")
              (org-babel-execute-src-block)
              (setq dir-result
                    (org-babel-read-result))
              (setq tangle-files (org-babel-tangle))
              (when (file-exists-p tangle-out)
                (with-temp-buffer
                  (insert-file-contents tangle-out)
                  (setq tangle-content (buffer-string))))
              (goto-char (point-min))
              (while (re-search-forward "#\\+RESULTS:" nil t)
                (forward-line 1)
                (let ((beg (point)))
                  (if (re-search-forward "^$" nil t)
                      (push (buffer-substring-no-properties beg (point))
                            all-results)
                    (push (buffer-substring-no-properties beg (point-max))
                          all-results))))
              (list noweb-result
                    dir-result
                    (mapcar #'file-name-nondirectory tangle-files)
                    tangle-content
                    (nreverse all-results)
                    (replace-regexp-in-string
                     (regexp-quote root) "<root>"
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))
             (kill-buffer)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_babel_cache_file_var_result_lifecycle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (let* ((root (make-temp-file "org-babel-cache" t))
         (cache-file (expand-file-name "cache.org" root))
         (out-file (expand-file-name "output.txt" root))
         (org-confirm-babel-evaluate nil)
         (norm (lambda (s)
                 (replace-regexp-in-string
                  "[0-9a-f]\\{40\\}" "HASH"
                  (replace-regexp-in-string
                   "(27[0-9]+ [0-9]+ [0-9]+ [0-9]+)"
                   "(TIMESTAMP)"
                   (replace-regexp-in-string
                    (regexp-quote root) "<root>" s))))))
    (unwind-protect
        (progn
          (with-temp-file cache-file
            (insert "#+PROPERTY: header-args:emacs-lisp :results value replace\n\n")
            (insert "#+NAME: counter\n")
            (insert "#+begin_src emacs-lisp :var n=1 :cache yes\n")
            (insert "(list :count n :double (* 2 n) :ts (format \"%s\" (current-time)))\n")
            (insert "#+end_src\n\n")
            (insert "#+RESULTS[abc]: counter\n")
            (insert ":cached-placeholder\n\n")
            (insert "#+NAME: adder\n")
            (insert "#+begin_src emacs-lisp :var a=1 b=2 :results value replace\n")
            (insert "(list :sum (+ a b) :product (* a b) :diff (- a b))\n")
            (insert "#+end_src\n\n")
            (insert "#+NAME: writer\n")
            (insert "#+begin_src emacs-lisp :var data=adder :file " out-file "\n")
            (insert "(with-temp-file \"" out-file "\"\n")
            (insert "  (insert (format \"sum=%s prod=%s\" (plist-get data :sum) (plist-get data :product))))\n")
            (insert "\"done\")\n")
            (insert "#+end_src\n\n"))
          (with-current-buffer (find-file-noselect cache-file)
            (org-mode)
            (let ((snap (lambda ()
                          (funcall norm
                                   (buffer-substring-no-properties
                                    (point-min) (point-max))))))
              (let ((before (funcall snap)))
                ;; Execute counter
                (goto-char (point-min))
                (search-forward "counter")
                (org-babel-execute-src-block)
                (let ((after-counter (funcall snap)))
                  ;; Execute adder
                  (goto-char (point-min))
                  (search-forward "adder")
                  (org-babel-execute-src-block)
                  (let ((after-adder (funcall snap)))
                    ;; Execute writer
                    (goto-char (point-min))
                    (search-forward "writer")
                    (org-babel-execute-src-block)
                    (let ((after-writer (funcall snap))
                          (file-content
                           (when (file-exists-p out-file)
                             (with-temp-buffer
                               (insert-file-contents out-file)
                               (funcall norm (buffer-string))))))
                      ;; Re-execute counter with different var
                      (goto-char (point-min))
                      (search-forward "counter")
                      (org-babel-execute-src-block '(4))
                      (let ((after-re-exec (funcall snap)))
                        (list before
                              after-counter
                              after-adder
                              after-writer
                              (or file-content "no-file")
                              after-re-exec))))))))
            (kill-buffer)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_babel_execute_result_value_insertion_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+NAME: adder\n")
      (insert "#+begin_src emacs-lisp :var a=1 b=2 :results value replace\n")
      (insert "(list :sum (+ a b) :product (* a b) :diff (- a b))\n")
      (insert "#+end_src\n\n")
      (insert "#+NAME: lister\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(princ (format \"item1=%s item2=%s\" 42 99))\n")
      (insert "#+end_src\n\n")
      (insert "#+NAME: tabler\n")
      (insert "#+begin_src emacs-lisp :results value table replace\n")
      (insert "'((\"X\" \"Y\") hline (1 2) (3 4) (5 6))\n")
      (insert "#+end_src\n\n")
      ;; Execute adder
      (goto-char (point-min))
      (search-forward "adder")
      (org-babel-execute-src-block)
      (let ((adder-result (org-babel-read-result))
            (adder-buf (buffer-substring-no-properties
                        (point-min) (point-max))))
        ;; Execute lister
        (goto-char (point-min))
        (search-forward "lister")
        (org-babel-execute-src-block)
        (let ((lister-result (org-babel-read-result))
              (lister-buf (buffer-substring-no-properties
                           (point-min) (point-max))))
          ;; Execute tabler
          (goto-char (point-min))
          (search-forward "tabler")
          (org-babel-execute-src-block)
          (let ((tabler-result (org-babel-read-result))
                (tabler-buf (buffer-substring-no-properties
                             (point-min) (point-max))))
            ;; Extract all RESULTS blocks
            (let ((results-blocks nil))
              (goto-char (point-min))
              (while (re-search-forward "^#\\+RESULTS:" nil t)
                (forward-line 1)
                (let ((beg (point)))
                  (if (re-search-forward "^$" nil t)
                      (push (buffer-substring-no-properties beg (point))
                            results-blocks)
                    (push (buffer-substring-no-properties beg (point-max))
                          results-blocks))))
              (list adder-result
                    lister-result
                    tabler-result
                    (nreverse results-blocks)
                    (org-element-map (org-element-parse-buffer) 'src-block
                      (lambda (sb)
                        (list (org-element-property :name sb)
                              (org-element-property :language sb)
                              (org-element-property :parameters sb))))
                    tabler-buf))))))))))"##,
    );
}

#[test]
fn org_babel_dir_default_dir_header_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (let* ((root (make-temp-file "org-babel-dir" t))
         (probe (expand-file-name "probe.txt" root))
         (org-confirm-babel-evaluate nil))
    (unwind-protect
        (with-temp-buffer
          (org-mode)
          (insert "#+begin_src emacs-lisp :dir " root " :results value replace\n")
          (insert "(expand-file-name \"probe.txt\")\n")
          (insert "#+end_src\n\n")
          (insert "#+begin_src emacs-lisp :results output replace\n")
          (insert "(princ (format \"default-dir=%s\" default-directory))\n")
          (insert "#+end_src\n\n")
          ;; Execute dir block
          (goto-char (point-min))
          (search-forward "expand-file-name")
          (org-babel-execute-src-block)
          (let ((dir-result (org-babel-read-result))
                (after-dir (buffer-substring-no-properties
                            (point-min) (point-max))))
            ;; Execute default-dir block
            (goto-char (point-min))
            (search-forward "default-dir=")
            (org-babel-execute-src-block)
            (let ((default-result (org-babel-read-result))
                  (after-default (buffer-substring-no-properties
                                  (point-min) (point-max))))
              (list (replace-regexp-in-string
                     (regexp-quote root) "<root>" dir-result)
                    (replace-regexp-in-string
                     (regexp-quote root) "<root>" default-result)
                    (replace-regexp-in-string
                     (regexp-quote root) "<root>" after-dir)
                    (replace-regexp-in-string
                     (regexp-quote root) "<root>" after-default)))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_babel_execute_buffer_inline_call_remove_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+NAME: double\n")
      (insert "#+begin_src emacs-lisp :var n=3 :results value replace\n")
      (insert "(* n 2)\n")
      (insert "#+end_src\n\n")
      (insert "#+CALL: double(n=7) :results value replace\n\n")
      (insert "Inline call_double[:results raw](n=5) here.\n\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(princ \"output-block\")\n")
      (insert "#+end_src\n\n")
      ;; Execute buffer
      (org-babel-execute-buffer)
      (let ((after-execute (buffer-substring-no-properties
                            (point-min) (point-max)))
            (call-result
             (progn
               (goto-char (point-min))
               (search-forward "CALL: double")
               (when (org-babel-where-is-src-block-result)
                 (goto-char (org-babel-where-is-src-block-result))
                 (forward-line 1)
                 (buffer-substring-no-properties
                  (line-beginning-position) (line-end-position)))))
            (inline-result
             (progn
               (goto-char (point-min))
               (search-forward "Inline call_double")
               (end-of-line)
               (search-backward "call_double")
               (search-forward "=>")
               (buffer-substring-no-properties
                (point) (line-end-position))))
            (elements
             (org-element-map (org-element-parse-buffer)
                 '(babel-call inline-babel-call src-block)
               (lambda (el)
                 (list (org-element-type el)
                       (org-element-property :call el)
                       (org-element-property :value el))))))
        ;; Remove inline result
        (goto-char (point-min))
        (search-forward "call_double")
        (org-babel-remove-inline-result)
        (let ((after-remove (buffer-substring-no-properties
                             (point-min) (point-max))))
          ;; Remove block result
          (goto-char (point-min))
          (search-forward "output-block")
          (org-babel-remove-result)
          (let ((after-remove-block (buffer-substring-no-properties
                                     (point-min) (point-max))))
            (list after-execute
                  call-result
                  inline-result
                  elements
                  after-remove
                  after-remove-block)))))))))"##,
    );
}
