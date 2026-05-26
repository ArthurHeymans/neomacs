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
