use expect_test::expect;

use super::assert_amd_mode_parity;

#[test]
fn module_name_and_sequential_rewrite_rules_cover_paths_and_plain_names() {
    let elisp_form = r##"
(let ((amd-rewrite-rules-alist
       '(("^src/" . "")
         ("components/" . "ui/")
         ("\\.legacy\\'" . ""))))
  (list
   (mapcar #'amd--module-name
           '("foo.js" "dir/archive.tar.js"
             "no-extension" ".hidden"))
   (mapcar #'amd--rewrite-path
           '("src/components/button.legacy"
             "vendor/components/menu"
             "plain"))))
"##;
    let expect = expect![[
        r#"OK (("foo" "archive.tar" "no-extension" ".hidden") ("ui/button" "vendor/ui/menu" "plain"))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn project_buffer_file_and_directory_helpers_use_real_nested_files() {
    let elisp_form = r##"
(let* ((root (amd-test-project "path-helpers"))
       (buffer
        (amd-test-open root "src/views/page.js" "define([]);")))
  (with-current-buffer buffer
    (let ((default-directory root))
      (list
       (amd--buffer-file-name)
       (amd--buffer-directory)
       (amd--buffer-module)
       (amd--project-file-name
        (expand-file-name "lib/tool.js" root))
       (with-temp-buffer
         (list (amd--buffer-file-name)
               (amd--buffer-directory)))))))
"##;
    let expect = expect![[
        r#"OK ("src/views/page.js" "src/views/" "src/views/page" "lib/tool.js" (nil nil))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn relative_file_policy_matrix_covers_same_child_sibling_parent_and_no_file() {
    let elisp_form = r##"
(let* ((root (amd-test-project "relative-policy"))
       (current
        (amd-test-write root "src/views/page.js" ""))
       (files
        (mapcar
         (lambda (relative)
           (amd-test-write root relative ""))
         '("src/views/page.js"
           "src/views/helper.js"
           "src/views/child/item.js"
           "src/model.js"
           "test/spec.js")))
       (buffer (find-file-noselect current)))
  (with-current-buffer buffer
    (let ((default-directory root))
      (mapcar
       (lambda (settings)
         (let ((amd-use-relative-file-name (car settings))
               (amd-always-use-relative-file-name (cadr settings)))
           (list settings
                 (mapcar #'amd--use-relative-file-name-p files)
                 (mapcar #'amd--file-name files))))
       '((nil nil) (t nil) (nil t))))))
"##;
    let expect = expect![[
        r#"OK (((nil nil) (nil nil nil nil nil) ("src/views/page.js" "src/views/helper.js" "src/views/child/item.js" "src/model.js" "test/spec.js")) ((t nil) (nil nil nil nil nil) ("src/views/page.js" "src/views/helper.js" "src/views/child/item.js" "src/model.js" "test/spec.js")) ((nil t) (nil t t t t) ("src/views/page.js" "./helper.js" "./child/item.js" "../model.js" "../../test/spec.js")))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn module_resolves_existing_files_but_preserves_non_file_module_names() {
    let elisp_form = r##"
(let* ((root (amd-test-project "module-resolution"))
       (current
        (amd-test-write root "src/main.js" ""))
       (target
        (amd-test-write root "src/lib/widget.js" ""))
       (buffer (find-file-noselect current)))
  (with-current-buffer buffer
    (let ((default-directory root)
          (amd-always-use-relative-file-name nil)
          (amd-rewrite-rules-alist
           '(("^src/" . "app/"))))
      (list
       (amd--module target)
       (amd--module "src/lib/widget.js")
       (amd--module "vendor/widget")
       (let ((amd-always-use-relative-file-name t))
         (amd--module target))
       (file-exists-p target)
       (file-relative-name target root)))))
"##;
    let expect = expect![[
        r#"OK ("app/lib/widget" "app/lib/widget" "vendor/widget" "./lib/widget" t "src/lib/widget.js")"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn current_project_file_filter_uses_real_projectile_inventory() {
    let elisp_form = r##"
(let ((root (amd-test-project "project-files")))
  (dolist (relative
           '("src/foo.js" "src/foo.test.js"
             "src/bar.js" "docs/foo.md"))
    (amd-test-write root relative relative))
  (let ((default-directory root))
    (list
     (sort (projectile-current-project-files)
           #'string-lessp)
     (sort (amd--current-files-matching "foo")
           #'string-lessp)
     (amd--current-files-matching "missing"))))
"##;
    let expect = expect![[
        r#"OK (("docs/foo.md" "src/bar.js" "src/foo.js" "src/foo.test.js") ("docs/foo.md" "src/foo.js" "src/foo.test.js") nil)"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn kill_buffer_module_uses_project_path_rewrite_and_exact_kill_ring_value() {
    let elisp_form = r##"
(let* ((root (amd-test-project "kill-buffer-module"))
       (buffer
        (amd-test-open root "src/legacy/foo.js" "define([]);")))
  (with-current-buffer buffer
    (let ((default-directory root)
          (kill-ring nil)
          (amd-rewrite-rules-alist
           '(("^src/" . "")
             ("legacy/" . "modern/"))))
      (list
       (amd-kill-buffer-module)
       kill-ring
       (current-kill 0 t)))))
"##;
    let expect = expect![[r#"OK (nil ("'modern/foo'") "'modern/foo'")"#]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn find_file_matching_opens_selected_project_file_and_runs_projectile_hook() {
    let elisp_form = r##"
(let* ((root (amd-test-project "find-matching"))
       (source (amd-test-write root "src/source.js" "var target;"))
       (target
        (amd-test-write root "lib/target.js" "TARGET CONTENT"))
       events)
  (let ((default-directory root)
        (projectile-find-file-hook
         (list (lambda ()
                 (push
                  (file-name-nondirectory
                   (buffer-file-name))
                  events)))))
    (cl-letf
        (((symbol-function 'projectile-completing-read)
          (lambda (&rest arguments)
            (push arguments events)
            "lib/target.js")))
      (find-file source)
      (amd--find-file-matching "target")
      (list
       (file-relative-name (buffer-file-name) root)
       (buffer-string)
       (nreverse events)))))
"##;
    let expect = expect![[
        r#"OK ("lib/target.js" "TARGET CONTENT" (("Find file: " ("lib/target.js" "src/source.js") :initial-input "target.js") "target.js"))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn file_search_and_replace_regexps_match_real_amd_definitions() {
    let elisp_form = r##"
(let* ((root (amd-test-project "regex-contract"))
       (file (amd-test-write root "src/foo.js" ""))
       (buffer (find-file-noselect file))
       (samples
        '("define(['src/foo'], function(foo) {})"
          "define([\"foo\"], function(foo) {})"
          "require(['foo'], function(foo) {})"
          "define(['foobar'], function(foo) {})")))
  (with-current-buffer buffer
    (let ((default-directory root))
      (let ((search (amd--file-search-regexp "foo"))
            (replace (amd--file-replace-regexp)))
        (list
         search
         replace
         (mapcar
         (lambda (sample)
            (list
             (condition-case error-data
                 (string-match-p search sample)
               (error
                (cons (car error-data)
                      (cdr error-data))))
             (and (string-match replace sample)
                  (match-string 2 sample))))
          samples))))))
"##;
    let expect = expect![[
        r#"OK ("define\\([^])]+['|\"](.*/)?foo['|\"]" "\\(define([^)]+['|\"]\\)\\(.*/foo\\)\\(['|\"]\\)" (((invalid-regexp "Unmatched ( or \\(") "src/foo") ((invalid-regexp "Unmatched ( or \\(") nil) ((invalid-regexp "Unmatched ( or \\(") nil) ((invalid-regexp "Unmatched ( or \\(") nil)))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}
