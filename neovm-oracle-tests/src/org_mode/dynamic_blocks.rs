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
