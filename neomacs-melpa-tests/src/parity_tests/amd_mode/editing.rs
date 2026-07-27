use expect_test::expect;

use super::assert_amd_mode_parity;

#[test]
fn auto_insert_creates_parseable_default_module_and_places_point_in_body() {
    let elisp_form = r##"
(let ((root (amd-test-project "auto-insert")))
  (with-temp-buffer
    (let ((default-directory root))
      (amd-auto-insert)
      (amd-test-parse)
      (list
       (buffer-string)
       (point)
       (line-number-at-pos)
       (current-column)
       (amd--imported-modules)
       amd-mode
       major-mode))))
"##;
    let expect = expect![[
        r#"OK (#("define([], function() {\n    \n});\n" 0 6 (font-lock-face js2-function-call) 11 19 (font-lock-face font-lock-keyword-face)) 29 2 4 ("") t js2-mode)"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn import_plain_module_into_empty_definition_updates_array_and_function_params() {
    let elisp_form = r##"
(let ((root (amd-test-project "import-module")))
  (with-temp-buffer
    (let ((default-directory root))
      (insert "define([], function() {\n});\n")
      (amd-test-parse)
      (cl-letf (((symbol-function 'read-string)
                 (lambda (&rest _) "")))
        (amd--import "lib/foo"))
      (js2-parse)
      (list
       (buffer-string)
       (amd--imported-modules)
       (amd--number-of-named-modules)))))
"##;
    let expect = expect![[
        r#"OK (#("define(['lib/foo'], function(foo) {\n});\n" 0 6 (font-lock-face js2-function-call) 8 17 (font-lock-face font-lock-string-face) 20 28 (font-lock-face font-lock-keyword-face) 29 32 (font-lock-face js2-function-param)) ("foo") 1)"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn import_existing_child_file_uses_project_path_and_prompted_alias() {
    let elisp_form = r##"
(let* ((root (amd-test-project "import-file"))
       (target (amd-test-write root "src/lib/widget.js" ""))
       (buffer
        (amd-test-open
         root "src/main.js"
         "define([], function() {\n});\n")))
  (with-current-buffer buffer
    (let ((default-directory root)
          (amd-use-relative-file-name nil)
          prompts)
      (amd-test-parse)
      (cl-letf (((symbol-function 'read-string)
                 (lambda (&rest arguments)
                   (push arguments prompts)
                   "widgetAlias")))
        (amd--import target))
      (js2-parse)
      (list
       (buffer-string)
       (amd--imported-modules)
       (amd--module target)
       (nreverse prompts)))))
"##;
    let expect = expect![[
        r#"OK (#("define(['src/lib/widget'], function(widgetAlias) {\n});\n" 0 6 (font-lock-face js2-function-call) 8 24 (font-lock-face font-lock-string-face) 27 35 (font-lock-face font-lock-keyword-face) 36 47 (font-lock-face js2-function-param)) ("widgetAlias") "src/lib/widget" (("Import as (widget): ")))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn relative_imports_cover_same_directory_child_and_parent_files() {
    let elisp_form = r##"
(let* ((root (amd-test-project "relative-imports"))
       (current
        (amd-test-write root "src/views/main.js" ""))
       (targets
        (mapcar
         (lambda (relative)
           (amd-test-write root relative ""))
         '("src/views/helper.js"
           "src/views/sub/item.js"
           "src/model.js")))
       (buffer (find-file-noselect current)))
  (with-current-buffer buffer
    (let ((default-directory root)
          (amd-always-use-relative-file-name t))
      (mapcar #'amd--module targets))))
"##;
    let expect = expect![[r#"OK ("./helper" "./sub/item" "../model")"#]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn duplicate_import_is_a_true_noop_for_text_params_and_point() {
    let elisp_form = r##"
(let ((root (amd-test-project "duplicate-import")))
  (with-temp-buffer
    (let ((default-directory root))
      (insert
       "define([\n    'lib/foo'\n], function(foo) {\n});\n")
      (amd-test-parse)
      (let ((before (buffer-string))
            (before-point (point)))
        (cl-letf (((symbol-function 'read-string)
                   (lambda (&rest _) "foo")))
          (amd--import "lib/foo"))
        (list
         (equal before (buffer-string))
         before-point
         (point)
         (amd--imported-modules))))))
"##;
    let expect = expect![[r#"OK (t 47 47 ("foo"))"#]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn import_name_conflict_decline_reprompts_and_accepts_second_alias() {
    let elisp_form = r##"
(let ((root (amd-test-project "name-conflict")))
  (with-temp-buffer
    (let ((default-directory root)
          (answers '("existing" "replacement"))
          prompts)
      (insert
       "define([], function() {\n  var existing = 1;\n});\n")
      (amd-test-parse)
      (goto-char (point-min))
      (search-forward "existing")
      (cl-letf
          (((symbol-function 'read-string)
            (lambda (&rest arguments)
              (push arguments prompts)
              (pop answers)))
           ((symbol-function 'y-or-n-p)
            (lambda (prompt)
              (push (list prompt) prompts)
              nil)))
        (amd--import "lib/item"))
      (list
       (buffer-string)
       (nreverse prompts)))))
"##;
    let expect = expect![[
        r#"OK (#("define(['lib/item'], function(replacement) {\n  var existing = 1;\n});\n" 0 6 (font-lock-face js2-function-call) 21 29 (font-lock-face font-lock-keyword-face) 47 50 (font-lock-face font-lock-keyword-face) 51 59 (font-lock-face font-lock-variable-name-face)) (("Import as (item): ") ("Name existing already defined.  Use anyway? ") ("Import as (item): ")))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn goto_imports_materializes_missing_dependency_array_before_function() {
    let elisp_form = r##"
(let ((root (amd-test-project "missing-array")))
  (with-temp-buffer
    (let ((default-directory root))
      (insert "define(function() {\n});\n")
      (amd-test-parse)
      (cl-letf (((symbol-function 'read-string)
                 (lambda (&rest _) "")))
        (amd--import "lib/new"))
      (list (buffer-string)
            (point)))))
"##;
    let expect = expect![[
        r#"OK (#("define([\n    'lib/new'\n], function(new) {\n});\n" 0 6 (font-lock-face js2-function-call) 26 34 (font-lock-face font-lock-keyword-face)) 23)"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn kill_module_removes_middle_dependency_parameter_and_repairs_commas() {
    let elisp_form = r##"
(let ((root (amd-test-project "kill-module")))
  (with-temp-buffer
    (let ((default-directory root)
          (kill-ring nil))
      (insert
       "define([\n  'a',\n  'b',\n  'c'\n], function(a, b, c) {\n});\n")
      (amd-test-parse)
      (goto-char (point-min))
      (search-forward "'b'")
      (beginning-of-line)
      (amd-kill-module)
      (js2-parse)
      (list
       (buffer-string)
       (amd--imported-modules)
       kill-ring))))
"##;
    let expect = expect![[
        r#"OK (#("define([\n  'a',\n  'c'\n], function(a, c) {\n});\n" 0 6 (font-lock-face js2-function-call) 11 14 (font-lock-face font-lock-string-face) 18 21 (font-lock-face font-lock-string-face) 25 33 (font-lock-face font-lock-keyword-face) 34 35 (font-lock-face js2-function-param) 37 38 (font-lock-face js2-function-param)) ("a" "c") ("\n" #("  'b'," 2 5 (font-lock-face font-lock-string-face))))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn move_module_swaps_dependency_and_parameter_while_js2_refactor_moves_line() {
    let elisp_form = r##"
(let ((root (amd-test-project "move-module")))
  (with-temp-buffer
    (let ((default-directory root))
      (insert
       "define([\n  'a',\n  'b',\n  'c'\n], function(a, b, c) {\n});\n")
      (amd-test-parse)
      (goto-char (point-min))
      (search-forward "'b'")
      (beginning-of-line)
      (amd-move-line-up)
      (js2-parse)
      (list
       (buffer-string)
       (amd--imported-modules)))))
"##;
    let expect = expect![[
        r#"OK (#("define([\n    'b',\n  'a',\n  'c'\n], function(b, a, c) {\n});\n" 0 6 (font-lock-face js2-function-call) 13 16 (font-lock-face font-lock-string-face) 20 23 (font-lock-face font-lock-string-face) 27 30 (font-lock-face font-lock-string-face) 34 42 (font-lock-face font-lock-keyword-face) 43 44 (font-lock-face js2-function-param) 46 47 (font-lock-face js2-function-param) 49 50 (font-lock-face js2-function-param)) ("b" "a" "c"))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}

#[test]
fn kill_line_falls_back_to_real_js2_refactor_outside_import_array() {
    let elisp_form = r##"
(let ((root (amd-test-project "kill-line")))
  (with-temp-buffer
    (let ((default-directory root)
          (kill-ring nil))
      (insert
       "define([], function() {\n  var value = 1;\n  return value;\n});\n")
      (amd-test-parse)
      (goto-char (point-min))
      (search-forward "var value")
      (beginning-of-line)
      (amd-kill-line)
      (list
       (buffer-string)
       kill-ring))))
"##;
    let expect = expect![[
        r#"OK (#("define([], function() {\n\n  return value;\n});\n" 0 6 (font-lock-face js2-function-call) 11 19 (font-lock-face font-lock-keyword-face) 27 33 (font-lock-face font-lock-keyword-face)) (#("  var value = 1;" 2 5 (font-lock-face font-lock-keyword-face) 6 11 (font-lock-face font-lock-variable-name-face))))"#
    ]];
    assert_amd_mode_parity(elisp_form, expect);
}
