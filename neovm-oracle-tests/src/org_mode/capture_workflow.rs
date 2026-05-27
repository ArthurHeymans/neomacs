use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_capture_table_line_and_plain_append_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file
                "org-capture-table" nil ".org"
                "* Log\n| When | Text |\n|------|------|\n"))
         (org-capture-templates
          `(("t" "Table" table-line
             (file+headline ,file "Log")
             "| %u | %i |"
             :empty-lines 0)
            ("p" "Plain" plain
             (file+headline ,file "Log")
             "Plain: %i\n"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "row text" "t")
          (org-capture-finalize)
          (org-capture-string "plain text" "p")
          (org-capture-finalize)
          (with-temp-buffer
            (insert-file-contents file)
            (replace-regexp-in-string
             "\\[[0-9-]+ [A-Za-z]+\\]"
             "[date]"
             (buffer-string))))
      (dolist (buf '("CAPTURE-org-capture-table" "CAPTURE-org-capture-table.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_item_checkitem_prepend_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file "org-capture-items" nil ".org"
                               "* List\n- existing\n"))
         (org-capture-templates
          `(("i" "Item" item
             (file+headline ,file "List")
             "%i"
             :prepend t
             :empty-lines 0)
            ("c" "Check" checkitem
             (file+headline ,file "List")
             "%i"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "first" "i")
          (org-capture-finalize)
          (org-capture-string "done-ish" "c")
          (org-capture-finalize)
          (with-temp-buffer
            (insert-file-contents file)
            (buffer-string)))
      (dolist (buf '("CAPTURE-org-capture-items" "CAPTURE-org-capture-items.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_template_expand_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (with-temp-buffer
    (let ((buffer-file-name "/tmp/source.org"))
      (org-mode)
      (insert "* Source\nBody\n")
      (goto-char (point-min))
      (search-forward "Source")
      (let ((org-capture-plist
             (list :template
                   "* TODO %i\nFrom: %a\nFile: %F\nName: %f\nTime: %<%Y-%m-%d>\n%(upcase \"ok\")\n"
                   :initial "Initial text"
                   :annotation "[[file:/tmp/source.org::*Source][Source]]"
                   :original-file "/tmp/source.org"
                   :original-file-nondirectory "source.org"
                   :default-time (encode-time 0 30 9 27 5 2026)
                   :buffer (current-buffer))))
        (org-capture-fill-template)))))"##,
    );
}

#[test]
fn org_capture_olp_datetree_week_clock_template_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (require 'org-datetree)
  (require 'org-clock)
  (let* ((file (make-temp-file "org-capture-datetree" nil ".org"
                               "* Journal\n** Work\n"))
         (org-overriding-default-time (encode-time 0 30 9 27 5 2026))
         (org-capture-templates
          `(("w" "Week" entry
             (file+olp+datetree ,file "Journal" "Work")
             "* TODO %?%i\nCreated: %U\n"
             :tree-type week
             :clock-in t
             :clock-keep nil
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "Captured body" "w")
          (let ((during (list (org-clocking-p)
                              org-clock-current-task
                              (and (markerp org-clock-marker)
                                   (marker-buffer org-clock-marker)))))
            (org-capture-finalize)
            (with-temp-buffer
              (insert-file-contents file)
              (list during
                    (org-clocking-p)
                    (replace-regexp-in-string
                     "=> +[-0-9:]+"
                     "=> [duration]"
                     (replace-regexp-in-string
                      "\\[[0-9][^]\n]+\\]"
                      "[stamp]"
                      (buffer-string)))))))
      (dolist (buf '("CAPTURE-org-capture-datetree"
                     "CAPTURE-org-capture-datetree.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_regexp_function_prepend_append_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (let* ((file (make-temp-file "org-capture-targets" nil ".org"
                               "* Inbox\n:marker:\nold\n:end:\n* Tail\n"))
         (finder
          (lambda ()
            (goto-char (point-min))
            (search-forward "* Tail")
            (end-of-line)))
         (org-capture-templates
          `(("r" "Regexp prepend" plain
             (file+regexp ,file ":marker:")
             "REGEXP:%i\n"
             :prepend t
             :empty-lines 0)
            ("f" "Function append" entry
             (file+function ,file ,finder)
             "* FUNCTION %i\n"
             :empty-lines 0))))
    (unwind-protect
        (progn
          (org-capture-string "one" "r")
          (let ((regexp-pos (marker-position org-capture-last-stored-marker)))
            (org-capture-finalize)
            (org-capture-string "two" "f")
            (let ((function-pos (marker-position org-capture-last-stored-marker)))
              (org-capture-finalize)
              (with-temp-buffer
                (insert-file-contents file)
                (list regexp-pos
                      function-pos
                      (buffer-string))))))
      (dolist (buf '("CAPTURE-org-capture-targets"
                     "CAPTURE-org-capture-targets.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_capture_clock_target_resume_and_links_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-capture)
  (require 'org-clock)
  (let* ((file (make-temp-file "org-capture-clock" nil ".org"
                               "* TODO Clocked\nBody\n* Notes\n"))
         (org-capture-templates
          `(("c" "Clock note" entry
             (clock)
             "* NOTE %?\nFrom clock: %k\nLink: %K\nInitial: %i\n"
             :clock-in t
             :clock-resume t
             :empty-lines 0))))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (let ((org-clock-into-drawer "LOGBOOK")
                (org-clock-history-length 5)
                (org-clock-persist nil))
            (goto-char (point-min))
            (search-forward "Clocked")
            (beginning-of-line)
            (org-clock-in nil (encode-time 0 0 9 27 5 2026))
            (let ((before-task org-clock-current-task))
              (org-capture-string "captured text" "c")
              (let ((capture-task org-clock-current-task)
                    (capture-running (org-clocking-p)))
                (org-capture-finalize)
                (let ((after-task org-clock-current-task)
                      (after-running (org-clocking-p)))
                  (when (org-clocking-p)
                    (org-clock-out nil t (encode-time 0 45 9 27 5 2026)))
                  (save-buffer)
                  (list before-task
                        capture-task
                        capture-running
                        after-task
                        after-running
                        (replace-regexp-in-string
                         (regexp-quote file)
                         "<file>"
                         (replace-regexp-in-string
                          "=> +[-0-9:]+"
                          "=> [duration]"
                          (replace-regexp-in-string
                           "\\[[0-9][^]\n]+\\]"
                           "[stamp]"
                           (buffer-substring-no-properties
                            (point-min) (point-max)))))))))))
      (dolist (buf '("CAPTURE-org-capture-clock"
                     "CAPTURE-org-capture-clock.org"))
        (when (get-buffer buf) (kill-buffer buf)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}
