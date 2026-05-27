use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_pcomplete_file_options_startup_tags_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-pcomplete)
  (require 'ox)
  (with-temp-buffer
    (let ((org-tag-alist '(("work" . ?w) ("home" . ?h) (:startgroup)
                           ("a" . ?a) ("b" . ?b) (:endgroup)))
          (org-export-select-tags '("export" "ship"))
          (org-export-exclude-tags '("noexport" "draft"))
          (user-full-name "Ada Lovelace")
          (user-mail-address "ada@example.invalid"))
      (org-mode)
      (org-set-regexps-and-options)
      (cl-labels
          ((complete-at
            (needle)
            (goto-char (point-min))
            (search-forward needle)
            (let* ((capf (run-hook-with-args-until-success
                          'completion-at-point-functions))
                   (beg (nth 0 capf))
                   (end (nth 1 capf))
                   (table (nth 2 capf))
                   (stub (buffer-substring-no-properties beg end)))
              (list stub
                    (sort (all-completions stub table) #'string<)))))
        (insert "#+STA\n")
        (insert "#+STARTUP: hid\n")
        (insert "#+TAGS: \n")
        (insert "#+SELECT_TAGS: \n")
        (insert "#+EXCLUDE_TAGS: \n")
        (insert "#+AUTHOR: \n")
        (list (complete-at "#+STA")
              (complete-at "hid")
              (complete-at "#+TAGS: ")
              (complete-at "#+SELECT_TAGS: ")
              (complete-at "#+EXCLUDE_TAGS: ")
              (complete-at "#+AUTHOR: "))))))"##,
    );
}

#[test]
fn org_pcomplete_heading_todo_tag_property_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-pcomplete)
  (with-temp-buffer
    (let ((org-todo-keywords '((sequence "TODO" "NEXT" "WAIT" "|" "DONE")))
          (org-tag-alist '(("work" . ?w) ("urgent" . ?u) ("blocked" . ?b))))
      (org-mode)
      (insert "#+PROPERTY: Effort_ALL 0:15 0:30 1:00\n")
      (insert "* TODO Alpha :work:\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "** NEXT Beta :urgent:\n")
      (insert "* TO\n")
      (insert "* Plain :ur\n")
      (insert ":PROPERTIES:\n:Eff\n:END:\n")
      (org-set-regexps-and-options)
      (cl-labels
          ((complete-at
            (needle)
            (goto-char (point-min))
            (search-forward needle)
            (let* ((capf (run-hook-with-args-until-success
                          'completion-at-point-functions))
                   (beg (nth 0 capf))
                   (end (nth 1 capf))
                   (table (nth 2 capf))
                   (stub (buffer-substring-no-properties beg end)))
              (list stub
                    (sort (all-completions stub table) #'string<)
                    (org-thing-at-point)
                    (org-command-at-point)))))
        (list (complete-at "* TO")
              (complete-at ":ur")
              (complete-at ":Eff")))))"##,
    );
}

#[test]
fn org_pcomplete_link_drawer_src_block_option_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-pcomplete)
  (require 'ob-core)
  (with-temp-buffer
    (let ((org-link-abbrev-alist-local '(("issue" . "https://example/%s")))
          (org-link-abbrev-alist '(("doc" . "file:%s")))
          (org-babel-load-languages '((emacs-lisp . t) (shell . t))))
      (org-mode)
      (insert "[[is\n")
      (insert "* H\n:LOG\n:END:\n:PRO\n")
      (insert "#+begin_src emacs-lisp :res\n(+ 1 2)\n#+end_src\n")
      (insert "#+BEGIN: clocktable :sc\n#+END:\n")
      (cl-labels
          ((complete-at
            (needle)
            (goto-char (point-min))
            (search-forward needle)
            (let* ((capf (run-hook-with-args-until-success
                          'completion-at-point-functions))
                   (beg (nth 0 capf))
                   (end (nth 1 capf))
                   (table (nth 2 capf))
                   (stub (buffer-substring-no-properties beg end)))
              (list stub
                    (sort (all-completions stub table) #'string<)
                    (org-thing-at-point)
                    (org-command-at-point)))))
        (list (complete-at "[[is")
              (complete-at ":PRO")
              (complete-at ":res")
              (complete-at ":sc"))))))"##,
    );
}
