use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_custom_dynamic_block_insert_update_all_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-dynamic-block-alist nil))
    (org-dynamic-block-define
     "probe"
     (lambda (params)
       (insert
        (format "| key | value |\n| name | %s |\n| limit | %S |\n"
                (plist-get params :name)
                (plist-get params :limit)))))
    (with-temp-buffer
      (org-mode)
      (insert "* Blocks\n")
      (org-dynamic-block-insert-dblock "probe")
      (goto-char (point-min))
      (search-forward "#+BEGIN: probe")
      (end-of-line)
      (insert " :limit 3")
      (org-update-dblock)
      (insert "\n#+BEGIN: probe :limit (1 2)\nstale\n#+END:\n")
      (org-update-all-dblocks)
      (list (org-dynamic-block-types)
            (functionp (org-dynamic-block-function "probe"))
            (org-find-dblock "probe")
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_dblock_prepare_nested_content_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (defun org-dblock-write:probe-prepare (params)
    (insert (format "- heading :: %s\n" (plist-get params :heading)))
    (insert "  #+begin_example\n  example\n  #+end_example\n"))
  (with-temp-buffer
    (org-mode)
    (insert "#+BEGIN: probe-prepare :heading \"A B\"\n")
    (insert "old line\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
    (insert "#+END:\n\n")
    (insert "* After\n")
    (goto-char (point-min))
    (let ((prepared (org-prepare-dblock)))
      (org-update-dblock)
      (let ((inside (save-excursion
                      (search-backward "#+BEGIN: probe")
                      (org-beginning-of-dblock)
                      (org-in-block-p '("probe-prepare")))))
        (list prepared
              inside
              (buffer-substring-no-properties
               (point-min) (point-max))
              (org-element-map (org-element-parse-buffer)
                  '(dynamic-block plain-list example-block headline)
                (lambda (e)
                  (list (org-element-type e)
                        (org-element-property :begin e)
                        (org-element-property :end e))))))))"##,
    );
}

#[test]
fn org_clocktable_dblock_shift_steps_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* Project\n")
    (insert "** Alpha\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:00] =>  1:00\n")
    (insert "** Beta\n")
    (insert "CLOCK: [2026-05-28 Thu 11:00]--[2026-05-28 Thu 12:30] =>  1:30\n\n")
    (insert "#+BEGIN: clocktable :scope file :block 2026-05-27 :maxlevel 3 :link nil :step daysteps\n")
    (insert "#+END:\n")
    (goto-char (point-min))
    (search-forward "#+BEGIN: clocktable")
    (beginning-of-line)
    (let ((steps-before
           (mapcar (lambda (pair)
                     (list (format-time-string "%F" (car pair))
                           (format-time-string "%F" (cdr pair))))
                   (org-clocktable-steps
                    '(:block "2026-05-27" :step daysteps)))))
      (org-update-dblock)
      (let ((after-update
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "#+BEGIN: clocktable")
        (beginning-of-line)
        (org-clocktable-shift 'right 1)
        (list steps-before
              after-update
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_columnview_dblock_filters_tblfm_links_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-colview)
  (with-temp-buffer
    (org-mode)
    (insert "#+COLUMNS: %25ITEM(Task) %TODO(State) %Effort{:} %Owner %Score{+}\n")
    (insert "#+PROPERTY: Owner_ALL Ada Bea Cal\n")
    (insert "* Project :root:\n")
    (insert ":PROPERTIES:\n:Owner: Ada\n:Score: 0\n:Effort: 0:00\n:END:\n")
    (insert "** TODO Alpha :keep:\n")
    (insert ":PROPERTIES:\n:Effort: 1:15\n:Owner: Bea\n:Score: 2\n:END:\n")
    (insert "** DONE Beta :skip:keep:\n")
    (insert ":PROPERTIES:\n:Effort: 0:45\n:Owner: Cal\n:Score: 3\n:END:\n")
    (insert "** TODO Gamma :keep:\n")
    (insert ":PROPERTIES:\n:Effort: 0:30\n:Score: 4\n:END:\n\n")
    (insert "#+BEGIN: columnview :id local :match \"+keep\" ")
    (insert ":exclude-tags (\"skip\") :maxlevel 3 :skip-empty-rows t ")
    (insert ":indent t :hlines 2 :vlines t :link t ")
    (insert ":format \"%25ITEM(Task) %TODO(State) %Effort{:} %Owner %Score{+}\"\n")
    (insert "#+CAPTION: kept caption\n")
    (insert "| stale | data |\n")
    (insert "#+TBLFM: @>$6=vsum(@3..@-1)\n")
    (insert "#+END:\n")
    (goto-char (point-min))
    (search-forward "#+BEGIN: columnview")
    (beginning-of-line)
    (let ((start (point))
          before-table after-table ast)
      (org-update-dblock)
      (goto-char start)
      (setq before-table
            (progn
              (search-forward "|")
              (org-table-to-lisp)))
      (goto-char start)
      (setq ast
            (org-element-map (org-element-parse-buffer)
                '(dynamic-block table keyword)
              (lambda (e)
                (list (org-element-type e)
                      (org-element-property :key e)
                      (org-element-property :value e)
                      (org-element-property :name e)
                      (org-element-property :begin e)
                      (org-element-property :end e)))))
      (search-forward "#+TBLFM:")
      (end-of-line)
      (insert "::@4$6=@3$6+10")
      (goto-char start)
      (search-forward "|")
      (org-table-recalculate 'all t)
      (setq after-table (org-table-to-lisp))
      (list before-table
            after-table
            ast
            org-columns-current-fmt-compiled
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_custom_dblock_nested_table_rewrite_all_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-dynamic-block-alist nil)
        (calls nil))
    (defun org-dblock-write:oracle-matrix (params)
      (push (list (plist-get params :name)
                  (plist-get params :label)
                  (plist-get params :rows)
                  (plist-get params :content)
                  (plist-get params :indentation-column))
            calls)
      (let ((label (plist-get params :label))
            (rows (plist-get params :rows)))
        (insert (format "- label :: %s\n" label))
        (insert "| idx | value | double |\n")
        (insert "|-----+-------+--------|\n")
        (cl-loop for row in rows
                 for idx from 1
                 do (insert (format "| %d | %s | stale |\n" idx row)))
        (insert "#+TBLFM: $3=$2*2\n")))
    (org-dynamic-block-define
     "oracle-matrix"
     (lambda ()
       (interactive)
       (org-create-dblock
        '(:name "oracle-matrix" :label "inserted" :rows (4 5)))))
    (with-temp-buffer
      (org-mode)
      (insert "* Blocks\n")
      (insert "  #+BEGIN: oracle-matrix :label \"alpha\" :rows (1 2 3)\n")
      (insert "  old alpha\n")
      (insert "  #+END:\n\n")
      (insert "** Child\n")
      (insert "#+BEGIN: oracle-matrix :label \"beta\" :rows (7 8)\n")
      (insert "| stale | table |\n")
      (insert "#+END:\n\n")
      (goto-char (point-max))
      (org-dynamic-block-insert-dblock "oracle-matrix")
      (let ((types-before (org-dynamic-block-types))
            prepared-alpha after-alpha after-all ast table-summaries)
        (goto-char (point-min))
        (search-forward "alpha")
        (org-beginning-of-dblock)
        (setq prepared-alpha (org-prepare-dblock))
        (org-dblock-write:oracle-matrix prepared-alpha)
        (search-backward "#+BEGIN: oracle-matrix")
        (org-update-dblock)
        (setq after-alpha
              (buffer-substring-no-properties
               (point-min) (point-max)))
        (org-update-all-dblocks)
        (setq after-all
              (buffer-substring-no-properties
               (point-min) (point-max)))
        (setq table-summaries
              (let (out)
                (goto-char (point-min))
                (while (search-forward "| idx |" nil t)
                  (org-table-align)
                  (org-table-recalculate 'all)
                  (push (org-table-to-lisp) out))
                (nreverse out)))
        (setq ast
              (org-element-map (org-element-parse-buffer)
                  '(dynamic-block plain-list table table-row headline)
                (lambda (e)
                  (list (org-element-type e)
                        (org-element-property :name e)
                        (org-element-property :begin e)
                        (org-element-property :end e)))))
        (list types-before
              (functionp (org-dynamic-block-function "oracle-matrix"))
              prepared-alpha
              (nreverse calls)
              after-alpha
              after-all
              table-summaries
              ast
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_clocktable_properties_shift_recalc_ast_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "#+COLUMNS: %25ITEM %Owner %Effort{:}\n")
    (insert "* Project :root:\n")
    (insert ":PROPERTIES:\n:Owner: Ada\n:Effort: 0:00\n:END:\n")
    (insert "** TODO Alpha :work:\n")
    (insert ":PROPERTIES:\n:Owner: Bea\n:Effort: 1:00\n:END:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
    (insert "** TODO Beta :home:\n")
    (insert ":PROPERTIES:\n:Owner: Cy\n:Effort: 0:45\n:END:\n")
    (insert "CLOCK: [2026-05-28 Thu 11:15]--[2026-05-28 Thu 12:00] =>  0:45\n")
    (insert "*** TODO Beta child :work:\n")
    (insert ":PROPERTIES:\n:Owner: Dee\n:Effort: 0:30\n:END:\n")
    (insert "CLOCK: [2026-05-29 Fri 14:00]--[2026-05-29 Fri 15:15] =>  1:15\n\n")
    (insert "#+BEGIN: clocktable :scope file :block 2026-05-27 :step daysteps ")
    (insert ":maxlevel 4 :link nil :tags t :properties (\"Owner\" \"Effort\") ")
    (insert ":formula % :compact nil\n")
    (insert "#+CAPTION: Clock rollup\n")
    (insert "| stale | data |\n")
    (insert "#+END:\n")
    (goto-char (point-min))
    (search-forward "#+BEGIN: clocktable")
    (beginning-of-line)
    (let ((steps
           (mapcar (lambda (pair)
                     (list (format-time-string "%F" (car pair))
                           (format-time-string "%F" (cdr pair))))
                   (org-clocktable-steps
                    '(:block "2026-05-27" :step daysteps))))
          before-shift after-right after-left ast tables data)
      (org-update-dblock)
      (setq before-shift
            (buffer-substring-no-properties (point-min) (point-max)))
      (goto-char (point-min))
      (search-forward "#+BEGIN: clocktable")
      (beginning-of-line)
      (setq tables
            (let (out)
              (while (search-forward "| File" nil t)
                (org-table-align)
                (push (org-table-to-lisp) out))
              (nreverse out)))
      (setq data
            (org-clock-get-table-data
             (current-buffer)
             '(:maxlevel 4 :tags t :properties ("Owner" "Effort"))))
      (goto-char (point-min))
      (search-forward "#+BEGIN: clocktable")
      (beginning-of-line)
      (org-clocktable-shift 'right 1)
      (setq after-right
            (buffer-substring-no-properties (point-min) (point-max)))
      (goto-char (point-min))
      (search-forward "#+BEGIN: clocktable")
      (beginning-of-line)
      (org-clocktable-shift 'left 1)
      (setq after-left
            (buffer-substring-no-properties (point-min) (point-max)))
      (setq ast
            (org-element-map (org-element-parse-buffer)
                '(dynamic-block table keyword headline)
              (lambda (e)
                (list (org-element-type e)
                      (org-element-property :block-name e)
                      (org-element-property :key e)
                      (org-element-property :value e)
                      (org-element-property :raw-value e)
                      (org-element-property :begin e)
                      (org-element-property :end e)))))
      (list steps
            (mapcar (lambda (needle)
                      (not (null
                            (string-match-p needle before-shift))))
                    '("Clock rollup" "Alpha" "Beta" "Owner" "Effort"
                      "1:30" "0:45"))
            tables
            data
            (mapcar (lambda (needle)
                      (not (null
                            (string-match-p needle after-right))))
                    '("2026-05-28" "Beta" "0:45"))
            (mapcar (lambda (needle)
                      (not (null
                            (string-match-p needle after-left))))
                    '("2026-05-27" "Alpha" "1:30"))
            ast
            before-shift
            after-right
            after-left))))"##,
    );
}
