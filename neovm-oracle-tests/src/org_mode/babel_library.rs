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

#[test]
fn org_babel_execute_var_table_output_header_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+NAME: data\n")
      (insert "| X | Y |\n|---+---|\n| 1 | 10 |\n| 2 | 20 |\n| 3 | 30 |\n\n")
      (insert "#+NAME: compute\n")
      (insert "#+begin_src emacs-lisp :var tbl=data factor=2 :results value table replace\n")
      (insert "(cons '(\"X\" \"Y\" \"Product\")\n")
      (insert "      (cons 'hline\n")
      (insert "            (mapcar (lambda (row)\n")
      (insert "                      (list (car row) (cadr row) (* (cadr row) factor)))\n")
      (insert "                    tbl)))\n")
      (insert ")\n")
      (insert "#+end_src\n\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(dotimes (i 3)\n  (princ (format \"line %d\\n\" i)))\n")
      (insert "#+end_src\n\n")
      ;; Execute compute
      (goto-char (point-min))
      (search-forward "compute")
      (org-babel-execute-src-block)
      (let ((compute-result (org-babel-read-result))
            (after-compute (buffer-substring-no-properties
                            (point-min) (point-max))))
        ;; Execute output
        (goto-char (point-min))
        (search-forward "dotimes")
        (org-babel-execute-src-block)
        (let ((output-result (org-babel-read-result))
              (after-output (buffer-substring-no-properties
                             (point-min) (point-max)))
              ;; Parse results
              (results-els
               (org-element-map (org-element-parse-buffer)
                   '(fixed-width src-block)
                 (lambda (el)
                   (list (org-element-type el)
                         (org-element-property :value el))))))
           (list compute-result
                 after-compute
                 output-result
                 after-output
                 results-els)))))))"##,
    );
}

#[test]
fn org_babel_header_arg_merge_property_inherit_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Set file-level property
      (insert "#+PROPERTY: header-args :results value replace\n\n")
      ;; Block with its own header
      (insert "#+NAME: merged\n")
      (insert "#+begin_src emacs-lisp :var x=10\n")
      (insert "(list :x x :doubled (* x 2))\n")
      (insert "#+end_src\n\n")
      ;; Block with :results output override
      (insert "#+NAME: output-block\n")
      (insert "#+begin_src emacs-lisp :results output\n")
      (insert "(princ (format \"output-mode=%s\" 'output))\n")
      (insert "#+end_src\n\n")
      ;; Block with :file
      (let* ((root (make-temp-file "org-babel-merge" t))
             (out-file (expand-file-name "result.txt" root)))
        (unwind-protect
            (progn
              (insert "#+NAME: file-writer\n")
              (insert "#+begin_src emacs-lisp :file " out-file "\n")
              (insert "(with-temp-file \"" out-file "\"\n  (insert \"file-content\"))\n  \"done\"\n")
              (insert "#+end_src\n\n")
              ;; Execute merged
              (goto-char (point-min))
              (search-forward "merged")
              (org-babel-execute-src-block)
              (let ((merged-result (org-babel-read-result)))
                ;; Execute output-block
                (goto-char (point-min))
                (search-forward "output-block")
                (org-babel-execute-src-block)
                (let ((output-result (org-babel-read-result)))
                  ;; Execute file-writer
                  (goto-char (point-min))
                  (search-forward "file-writer")
                  (org-babel-execute-src-block)
                  (let ((file-result (org-babel-read-result))
                        (file-content
                         (when (file-exists-p out-file)
                           (with-temp-buffer
                             (insert-file-contents out-file)
                             (buffer-string)))))
                    (list merged-result
                          output-result
                          file-result
                          (replace-regexp-in-string
                           (regexp-quote root) "<root>"
                           (or file-content "no-file"))
                          (buffer-substring-no-properties
                           (point-min) (point-max))))))))
           (delete-directory root t))))))"##,
    );
}

#[test]
fn org_babel_execute_result_type_handling_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Scalar value
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(+ 10 20)\n")
      (insert "#+end_src\n\n")
      ;; List value
      (insert "#+NAME: lister\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'(a b c)\n")
      (insert "#+end_src\n\n")
      ;; String value
      (insert "#+NAME: stringer\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(concat \"hello\" \" \" \"world\")\n")
      (insert "#+end_src\n\n")
      ;; Nil value
      (insert "#+NAME: niler\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "nil\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dolist (name '("src_" "lister" "stringer" "niler"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_header_override_noweb_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+NAME: config\n")
      (insert "#+begin_src emacs-lisp\n")
      (insert "(defconst multiplier 3)\n")
      (insert "#+end_src\n\n")
      (insert "#+NAME: compute\n")
      (insert "#+begin_src emacs-lisp :var x=5 :noweb yes :results value replace\n")
      (insert "(let ((m (progn <<config>> multiplier)))\n")
      (insert "  (* x m))\n")
      (insert "#+end_src\n\n")
      (insert "#+begin_src emacs-lisp :results output replace :var val=compute\n")
      (insert "(princ (format \"result=%s\" val))\n")
      (insert "#+end_src\n\n")
      ;; Execute compute with noweb
      (goto-char (point-min))
      (search-forward "compute")
      (org-babel-execute-src-block)
      (let ((compute-result (org-babel-read-result))
            (after-compute (buffer-substring-no-properties
                            (point-min) (point-max))))
        ;; Execute output with var reference
        (goto-char (point-min))
        (search-forward "princ")
        (org-babel-execute-src-block)
        (let ((output-result (org-babel-read-result))
              (after-output (buffer-substring-no-properties
                             (point-min) (point-max)))
              ;; Parse results
              (results
               (org-element-map (org-element-parse-buffer)
                   '(fixed-width src-block)
                 (lambda (el)
                   (list (org-element-type el)
                         (org-element-property :name el)
                         (org-element-property :value el))))))
           (list compute-result
                 after-compute
                 output-result
                 after-output
                 results)))))))"##,
    );
}

#[test]
fn org_babel_execute_multiple_blocks_result_chain_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Block A: produces a list
      (insert "#+NAME: producer\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((:x 1) (:y 2) (:z 3))\n")
      (insert "#+end_src\n\n")
      ;; Block B: uses producer result
      (insert "#+NAME: consumer\n")
      (insert "#+begin_src emacs-lisp :var data=producer :results value replace\n")
      (insert "(mapcar (lambda (item)\n")
      (insert "          (list (car item) (* 10 (cadr item))))\n")
      (insert "        data)\n")
      (insert "#+end_src\n\n")
      ;; Block C: uses consumer result
      (insert "#+NAME: final\n")
      (insert "#+begin_src emacs-lisp :var data=consumer :results output replace\n")
      (insert "(dolist (item data)\n")
      (insert "  (princ (format \"%s=%s\\n\" (car item) (cadr item))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all in order
      (dolist (name '("producer" "consumer" "final"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read all results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Parse elements
        (let ((elements
               (org-element-map (org-element-parse-buffer)
                   '(src-block fixed-width)
                 (lambda (el)
                   (list (org-element-type el)
                         (org-element-property :name el)
                         (org-element-property :value el))))))
           (list (nreverse results)
                 elements
                 (buffer-substring-no-properties
                  (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_babel_execute_tree_map_reduce_pipeline_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Build tree
      (insert "#+NAME: tree\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((dept-a . ((emp . 10) (budget . 100)))\n")
      (insert "  (dept-b . ((emp . 20) (budget . 200)))\n")
      (insert "  (dept-c . ((emp . 15) (budget . 150))))\n")
      (insert "#+end_src\n\n")
      ;; Map: extract and transform
      (insert "#+NAME: mapped\n")
      (insert "#+begin_src emacs-lisp :var data=tree :results value replace\n")
      (insert "(mapcar (lambda (dept)\n")
      (insert "          (let ((name (car dept))\n")
      (insert "                (emp (cdr (assoc 'emp (cdr dept))))\n")
      (insert "                (budget (cdr (assoc 'budget (cdr dept)))))\n")
      (insert "            (list name emp budget (/ budget emp))))\n")
      (insert "        data)\n")
      (insert "#+end_src\n\n")
      ;; Reduce: aggregate
      (insert "#+NAME: agg\n")
      (insert "#+begin_src emacs-lisp :var data=mapped :results value replace\n")
      (insert "(let ((emps (mapcar #'cadr data))\n")
      (insert "      (budgets (mapcar #'caddr data)))\n")
      (insert "  (list :total-emp (apply #'+ emps)\n")
      (insert "        :total-budget (apply #'+ budgets)\n")
      (insert "        :avg-per-person (/ (apply #'+ budgets)\n")
      (insert "                          (apply #'+ emps))))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("tree" "mapped" "agg"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_multi_table_var_chain_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Table A
      (insert "#+NAME: prices\n")
      (insert "| Item | Cost |\n")
      (insert "|------+------|\n")
      (insert "| A | 10 |\n")
      (insert "| B | 20 |\n")
      (insert "| C | 30 |\n\n")
      ;; Table B
      (insert "#+NAME: quantities\n")
      (insert "| Item | Qty |\n")
      (insert "|------+-----|\n")
      (insert "| A | 3 |\n")
      (insert "| B | 1 |\n")
      (insert "| C | 5 |\n\n")
      ;; Compute join
      (insert "#+NAME: totals\n")
      (insert "#+begin_src emacs-lisp :var p=prices q=quantities :results value table replace\n")
      (insert "(cons '(\"Item\" \"Cost\" \"Qty\" \"Total\")\n")
      (insert "      (cons 'hline\n")
      (insert "            (mapcar (lambda (row)\n")
      (insert "                      (let* ((item (car row))\n")
      (insert "                             (cost (cadr row))\n")
      (insert "                             (qty-row (assoc item q))\n")
      (insert "                             (qty (if qty-row (cadr qty-row) 0)))\n")
      (insert "                        (list item cost qty (* cost qty))))\n")
      (insert "                    p)))\n")
      (insert "#+end_src\n\n")
      ;; Execute
      (goto-char (point-min))
      (search-forward "totals")
      (org-babel-execute-src-block)
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_sequence_slice_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Sequence slicing
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s '(a b c d e f g h)))\n")
      (insert "  (list (seq-take s 3) (seq-drop s 5)\n")
      (insert "        (seq-subseq s 1 4) (seq-subseq s 2)\n")
      (insert "        (seq-take-while #'symbolp s)\n")
      (insert "        (seq-drop-while (lambda (x) (not (eq x 'd))) s)))\n")
      (insert "#+end_src\n\n")
      ;; Sequence search
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s '(3 1 4 1 5 9 2 6 5 3 5)))\n")
      (insert "  (list (seq-find (lambda (x) (> x 4)) s)\n")
      (insert "        (seq-count (lambda (x) (= x 5)) s)\n")
      (insert "        (seq-contains-p s 9)\n")
      (insert "        (seq-contains-p s 0)\n")
      (insert "        (seq-positions s 5)\n")
      (insert "        (seq-uniq s)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_io_process_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Buffer operations
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(with-temp-buffer\n")
      (insert "  (insert \"hello\\nworld\\nfoo\\n\")\n")
      (insert "  (list (buffer-string)\n")
      (insert "        (count-lines (point-min) (point-max))\n")
      (insert "        (progn (goto-char (point-min))\n")
      (insert "               (search-forward \"world\")\n")
      (insert "               (line-number-at-pos))))\n")
      (insert "#+end_src\n\n")
      ;; File operations
      (let* ((root (make-temp-file "org-babel-io" t))
             (tmp (expand-file-name "test.txt" root)))
        (insert "#+begin_src emacs-lisp :results value replace\n")
        (insert "(let ((f \"" tmp "\"))\n")
        (insert "  (with-temp-file f (insert \"line1\\nline2\\n\"))\n")
        (insert "  (list (file-exists-p f)\n")
        (insert "        (with-temp-buffer\n")
        (insert "          (insert-file-contents f)\n")
        (insert "          (buffer-string))))\n")
        (insert "#+end_src\n\n"))
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_vector_operations_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Vector create and access
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((v [10 20 30 40 50]))\n")
      (insert "  (list (aref v 0) (aref v 4) (length v)\n")
      (insert "        (seq-into '(1 2 3) 'vector)\n")
      (insert "        (append v nil)))\n")
      (insert "#+end_src\n\n")
      ;; Vector operations
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((v1 [1 2 3]) (v2 [4 5 6]))\n")
      (insert "  (list (vconcat v1 v2)\n")
      (insert "        (mapcar (lambda (x) (* x 2)) (append v1 nil))\n")
      (insert "        (seq-sort #'< (vconcat v2 v1))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_type_check_coerce_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Type checks
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (type-of 42) (type-of 3.14) (type-of \"hi\")\n")
      (insert "      (type-of '(1 2)) (type-of [1 2]) (type-of t)\n")
      (insert "      (type-of nil) (type-of (make-hash-table)))\n")
      (insert "#+end_src\n\n")
      ;; Type coercion
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (string-to-number \"42\") (string-to-number \"3.14\")\n")
      (insert "      (number-to-string 42) (number-to-string 3.14)\n")
      (insert "      (int-to-string 42) (string-to-int \"42\")\n")
      (insert "      (float 42) (truncate 3.7) (round 3.7))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_string_predicates_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; String predicates
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (stringp \"hello\") (stringp 42)\n")
      (insert "      (string-empty-p \"\") (string-empty-p \"x\")\n")
      (insert "      (string< \"a\" \"b\") (string> \"z\" \"a\")\n")
      (insert "      (string-prefix-p \"Hel\" \"Hello\")\n")
      (insert "      (string-suffix-p \"lo\" \"Hello\")\n")
      (insert "      (string-match-p \"[0-9]+\" \"abc123\"))\n")
      (insert "#+end_src\n\n")
      ;; String operations
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s \"  Hello World  \"))\n")
      (insert "  (list (string-trim s)\n")
      (insert "        (string-trim-left s)\n")
      (insert "        (string-trim-right s)\n")
      (insert "        (string-pad \"hi\" 5)\n")
      (insert "        (string-fill \"Hello World\" 5)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_number_predicates_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Number predicates
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (numberp 42) (integerp 42) (floatp 3.14)\n")
      (insert "      (natnump 0) (wholenump 5) (zerop 0)\n")
      (insert "      (plusp 1) (minusp -1) (oddp 3) (evenp 4))\n")
      (insert "#+end_src\n\n")
      ;; Arithmetic with edge cases
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (/ 10 3) (% 10 3) (mod 10 3)\n")
      (insert "      (abs -7) (max 1 3 2) (min 5 2 8)\n")
      (insert "      (expt 2 10) (sqrt 144) (log 100 10))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_list_map_tree_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; list + mapcar + tree
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((tree '(1 (2 (3 4) 5) (6 7))))\n")
      (insert "  (list tree\n")
      (insert "        (mapcar (lambda (x) (if (listp x) (length x) (* x 10)))\n")
      (insert "                tree)\n")
      (insert "        (apply #'+ (mapcar (lambda (x)\n")
      (insert "                             (if (listp x)\n")
      (insert "                                 (apply #'+ x)\n")
      (insert "                                 x))\n")
      (insert "                           tree))))\n")
      (insert "#+end_src\n\n")
      ;; cons + nth + length
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((l (cons 1 (cons 2 (cons 3 nil)))))\n")
      (insert "  (list l (nth 1 l) (length l)\n")
      (insert "        (last l) (butlast l 1)\n")
      (insert "        (append l '(4 5))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_seq_count_position_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; seq-count with predicate
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (seq-count #'evenp '(1 2 3 4 5 6 7 8))\n")
      (insert "      (seq-count (lambda (x) (> x 5)) '(1 2 3 4 5 6 7 8)))\n")
      (insert "#+end_src\n\n")
      ;; seq-position and seq-index-of
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s '(a b c d e f)))\n")
      (insert "  (list (seq-position s 'c)\n")
      (insert "        (seq-position s 'z)\n")
      (insert "        (seq-index-of s 'e)\n")
      (insert "        (seq-contains-p s 'd)\n")
      (insert "        (seq-contains-p s 'g)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_seq_partition_flatten_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; seq-partition
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-partition '(1 2 3 4 5 6 7 8 9) 3)\n")
      (insert "#+end_src\n\n")
      ;; Flatten nested
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((nested '((1 2) (3 (4 5)) (6))))\n")
      (insert "  (list (apply #'append nested)\n")
      (insert "        (cl-loop for x in nested append x)\n")
      (insert "        (seq-uniq (apply #'append (apply #'append nested)))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_map_assoc_sort_reverse_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Map + assoc
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((data (mapcar (lambda (x) (cons x (* x x)))\n")
      (insert "                    '(3 1 4 1 5 9 2 6))))\n")
      (insert "  (list data\n")
      (insert "        (sort (copy-sequence data) (lambda (a b) (< (cdr a) (cdr b))))\n")
      (insert "        (reverse (sort (copy-sequence data) (lambda (a b) (< (cdr a) (cdr b)))))))\n")
      (insert "#+end_src\n\n")
      ;; Nested mapcar with lambda
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((matrix (mapcar (lambda (row)\n")
      (insert "                        (mapcar (lambda (cell) (* cell cell))\n")
      (insert "                                row))\n")
      (insert "                      '((1 2 3) (4 5 6) (7 8 9)))))\n")
      (insert "  (list matrix\n")
      (insert "        (apply #'+ (mapcar (lambda (row) (apply #'+ row)) matrix))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_string_replace_concat_length_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; String replace
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s \"Hello World Foo Bar\"))\n")
      (insert "  (list (replace-regexp-in-string \"World\" \"Elisp\" s)\n")
      (insert "        (replace-regexp-in-string \"[A-Z]\" \"X\" s)\n")
      (insert "        (length s)))\n")
      (insert "#+end_src\n\n")
      ;; String concat and format
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (concat \"a\" \"-\" \"b\" \"-\" \"c\")\n")
      (insert "      (format \"%05d\" 42)\n")
      (insert "      (format \"%.3f\" 3.14159)\n")
      (insert "      (format \"%-10s|\" \"left\"))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_multi_var_header_chain_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Seed block
      (insert "#+NAME: seed\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((a . 5) (b . 10) (c . 15))\n")
      (insert "#+end_src\n\n")
      ;; Transform with default var
      (insert "#+NAME: xform\n")
      (insert "#+begin_src emacs-lisp :var data=seed :results value replace\n")
      (insert "(mapcar (lambda (pair)\n")
      (insert "          (cons (car pair) (list (cdr pair) (* (cdr pair) 2))))\n")
      (insert "        data)\n")
      (insert "#+end_src\n\n")
      ;; Aggregate
      (insert "#+NAME: agg\n")
      (insert "#+begin_src emacs-lisp :var data=xform :results value replace\n")
      (insert "(let ((vals (mapcar #'cadr data)))\n")
      (insert "  (list :count (length vals)\n")
      (insert "        :sum (apply #'+ vals)\n")
      (insert "        :min (apply #'min vals)\n")
      (insert "        :max (apply #'max vals)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("seed" "xform" "agg"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_table_row_col_access_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Table as data
      (insert "#+NAME: scores\n")
      (insert "| Name | Math | Sci |\n")
      (insert "|------+-----------|\n")
      (insert "| Ada | 95 | 90 |\n")
      (insert "| Bob | 87 | 92 |\n")
      (insert "| Cal | 78 | 85 |\n\n")
      ;; Row access and column compute
      (insert "#+NAME: analyze\n")
      (insert "#+begin_src emacs-lisp :var tbl=scores :results value replace\n")
      (insert "(let ((names (mapcar #'car tbl))\n")
      (insert "      (math (mapcar #'cadr tbl))\n")
      (insert "      (sci (mapcar #'caddr tbl)))\n")
      (insert "  (list :names names\n")
      (insert "        :math-avg (/ (apply #'+ math) (length math))\n")
      (insert "        :sci-avg (/ (apply #'+ sci) (length sci))\n")
      (insert "        :best-math (car (sort (copy-sequence math) #'>))\n")
      (insert "        :best-sci (car (sort (copy-sequence sci) #'>))))\n")
      (insert "#+end_src\n\n")
      ;; Execute
      (dolist (name '("analyze"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Table to lisp
        (goto-char (point-min))
        (search-forward "| Name")
        (let ((table-lisp (org-table-to-lisp)))
          (list (nreverse results)
                table-lisp
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_babel_execute_mapcar_plist_alist_transform_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Plist transform
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((pl (list :a 1 :b 2 :c 3 :d 4)))\n")
      (insert "  (cl-loop for (k v) on pl by #'cddr\n")
      (insert "           collect (cons k (* v v))))\n")
      (insert "#+end_src\n\n")
      ;; Alist filter + transform
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((al '((x . 10) (y . 20) (z . 30) (w . 40))))\n")
      (insert "  (cl-loop for (k . v) in al\n")
      (insert "           when (> v 15)\n")
      (insert "           collect (cons k (list v (* v 2)))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_seq_union_intersection_diff_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Union
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-uniq (append '(1 2 3) '(2 3 4 5)))\n")
      (insert "#+end_src\n\n")
      ;; Intersection
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-intersection '(1 2 3 4 5) '(3 4 5 6 7))\n")
      (insert "#+end_src\n\n")
      ;; Difference
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-difference '(1 2 3 4 5) '(2 4))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_map_table_reduce_pipeline_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Source data as table
      (insert "#+NAME: raw\n")
      (insert "| X | Y |\n|---+---|\n| 2 | 3 |\n| 4 | 5 |\n| 6 | 7 |\n\n")
      ;; Map: compute products
      (insert "#+NAME: products\n")
      (insert "#+begin_src emacs-lisp :var data=raw :results value table replace\n")
      (insert "(cons '(\"X\" \"Y\" \"Product\")\n")
      (insert "      (cons 'hline\n")
      (insert "            (mapcar (lambda (r) (list (car r) (cadr r) (* (car r) (cadr r)))) data)))\n")
      (insert "#+end_src\n\n")
      ;; Reduce: sum products
      (insert "#+NAME: total\n")
      (insert "#+begin_src emacs-lisp :var data=products :results value replace\n")
      (apply #'concat
             (list "(let ((nums (mapcar #'caddr (cdr (memq 'hline data)))))\n"
                   "  (list :count (length nums)\n"
                   "        :total (apply #'+ nums)\n"
                   "        :avg (/ (apply #'+ nums) (length nums))))\n"))
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("products" "total"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Get table lisp
        (goto-char (point-min))
        (search-forward "| X")
        (let ((raw-lisp (org-table-to-lisp)))
          (list (nreverse results)
                raw-lisp
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_babel_execute_struct_access_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Record-like structure via cl-struct
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(progn\n  (cl-defstruct person name age lang)\n")
      (insert "  (let ((p (make-person :name \"Ada\" :age 30 :lang \"elisp\")))\n")
      (insert "    (list (person-name p) (person-age p) (person-lang p)\n")
      (insert "          (person-p p) (cl-typep p 'person))))\n")
      (insert "#+end_src\n\n")
      ;; Nested structure
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((data (list (cons 'users (list (cons 'name \"Ada\") (cons 'age 30)))\n")
      (insert "                 (cons 'scores (list 95 87 92))\n")
      (insert "                 (cons 'meta (list (cons 'v 2) (cons 'ts \"now\"))))))\n")
      (insert "  (list (cdr (assoc 'name (cdr (assoc 'users data))))\n")
      (insert "        (cdr (assoc 'scores data))\n")
      (insert "        (cdr (assoc 'v (cdr (assoc 'meta data))))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_catch_throw_block_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; catch/throw success
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(catch 'done\n  (dotimes (i 10)\n    (when (= i 5) (throw 'done i))))\n")
      (insert "#+end_src\n\n")
      ;; catch/throw with handler
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list\n  (catch 'a (throw 'a 42))\n  (catch 'b (+ 1 2 3))\n  (catch 'c (throw 'c (list 'x 'y))))\n")
      (insert "#+end_src\n\n")
      ;; Nested catch
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(catch 'outer\n  (catch 'inner\n    (throw 'outer 'escaped))\n  'not-reached)\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_cl_loop_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; cl-loop collect
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(cl-loop for i from 1 to 5 collect (* i i))\n")
      (insert "#+end_src\n\n")
      ;; cl-loop with sum
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(cl-loop for x in '(10 20 30 40 50) sum x)\n")
      (insert "#+end_src\n\n")
      ;; cl-loop with when/append
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(cl-loop for i from 1 to 10\n         when (evenp i)\n         collect i into evens\n         finally (return (list evens (length evens))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_multi_type_result_chain_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Number block
      (insert "#+NAME: num\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(+ 10 20 30)\n")
      (insert "#+end_src\n\n")
      ;; String block using num
      (insert "#+NAME: str\n")
      (insert "#+begin_src emacs-lisp :var n=num :results value replace\n")
      (insert "(format \"total=%d\" n)\n")
      (insert "#+end_src\n\n")
      ;; List block using str
      (insert "#+NAME: lst\n")
      (insert "#+begin_src emacs-lisp :var s=str :results value replace\n")
      (insert "(list (length s) (upcase s) (concat s \"!\"))\n")
      (insert "#+end_src\n\n")
      ;; Output block using lst
      (insert "#+NAME: out\n")
      (insert "#+begin_src emacs-lisp :var data=lst :results output replace\n")
      (insert "(dolist (item data)\n  (princ (format \"%s\\n\" item)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("num" "str" "lst" "out"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_list_append_reverse_sort_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; List append and reverse
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((l (list 1 2 3)))\n")
      (insert "  (list (append l '(4 5))\n")
      (insert "        (reverse l)\n")
      (insert "        (append nil l)\n")
      (insert "        (append l nil)))\n")
      (insert "#+end_src\n\n")
      ;; Sort with predicate
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (sort '(3 1 4 1 5 9 2 6) #'<)\n")
      (insert "      (sort '(\"banana\" \"apple\" \"cherry\") #'string<)\n")
      (insert "      (sort '((b . 2) (a . 1) (c . 3))\n")
      (insert "            (lambda (x y) (< (cdr x) (cdr y))))))\n")
      (insert "#+end_src\n\n")
      ;; Nested list operations
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let* ((a '(1 2 3))\n")
      (insert "       (b '(4 5 6))\n")
      (insert "       (c (append a b)))\n")
      (insert "  (list (length c)\n")
      (insert "        (nth 3 c)\n")
      (insert "        (last c 2)\n")
      (insert "        (butlast c 2)\n")
      (insert "        (subseq c 1 4)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_condition_case_unwind_protect_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; condition-case success
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(condition-case err\n    (+ 1 2)\n  (error (list :err (cdr err))))\n")
      (insert "#+end_src\n\n")
      ;; condition-case error
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(condition-case err\n    (/ 1 0)\n  (error (list :caught t :msg (error-message-string err))))\n")
      (insert "#+end_src\n\n")
      ;; unwind-protect
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((result nil) (cleanup nil))\n")
      (insert "  (unwind-protect\n")
      (insert "      (progn (setq result 42) result)\n")
      (insert "    (setq cleanup t))\n")
      (insert "  (list result cleanup))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_multi_block_dependency_chain_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Block A: generates list
      (insert "#+NAME: gen\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(mapcar (lambda (i) (list i (* i i))) (number-sequence 1 5))\n")
      (insert "#+end_src\n\n")
      ;; Block B: transforms gen result
      (insert "#+NAME: xform\n")
      (insert "#+begin_src emacs-lisp :var data=gen :results value replace\n")
      (insert "(mapcar (lambda (r) (cons (car r) (+ (cadr r) 100))) data)\n")
      (insert "#+end_src\n\n")
      ;; Block C: summarizes xform
      (insert "#+NAME: summary\n")
      (insert "#+begin_src emacs-lisp :var data=xform :results value replace\n")
      (insert "(list :count (length data)\n")
      (insert "      :first (car data)\n")
      (insert "      :last (car (last data))\n")
      (insert "      :total (apply #'+ (mapcar #'cdr data)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("gen" "xform" "summary"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read all results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_seq_sort_group_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; seq-sort
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-sort #'< '(5 2 8 1 9 3 7 4 6))\n")
      (insert "#+end_src\n\n")
      ;; seq-group-by
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-group-by #'evenp '(1 2 3 4 5 6 7 8 9 10))\n")
      (insert "#+end_src\n\n")
      ;; seq-take seq-drop seq-take-while
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s '(1 2 3 4 5 6 7 8 9 10)))\n")
      (insert "  (list (seq-take s 3) (seq-drop s 7)\n")
      (insert "        (seq-take-while #'< (list 1 2 3 0 4 5))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_hash_table_operations_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Create and populate hash
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((ht (make-hash-table :test #'equal)))\n")
      (insert "  (puthash \"name\" \"Ada\" ht)\n")
      (insert "  (puthash \"age\" 30 ht)\n")
      (insert "  (puthash \"lang\" \"elisp\" ht)\n")
      (insert "  (list (gethash \"name\" ht)\n")
      (insert "        (gethash \"age\" ht)\n")
      (insert "        (gethash \"missing\" ht \"default\")\n")
      (insert "        (hash-table-count ht)\n")
      (insert "        (remhash \"age\" ht)\n")
      (insert "        (hash-table-count ht)))\n")
      (insert "#+end_src\n\n")
      ;; Hash to alist
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((ht (make-hash-table)))\n")
      (insert "  (puthash 'x 10 ht) (puthash 'y 20 ht) (puthash 'z 30 ht)\n")
      (insert "  (let ((al nil))\n")
      (insert "    (maphash (lambda (k v) (push (cons k v) al)) ht)\n")
      (insert "    (sort al (lambda (a b) (string< (symbol-name (car a))\n")
      (insert "                                      (symbol-name (car b))))))))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_property_list_alist_conversion_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Plist to alist
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((pl (list :a 1 :b 2 :c 3)))\n")
      (insert "  (list (cl-loop for (k v) on pl by #'cddr collect (cons k v))\n")
      (insert "        (plist-get pl :b)\n")
      (insert "        (plist-member pl :c)))\n")
      (insert "#+end_src\n\n")
      ;; Alist to plist
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((al '((x . 10) (y . 20) (z . 30))))\n")
      (insert "  (list (apply #'append al)\n")
      (insert "        (cdr (assq 'y al))\n")
      (insert "        (assoc 'w al)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_number_theory_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; GCD/LCM
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (gcd 12 8) (lcm 12 8) (gcd 0 5))\n")
      (insert "#+end_src\n\n")
      ;; Number predicates
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (numberp 42) (integerp 3.14) (floatp 3.14)\n")
      (insert "      (zerop 0) (plusp 5) (minusp -3) (evenp 4) (oddp 7))\n")
      (insert "#+end_src\n\n")
      ;; Rounding
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (truncate 7 2) (floor 7 2) (ceiling 7 2) (round 7 2)\n")
      (insert "      (truncate -7 2) (floor -7 2) (ceiling -7 2))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_string_split_join_match_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; String split
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(split-string \"hello world foo bar\" \" \")\n")
      (insert "#+end_src\n\n")
      ;; String join
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(mapconcat #'upcase '(\"hello\" \"world\") \"-\")\n")
      (insert "#+end_src\n\n")
      ;; String match
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (string-match \"[0-9]+\" \"abc123def456\")\n")
      (insert "      (match-string 0 \"abc123def456\")\n")
      (insert "      (replace-regexp-in-string \"[0-9]+\" \"#\" \"a1b2c3\"))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_setq_let_star_dolist_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; setq
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(progn (setq a 10 b 20 c 30) (list a b c))\n")
      (insert "#+end_src\n\n")
      ;; let*
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let* ((x 5) (y (* x 2)) (z (+ x y)))\n  (list x y z))\n")
      (insert "#+end_src\n\n")
      ;; dolist accumulation
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((acc nil))\n  (dolist (item '(a b c d e))\n    (push (symbol-name item) acc))\n  (nreverse acc))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_char_alist_plist_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Character operations
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (char-to-string 65) (string-to-char \"B\") ?C (char-to-string 945))\n")
      (insert "#+end_src\n\n")
      ;; Alist
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((al '((x . 10) (y . 20) (z . 30))))\n")
      (insert "  (list (assq 'y al) (rassoc 30 al) (length al)))\n")
      (insert "#+end_src\n\n")
      ;; Plist
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((pl (list :name \"Ada\" :age 30 :lang \"elisp\")))\n")
      (insert "  (list (plist-get pl :name) (plist-get pl :age) (plist-member pl :lang)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_recursive_fib_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Recursive fibonacci
      (insert "#+NAME: fib\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(progn\n  (defun fib (n)\n    (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))\n  (mapcar #'fib '(0 1 2 3 4 5 6 7 8 9 10)))\n")
      (insert "#+end_src\n\n")
      ;; Recursive factorial
      (insert "#+NAME: fact\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(progn\n  (defun fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))\n  (mapcar #'fact '(0 1 2 3 4 5 6 7 8)))\n")
      (insert "#+end_src\n\n")
      ;; Execute
      (dolist (name '("fib" "fact"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_type_coercion_boundary_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Zero
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list :zero 0 :neg -1 :float 3.14 :big 9999999999)\n")
      (insert "#+end_src\n\n")
      ;; Empty structures
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list :nil nil :empty-list '() :empty-str \"\" :empty-vec [])\n")
      (insert "#+end_src\n\n")
      ;; Nested nil
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (cons 'a nil) (cons nil 'b) (list nil nil nil))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_mapcar_filter_reduce_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; mapcar
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(mapcar (lambda (x) (* x x)) '(1 2 3 4 5))\n")
      (insert "#+end_src\n\n")
      ;; filter (remove-if-not)
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-filter (lambda (x) (> x 10)) '(5 12 8 20 3 15))\n")
      (insert "#+end_src\n\n")
      ;; reduce
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(seq-reduce #'+ '(1 2 3 4 5) 0)\n")
      (insert "#+end_src\n\n")
      ;; Combined
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let* ((data '(1 2 3 4 5 6 7 8 9 10))\n")
      (insert "       (evens (seq-filter #'evenp data))\n")
      (insert "       (squares (mapcar (lambda (x) (* x x)) evens))\n")
      (insert "       (total (seq-reduce #'+ squares 0)))\n")
      (insert "  (list :evens evens :squares squares :total total))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 4)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_file_output_tangle_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (require 'ob-tangle)
  (let* ((root (make-temp-file "org-babel-file" t))
         (out-el (expand-file-name "out.el" root))
         (out-txt (expand-file-name "out.txt" root))
         (org-confirm-babel-evaluate nil))
    (unwind-protect
        (with-temp-buffer
          (org-mode)
          ;; File output block
          (insert "#+begin_src emacs-lisp :file " out-txt " :results file\n")
          (insert "(with-temp-file \"" out-txt "\"\n")
          (insert "  (insert \"hello from babel\"))\n")
          (insert "\"" out-txt "\")\n")
          (insert "#+end_src\n\n")
          ;; Tangle block
          (insert "#+begin_src emacs-lisp :tangle " out-el "\n")
          (insert "(defun tangled-func () 42)\n")
          (insert "#+end_src\n\n")
          ;; Execute file block
          (goto-char (point-min))
          (search-forward "begin_src")
          (org-babel-execute-src-block)
          (let ((file-result (org-babel-read-result)))
            ;; Tangle
            (let ((tangle-result (org-babel-tangle)))
              ;; Read outputs
              (let ((txt-content
                     (when (file-exists-p out-txt)
                       (with-temp-buffer
                         (insert-file-contents out-txt)
                         (buffer-string))))
                    (el-content
                     (when (file-exists-p out-el)
                       (with-temp-buffer
                         (insert-file-contents out-el)
                         (buffer-string)))))
                (list (replace-regexp-in-string
                       (regexp-quote root) "<root>"
                       (or file-result "nil"))
                      (mapcar (lambda (f)
                                (replace-regexp-in-string
                                 (regexp-quote root) "<root>" f))
                              tangle-result)
                      (replace-regexp-in-string
                       (regexp-quote root) "<root>"
                       (or txt-content "no-txt"))
                      (replace-regexp-in-string
                       (regexp-quote root) "<root>"
                       (or el-content "no-el"))
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_babel_execute_string_list_table_mixed_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; String result
      (insert "#+NAME: str\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(concat \"Hello\" \" \" \"World\")\n")
      (insert "#+end_src\n\n")
      ;; List result using named ref
      (insert "#+NAME: lst\n")
      (insert "#+begin_src emacs-lisp :var s=str :results value replace\n")
      (insert "(list (length s) (upcase s) (downcase s))\n")
      (insert "#+end_src\n\n")
      ;; Table result using named ref
      (insert "#+NAME: tbl\n")
      (insert "#+begin_src emacs-lisp :var data=lst :results value table replace\n")
      (insert "(cons '(\"Metric\" \"Value\")\n")
      (insert "      (cons 'hline\n")
      (insert "            (mapcar (lambda (v) (list (type-of v) v)) data)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("str" "lst" "tbl"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_let_lambda_defun_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; let binding
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((a 10) (b 20))\n  (+ a b))\n")
      (insert "#+end_src\n\n")
      ;; lambda
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(funcall (lambda (x y) (+ (* x x) (* y y))) 3 4)\n")
      (insert "#+end_src\n\n")
      ;; defun + call
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(progn\n  (defun factorial (n)\n    (if (<= n 1) 1 (* n (factorial (1- n)))))\n  (factorial 10))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_cond_assoc_lookup_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Conditional
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((x 5))\n  (cond ((< x 3) 'small)\n        ((< x 10) 'medium)\n        (t 'large)))\n")
      (insert "#+end_src\n\n")
      ;; Assoc lookup
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((table '((a . 1) (b . 2) (c . 3))))\n  (list (cdr (assoc 'b table))\n        (assoc 'd table)\n        (assq 'a table)))\n")
      (insert "#+end_src\n\n")
      ;; Loop construct
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((acc 0))\n  (dotimes (i 10) (setq acc (+ acc i)))\n  acc)\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_arithmetic_comparison_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Arithmetic
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list :add (+ 1 2 3) :mul (* 4 5) :div (/ 10 3) :mod (% 10 3))\n")
      (insert "#+end_src\n\n")
      ;; Comparison
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (< 1 2 3) (> 3 2 1) (= 5 5) (<= 3 3) (>= 4 3))\n")
      (insert "#+end_src\n\n")
      ;; Math functions
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list (abs -5) (max 1 3 2) (min 5 2 8) (expt 2 10) (sqrt 144))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_string_concat_format_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; String concatenation
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(concat \"Hello\" \" \" \"World\" \" \" \"!\")\n")
      (insert "#+end_src\n\n")
      ;; Format with multiple args
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(format \"name=%s age=%d score=%.2f\" \"Ada\" 30 95.678)\n")
      (insert "#+end_src\n\n")
      ;; String operations
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((s \"Hello World\"))\n")
      (insert "  (list (upcase s) (downcase s) (length s) (substring s 0 5)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 3)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_map_accumulate_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Seed block
      (insert "#+NAME: seed\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'(1 2 3 4 5)\n")
      (insert "#+end_src\n\n")
      ;; Map block
      (insert "#+NAME: mapper\n")
      (insert "#+begin_src emacs-lisp :var data=seed :results value replace\n")
      (insert "(mapcar (lambda (x) (list x (* x x) (* x x x))) data)\n")
      (insert "#+end_src\n\n")
      ;; Accumulate block
      (insert "#+NAME: accumulator\n")
      (insert "#+begin_src emacs-lisp :var data=mapper :results value replace\n")
      (insert "(list :total-squares (apply #'+ (mapcar #'cadr data))\n")
      (insert "      :total-cubes (apply #'+ (mapcar #'caddr data))\n")
      (insert "      :count (length data))\n")
      (insert "#+end_src\n\n")
      ;; Output block
      (insert "#+NAME: displayer\n")
      (insert "#+begin_src emacs-lisp :var acc=accumulator :results output replace\n")
      (insert "(princ (format \"squares=%d cubes=%d n=%d\"\n")
      (insert "               (plist-get acc :total-squares)\n")
      (insert "               (plist-get acc :total-cubes)\n")
      (insert "               (plist-get acc :count)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("seed" "mapper" "accumulator" "displayer"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_complex_list_structure_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Nested structure
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((users . ((name . \"Ada\") (age . 30)))\n")
      (insert "  (scores . (95 87 92 88))\n")
      (insert "  (meta . ((created . \"2026-05-27\") (version . 2))))\n")
      (insert "#+end_src\n\n")
      ;; Process structure
      (insert "#+begin_src emacs-lisp :var data=src_1 :results value replace\n")
      (insert "(list :user-name (cdr (assoc 'name (cdr (assoc 'users data))))\n")
      (insert "      :avg-score (/ (apply #'+ (cdr (assoc 'scores data)))\n")
      (insert "                    (length (cdr (assoc 'scores data))))\n")
      (insert "      :version (cdr (assoc 'version (cdr (assoc 'meta data)))))\n")
      (insert "#+end_src\n\n")
      ;; Execute
      (dotimes (_ 2)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_result_type_boolean_vector_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Boolean true
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "t\n")
      (insert "#+end_src\n\n")
      ;; Boolean false
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "nil\n")
      (insert "#+end_src\n\n")
      ;; Vector
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "[1 2 3 4 5]\n")
      (insert "#+end_src\n\n")
      ;; Hash table
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(let ((ht (make-hash-table)))\n  (puthash 'a 1 ht)\n  (puthash 'b 2 ht)\n  ht)\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 4)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_block_var_ref_result_order_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Producer block
      (insert "#+NAME: producer\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((a . 10) (b . 20) (c . 30))\n")
      (insert "#+end_src\n\n")
      ;; Consumer block with var ref
      (insert "#+NAME: consumer\n")
      (insert "#+begin_src emacs-lisp :var data=producer :results value replace\n")
      (insert "(mapcar (lambda (pair)\n")
      (insert "          (cons (car pair) (* 2 (cdr pair))))\n")
      (insert "        data)\n")
      (insert "#+end_src\n\n")
      ;; Aggregator block
      (insert "#+NAME: aggregator\n")
      (insert "#+begin_src emacs-lisp :var data=consumer :results value replace\n")
      (insert "(apply #'+ (mapcar #'cdr data))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("producer" "consumer" "aggregator"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_table_named_result_header_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Named table
      (insert "#+NAME: scores\n")
      (insert "| Name | Score |\n")
      (insert "|------+-------|\n")
      (insert "| Alice | 95 |\n")
      (insert "| Bob | 87 |\n")
      (insert "| Carol | 92 |\n\n")
      ;; Use table as var
      (insert "#+NAME: analysis\n")
      (insert "#+begin_src emacs-lisp :var tbl=scores :results value replace\n")
      (insert "(let ((scores (mapcar #'cadr tbl)))\n")
      (insert "  (list :count (length scores)\n")
      (insert "        :sum (apply #'+ scores)\n")
      (insert "        :avg (/ (apply #'+ scores) (length scores))\n")
      (insert "        :max (apply #'max scores)\n")
      (insert "        :min (apply #'min scores)))\n")
      (insert "#+end_src\n\n")
      ;; Output from analysis
      (insert "#+NAME: report\n")
      (insert "#+begin_src emacs-lisp :var stats=analysis :results output replace\n")
      (insert "(princ (format \"n=%d sum=%d avg=%d max=%d min=%d\"\n")
      (insert "               (plist-get stats :count)\n")
      (insert "               (plist-get stats :sum)\n")
      (insert "               (plist-get stats :avg)\n")
      (insert "               (plist-get stats :max)\n")
      (insert "               (plist-get stats :min)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("analysis" "report"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Parse table
        (let ((table-lisp
               (progn
                 (goto-char (point-min))
                 (search-forward "| Name")
                 (org-table-to-lisp))))
          (list (nreverse results)
                table-lisp
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_babel_execute_result_insert_update_replace_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Block with placeholder result
      (insert "#+NAME: counter\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(random 1000)\n")
      (insert "#+end_src\n\n")
      (insert "#+RESULTS: counter\n")
      (insert ": placeholder\n\n")
      ;; Execute - should replace placeholder
      (goto-char (point-min))
      (search-forward "counter")
      (org-babel-execute-src-block)
      (let ((after-exec (buffer-substring-no-properties
                         (point-min) (point-max)))
            (result-1 (org-babel-read-result)))
        ;; Execute again - should replace previous result
        (goto-char (point-min))
        (search-forward "counter")
        (org-babel-execute-src-block)
        (let ((after-reexec (buffer-substring-no-properties
                             (point-min) (point-max)))
              (result-2 (org-babel-read-result)))
          ;; Remove result
          (goto-char (point-min))
          (search-forward "counter")
          (org-babel-remove-result)
          (let ((after-remove (buffer-substring-no-properties
                               (point-min) (point-max))))
            ;; Execute again - should create new result
            (goto-char (point-min))
            (search-forward "counter")
            (org-babel-execute-src-block)
            (let ((after-new (buffer-substring-no-properties
                              (point-min) (point-max)))
                  (result-3 (org-babel-read-result)))
              (list after-exec
                    (integerp result-1)
                    after-reexec
                    (integerp result-2)
                    after-remove
                    after-new
                    (integerp result-3))))))))))"##,
    );
}

#[test]
fn org_babel_execute_result_type_string_number_list_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Number
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "42\n")
      (insert "#+end_src\n\n")
      ;; String
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "\"hello world\"\n")
      (insert "#+end_src\n\n")
      ;; Association list
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((name . \"Ada\") (age . 30) (lang . \"elisp\"))\n")
      (insert "#+end_src\n\n")
      ;; Nested list
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((a 1) (b (2 3)) (c (d 4)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dotimes (_ 4)
        (goto-char (point-min))
        (search-forward "begin_src")
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_nested_var_reference_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Source data
      (insert "#+NAME: numbers\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'(1 2 3 4 5)\n")
      (insert "#+end_src\n\n")
      ;; Compute with var ref
      (insert "#+NAME: stats\n")
      (insert "#+begin_src emacs-lisp :var nums=numbers :results value replace\n")
      (insert "(list :count (length nums)\n")
      (insert "      :sum (apply #'+ nums)\n")
      (insert "      :avg (/ (apply #'+ nums) (length nums)))\n")
      (insert "#+end_src\n\n")
      ;; Format output
      (insert "#+NAME: display\n")
      (insert "#+begin_src emacs-lisp :var s=stats :results output replace\n")
      (insert "(princ (format \"count=%d sum=%d avg=%d\"\n")
      (insert "               (plist-get s :count)\n")
      (insert "               (plist-get s :sum)\n")
      (insert "               (plist-get s :avg)))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("numbers" "stats" "display"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_output_format_string_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Multi-line output
      (insert "#+NAME: multiline\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(dotimes (i 5)\n  (princ (format \"line-%d\\n\" i)))\n")
      (insert "#+end_src\n\n")
      ;; Formatted output
      (insert "#+NAME: formatted\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(princ (format \"key=%s val=%d pi=%.2f\" \"test\" 42 3.14159))\n")
      (insert "#+end_src\n\n")
      ;; Execute
      (dolist (name '("multiline" "formatted"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_multiple_named_results_order_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+NAME: first\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'first-val\n")
      (insert "#+end_src\n\n")
      (insert "#+NAME: second\n")
      (insert "#+begin_src emacs-lisp :var prev=first :results value replace\n")
      (insert "(list prev 'second-val)\n")
      (insert "#+end_src\n\n")
      (insert "#+NAME: third\n")
      (insert "#+begin_src emacs-lisp :var prev=second :results output replace\n")
      (insert "(princ (format \"chain=%S\" prev))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dolist (name '("first" "second" "third"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read all results in order
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_noweb_var_chain_output_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Config block
      (insert "#+NAME: config\n")
      (insert "#+begin_src emacs-lisp\n")
      (insert "(defconst base-val 10)\n")
      (insert "#+end_src\n\n")
      ;; Noweb block
      (insert "#+NAME: compute\n")
      (insert "#+begin_src emacs-lisp :var x=3 :noweb yes :results value replace\n")
      (insert "(let ((b (progn <<config>> base-val)))\n")
      (insert "  (* x b))\n")
      (insert "#+end_src\n\n")
      ;; Output block
      (insert "#+NAME: displayer\n")
      (insert "#+begin_src emacs-lisp :var val=compute :results output replace\n")
      (insert "(princ (format \"val=%d\" val))\n")
      (insert "#+end_src\n\n")
      ;; Execute chain
      (dolist (name '("compute" "displayer"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_table_var_format_spec_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Source table
      (insert "#+NAME: data\n")
      (insert "| X | Y |\n|---+---|\n| 1 | 2 |\n| 3 | 4 |\n| 5 | 6 |\n\n")
      ;; Sum block
      (insert "#+NAME: summer\n")
      (insert "#+begin_src emacs-lisp :var tbl=data :results value replace\n")
      (insert "(list :row-count (length tbl)\n")
      (insert "      :x-sum (apply #'+ (mapcar #'car tbl))\n")
      (insert "      :y-sum (apply #'+ (mapcar #'cadr tbl)))\n")
      (insert "#+end_src\n\n")
      ;; Format spec block
      (insert "#+NAME: formatter\n")
      (insert "#+begin_src emacs-lisp :var stats=summer :results output replace\n")
      (insert "(princ (format \"rows=%d x=%d y=%d\"\n")
      (insert "               (plist-get stats :row-count)\n")
      (insert "               (plist-get stats :x-sum)\n")
      (insert "               (plist-get stats :y-sum)))\n")
      (insert "#+end_src\n\n")
      ;; Execute
      (dolist (name '("summer" "formatter"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Parse table
        (let ((table-lisp
               (progn
                 (goto-char (point-min))
                 (search-forward "| X")
                 (org-table-to-lisp))))
          (list (nreverse results)
                table-lisp
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_babel_execute_header_arg_inherit_override_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; File-level property
      (insert "#+PROPERTY: header-args :results value replace\n\n")
      ;; Block inheriting file-level
      (insert "#+NAME: inherited\n")
      (insert "#+begin_src emacs-lisp\n")
      (insert "(+ 1 2)\n")
      (insert "#+end_src\n\n")
      ;; Block with output override
      (insert "#+NAME: output-override\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(princ \"overridden to output\")\n")
      (insert "#+end_src\n\n")
      ;; Block with file override
      (let* ((root (make-temp-file "org-babel-inherit" t))
             (out-file (expand-file-name "result.txt" root)))
        (unwind-protect
            (progn
              (insert "#+NAME: file-override\n")
              (insert "#+begin_src emacs-lisp :file " out-file "\n")
              (insert "(with-temp-file \"" out-file "\"\n  (insert \"file content\"))\n  \"done\"\n")
              (insert "#+end_src\n\n")
              ;; Execute all
              (dolist (name '("inherited" "output-override" "file-override"))
                (goto-char (point-min))
                (search-forward name)
                (org-babel-execute-src-block))
              ;; Read results
              (let ((results nil))
                (goto-char (point-min))
                (while (re-search-forward "#\\+RESULTS:" nil t)
                  (forward-line 1)
                  (push (org-babel-read-result) results))
                (let ((file-content
                       (when (file-exists-p out-file)
                         (with-temp-buffer
                           (insert-file-contents out-file)
                           (buffer-string)))))
                  (list (nreverse results)
                        (replace-regexp-in-string
                         (regexp-quote root) "<root>"
                         (or file-content "no-file"))
                        (replace-regexp-in-string
                         (regexp-quote root) "<root>"
                         (buffer-substring-no-properties
                          (point-min) (point-max))))))
          (delete-directory root t))))))"##,
    );
}

#[test]
fn org_babel_execute_assign_header_var_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Named block with default var
      (insert "#+NAME: doubler\n")
      (insert "#+begin_src emacs-lisp :var n=5 :results value replace\n")
      (insert "(list :input n :doubled (* n 2) :squared (* n n))\n")
      (insert "#+end_src\n\n")
      ;; Override var
      (insert "#+NAME: tripler\n")
      (insert "#+begin_src emacs-lisp :var n=10 :results value replace\n")
      (insert "(list :input n :tripled (* n 3) :squared (* n n))\n")
      (insert "#+end_src\n\n")
      ;; Call with different var
      (insert "#+CALL: doubler(n=20) :results value replace\n\n")
      ;; Execute
      (dolist (name '("doubler" "tripler"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Execute call
      (goto-char (point-min))
      (search-forward "CALL:")
      (org-babel-lob-execute-maybe)
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        (list (nreverse results)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_babel_execute_error_handling_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil)
          (org-babel-default-header-args '((:results . "value replace"))))
      ;; Valid block
      (insert "#+NAME: valid\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(+ 10 20)\n")
      (insert "#+end_src\n\n")
      ;; Block that returns nil
      (insert "#+NAME: niler\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "nil\n")
      (insert "#+end_src\n\n")
      ;; Block that returns empty list
      (insert "#+NAME: emptier\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'()\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dolist (name '("valid" "niler" "emptier"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Get buffer state
        (let ((buf-text (buffer-substring-no-properties
                         (point-min) (point-max)))
              (elements
               (org-element-map (org-element-parse-buffer)
                   '(src-block fixed-width)
                 (lambda (el)
                   (list (org-element-type el)
                         (org-element-property :name el)
                         (org-element-property :value el))))))
          (list (nreverse results)
                elements
                buf-text))))))"##,
    );
}

#[test]
fn org_babel_execute_result_insert_replace_remove_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+NAME: calc\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "(list :a 1 :b 2 :c 3)\n")
      (insert "#+end_src\n\n")
      (insert "#+RESULTS: calc\n")
      (insert ":placeholder\n\n")
      ;; Execute - should replace placeholder
      (goto-char (point-min))
      (search-forward "calc")
      (org-babel-execute-src-block)
      (let ((after-exec (buffer-substring-no-properties
                         (point-min) (point-max)))
            (result-val (org-babel-read-result)))
        ;; Remove result
        (goto-char (point-min))
        (search-forward "calc")
        (org-babel-remove-result)
        (let ((after-remove (buffer-substring-no-properties
                             (point-min) (point-max))))
          ;; Re-execute
          (goto-char (point-min))
          (search-forward "calc")
          (org-babel-execute-src-block)
          (let ((after-reexec (buffer-substring-no-properties
                               (point-min) (point-max)))
                (reexec-val (org-babel-read-result)))
             (list after-exec
                   result-val
                   after-remove
                   after-reexec
                   reexec-val))))))))"##,
    );
}

#[test]
fn org_babel_execute_output_var_list_table_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-core)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      ;; Output mode
      (insert "#+NAME: out\n")
      (insert "#+begin_src emacs-lisp :results output replace\n")
      (insert "(dotimes (i 3)\n  (princ (format \"line-%d\\n\" i)))\n")
      (insert "#+end_src\n\n")
      ;; Value list mode
      (insert "#+NAME: lst\n")
      (insert "#+begin_src emacs-lisp :results value replace\n")
      (insert "'((x 1) (y 2) (z 3))\n")
      (insert "#+end_src\n\n")
      ;; Value table mode
      (insert "#+NAME: tbl\n")
      (insert "#+begin_src emacs-lisp :var data=lst :results value table replace\n")
      (insert "(cons '(\"Key\" \"Val\")\n")
      (insert "      (cons 'hline\n")
      (insert "            (mapcar (lambda (r) (list (car r) (* 10 (cadr r)))) data)))\n")
      (insert "#+end_src\n\n")
      ;; Execute all
      (dolist (name '("out" "lst" "tbl"))
        (goto-char (point-min))
        (search-forward name)
        (org-babel-execute-src-block))
      ;; Read results
      (let ((results nil))
        (goto-char (point-min))
        (while (re-search-forward "#\\+RESULTS:" nil t)
          (forward-line 1)
          (push (org-babel-read-result) results))
        ;; Parse elements
        (let ((elements
               (org-element-map (org-element-parse-buffer)
                   '(src-block fixed-width)
                 (lambda (el)
                   (list (org-element-type el)
                         (org-element-property :name el)
                         (org-element-property :value el))))))
          (list (nreverse results)
                elements
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}
