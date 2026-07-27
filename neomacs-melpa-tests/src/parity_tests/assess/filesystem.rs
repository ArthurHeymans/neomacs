use super::assert_assess_parity;
use expect_test::{Expect, expect};

#[test]
fn related_file_constructor_preserves_extension_and_uses_requested_directory() {
    let elisp_form = r##"
(let* ((directory
        (file-name-as-directory
         (assess-test-path
          "related-target")))
       (generated nil))
  (make-directory directory t)
  (setq generated
        (assess--make-related-file-1
         "/fixture/source.data.el"
         directory))
  (list
   (file-name-directory generated)
   (file-name-extension generated)
   (string-match-p
    "\\`source\\.data[^/]*\\.el\\'"
    (file-name-nondirectory generated))
   (file-exists-p generated)))
"##;
    let expect: Expect = expect![[r#"OK ("[ORACLE-SANDBOX]/related-target/" "el" 0 t)"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn make_related_file_copies_real_content_without_mutating_the_source() {
    let elisp_form = r##"
(let* ((source
        (assess-test-path
         "related/source.txt"))
       (target-directory
        (file-name-as-directory
         (assess-test-path
          "related/copies")))
       related)
  (make-directory
   (file-name-directory source)
   t)
  (make-directory target-directory t)
  (with-temp-file source
    (insert "original λ\n"))
  (setq related
        (assess-make-related-file
         source
         target-directory))
  (with-temp-file related
    (insert "changed copy\n"))
  (list
   (assess-test-read-file source)
   (assess-test-read-file related)
   (equal
    (file-name-extension source)
    (file-name-extension related))
   (equal
    (file-name-directory related)
    target-directory)
   (not (equal source related))))
"##;
    let expect: Expect = expect![[r#"OK ("original \316\273\n" "changed copy\n" t t t)"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn find_file_macro_runs_visit_hooks_and_kills_new_buffer_after_success() {
    let elisp_form = r##"
(let* ((path
        (assess-test-path
         "visiting/input.el"))
       (hook-calls nil)
       visited
       live-after)
  (make-directory
   (file-name-directory path)
   t)
  (with-temp-file path
    (insert "(message \"before\")\n"))
  (let ((find-file-hook
         (list
          (lambda ()
            (setq hook-calls
                  (cons
                   (list
                    major-mode
                    (buffer-file-name))
                   hook-calls))))))
    (setq visited
          (assess-with-find-file path
            (goto-char (point-max))
            (insert ";; edited only in memory\n")
            (list
             (buffer-name)
             major-mode
             (buffer-modified-p)
             (buffer-string)))))
  (setq live-after
        (find-buffer-visiting path))
  (list
   visited
   (mapcar
    (lambda (entry)
      (list
       (car entry)
       (file-name-nondirectory
        (cadr entry))))
    hook-calls)
   live-after
   (assess-test-read-file path)))
"##;
    let expect: Expect = expect![[
        r#"OK (("input.el" emacs-lisp-mode t #("(message \"before\")\n;; edited only in memory\n" 0 19 (fontified nil) 19 44 (fontified nil))) ((emacs-lisp-mode "input.el")) nil "(message \"before\")\n")"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn find_file_macro_preserves_preexisting_visiting_buffer_and_restores_it_after_signal() {
    let elisp_form = r##"
(let* ((path
        (assess-test-path
         "visiting/existing.txt"))
       (existing nil)
       condition)
  (make-directory
   (file-name-directory path)
   t)
  (with-temp-file path
    (insert "disk"))
  (setq existing
        (find-file-noselect path))
  (unwind-protect
      (progn
        (with-current-buffer existing
          (goto-char (point-max))
          (insert " memory")
          (set-buffer-modified-p t))
        (setq condition
              (condition-case error-data
                  (assess-with-find-file path
                    (signal
                     'assess-deliberate-error
                     '(visit)))
                (assess-deliberate-error
                 error-data)))
        (list
         condition
         (buffer-live-p existing)
         (eq existing
             (find-buffer-visiting path))
         (with-current-buffer existing
           (list
            (buffer-string)
            (buffer-modified-p)))
         (assess-test-read-file path)))
    (with-current-buffer existing
      (set-buffer-modified-p nil))
    (kill-buffer existing)))
"##;
    let expect: Expect =
        expect![[r#"OK ((assess-deliberate-error visit) t t ("disk memory" t) "disk")"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn filesystem_initializers_build_nested_empty_content_and_recursive_specs() {
    let elisp_form = r##"
(let ((root
       (assess-test-path
        "initializer")))
  (make-directory root t)
  (dolist
      (spec
       '("empty"
         "nested/empty"
         "nested/directory/"
         ("payload.txt" "alpha\nbeta")
         ("tree"
          ("leaf"
           ("deep/value" "λ")))))
    (assess-with-filesystem--init
     spec
     root))
  (list
   (file-regular-p
    (expand-file-name "empty" root))
   (file-regular-p
    (expand-file-name
     "nested/empty"
     root))
   (file-directory-p
    (expand-file-name
     "nested/directory"
     root))
   (assess-test-read-file
    (expand-file-name
     "payload.txt"
     root))
   (file-regular-p
    (expand-file-name
     "tree/leaf"
     root))
   (assess-test-read-file
    (expand-file-name
     "tree/deep/value"
     root))))
"##;
    let expect: Expect = expect![[r#"OK (t t t "alpha\nbeta" t "\316\273")"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn filesystem_parent_helper_creates_only_for_nested_file_specs() {
    let elisp_form = r##"
(let ((root
       (assess-test-path
        "parents")))
  (make-directory root t)
  (list
   (assess-with-filesystem--make-parent
    "one/two/file.txt"
    root)
   (file-directory-p
    (expand-file-name
     "one/two"
     root))
   (assess-with-filesystem--make-parent
    "top-level.txt"
    root)
   (directory-files
    root nil
    "\\`[^.]")))
"##;
    let expect: Expect = expect![[r#"OK (nil t nil ("one"))"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn filesystem_initializer_rejects_directory_content_and_unknown_spec_types() {
    let elisp_form = r##"
(let ((root
       (assess-test-path
        "invalid-specs")))
  (make-directory root t)
  (mapcar
   (lambda (spec)
     (condition-case condition
         (progn
           (assess-with-filesystem--init
            spec root)
           :no-signal)
       (error
        (list
         (car condition)
         (cadr condition)))))
   '(("directory/" "forbidden")
     42
     (:keyword "value"))))
"##;
    let expect: Expect = expect![[
        r#"OK ((error "Invalid syntax: ‘directory/’ - cannot create a directory with text content") (error "Invalid syntax: ‘42’") (error "Invalid syntax: ‘:keyword’"))"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn filesystem_macro_exposes_a_real_working_tree_then_removes_it_and_restores_directory() {
    let elisp_form = r##"
(let ((before default-directory)
      during
      root)
  (setq during
        (assess-with-filesystem
            '("empty"
              "dir/"
              ("dir/data.txt"
               "alpha\n")
              ("recursive"
               ("leaf"
                ("deep/value"
                 "beta\n"))))
          (setq root default-directory)
          (with-temp-file
              "dir/generated.txt"
            (insert "generated\n"))
          (list
           (sort
            (mapcar
             (lambda (path)
               (file-relative-name
                path
                default-directory))
             (directory-files-recursively
              default-directory
              ".*"))
            #'string<)
           (assess-test-read-file
            "dir/data.txt")
           (assess-test-read-file
            "dir/generated.txt")
           (assess-test-read-file
            "recursive/deep/value"))))
  (list
   during
   (file-exists-p root)
   (equal before default-directory)))
"##;
    let expect: Expect = expect![[
        r#"OK ((("dir/data.txt" "dir/generated.txt" "empty" "recursive/deep/value" "recursive/leaf") "alpha\n" "generated\n" "beta\n") nil t)"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn filesystem_macro_removes_tree_and_restores_directory_after_nonlocal_exit() {
    let elisp_form = r##"
(let ((before default-directory)
      root
      condition)
  (setq condition
        (condition-case data
            (assess-with-filesystem
                '(("state.txt" "before"))
              (setq root default-directory)
              (with-temp-file
                  "state.txt"
                (insert "after"))
              (signal
               'assess-deliberate-error
               '(filesystem)))
          (assess-deliberate-error
           data)))
  (list
   condition
   (file-exists-p root)
   (equal before default-directory)))
"##;
    let expect: Expect = expect!["OK ((assess-deliberate-error filesystem) nil t)"];
    assert_assess_parity(elisp_form, expect);
}
