use expect_test::expect;

use super::assert_android_env_parity;

#[test]
fn refactor_map_reads_real_csv_rows_and_preserves_upstream_reverse_precedence() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
       (mapping (expand-file-name "androidx-map.csv" root)))
  (with-temp-file mapping
    (insert
     "android.support.v4.app.Fragment,androidx.fragment.app.Fragment\n"
     "android.support.v7.widget.RecyclerView,androidx.recyclerview.widget.RecyclerView\n"
     "android.arch.lifecycle.ViewModel,androidx.lifecycle.ViewModel\n"))
  (android-env-refactor-map mapping))"##;
    let expect = expect![[
        r#"OK (("android.arch.lifecycle.ViewModel" "androidx.lifecycle.ViewModel") ("android.support.v7.widget.RecyclerView" "androidx.recyclerview.widget.RecyclerView") ("android.support.v4.app.Fragment" "androidx.fragment.app.Fragment"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn refactor_file_ensure_reuses_configuration_but_prompts_when_missing_or_prefix_forced() {
    let elisp_form = r##"(let ((android-env-refactor-file "/mappings/existing.csv")
      (current-prefix-arg nil)
      (answers '("/mappings/first.csv" "/mappings/second.csv"))
      events)
  (cl-letf (((symbol-function 'read-file-name)
             (lambda (prompt)
               (let ((answer (pop answers)))
                 (push (list prompt answer) events)
                 answer))))
    (let ((reused (android-env-refactor-file-ensure)))
      (setq android-env-refactor-file nil)
      (let ((missing (android-env-refactor-file-ensure)))
        (setq current-prefix-arg '(4))
        (let ((forced (android-env-refactor-file-ensure)))
          (list reused
                missing
                forced
                android-env-refactor-file
                (nreverse events)))))))"##;
    let expect = expect![[
        r#"OK (nil "/mappings/first.csv" "/mappings/second.csv" "/mappings/second.csv" (("Mappings file: " "/mappings/first.csv") ("Mappings file: " "/mappings/second.csv")))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn refactor_rewrites_multiple_androidx_imports_counts_matches_and_restores_numeric_point() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
       (mapping (expand-file-name "androidx-map.csv" root))
       (android-env-refactor-file mapping)
       (current-prefix-arg nil)
       messages)
  (with-temp-file mapping
    (insert
     "android\\.support\\.v4,androidx.core\n"
     "LegacyWidget,ModernWidget\n"))
  (with-temp-buffer
    (insert
     "import android.support.v4.app.Fragment;\n"
     "LegacyWidget first = new LegacyWidget();\n"
     "android.support.v4.util.Pair pair;\n")
    (goto-char (point-min))
    (search-forward "first")
    (let ((original-point (point)))
      (cl-letf (((symbol-function 'message)
                 (lambda (format-string &rest arguments)
                   (let ((text (apply #'format format-string arguments)))
                     (push text messages)
                     text))))
        (let ((replacements (android-env-refactor)))
          (list
           replacements
           original-point
           (point)
           (buffer-string)
           (nreverse messages)))))))"##;
    let expect = expect![[
        r#"OK (4 59 59 "import androidx.core.app.Fragment;\nModernWidget first = new ModernWidget();\nandroidx.core.util.Pair pair;\n" ("Refactored 4 matches"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn refactor_treats_mapping_sources_as_regexps_and_destinations_as_backreference_templates() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
       (mapping (expand-file-name "class-map.csv" root))
       (android-env-refactor-file mapping)
       (current-prefix-arg nil))
  (with-temp-file mapping
    (insert "Legacy\\([A-Z][A-Za-z]+\\),Modern\\1\n"))
  (with-temp-buffer
    (insert
     "LegacyWidget widget; LegacyRepository repo; Legacy1 untouched;")
    (let ((replacements (android-env-refactor)))
      (list replacements (buffer-string)))))"##;
    let expect =
        expect![[r#"OK (2 "ModernWidget widget; ModernRepository repo; Legacy1 untouched;")"#]];
    assert_android_env_parity(elisp_form, expect);
}

#[test]
fn recursive_refactor_walks_real_android_sources_saves_only_changed_files_and_kills_visit_buffers()
{
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
       (project (file-name-as-directory
                 (expand-file-name "android-project" root)))
       (source (expand-file-name "app/src/main" project))
       (nested (expand-file-name "feature" source))
       (mapping (expand-file-name "androidx-map.csv" project))
       (java-file (expand-file-name "Checkout.java" source))
       (kotlin-file (expand-file-name "Cart.kt" nested))
       (stable-file (expand-file-name "Stable.java" nested))
       (ignored-file (expand-file-name "notes.txt" source))
       (android-env-refactor-file mapping)
       (current-prefix-arg nil)
       (default-directory project)
       messages)
  (make-directory nested t)
  (with-temp-file mapping
    (insert "LegacyWidget,ModernWidget\n"))
  (with-temp-file java-file
    (insert "class Checkout { LegacyWidget widget; }\n"))
  (with-temp-file kotlin-file
    (insert "val first = LegacyWidget()\nval second = LegacyWidget()\n"))
  (with-temp-file stable-file
    (insert "class Stable { ModernWidget widget; }\n"))
  (with-temp-file ignored-file
    (insert "LegacyWidget is documentation text\n"))
  (cl-letf (((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (let ((text (apply #'format format-string arguments)))
                 (when
                     (or
                      (string-match-p "\\`[0-9]+ files matched\\'" text)
                      (string-prefix-p "Working on:" text)
                      (string-match-p
                       "\\`Refactored [0-9]+ matches\\'"
                       text))
                   (push text messages))
                 text))))
    (android-env-recursive-refactor "\\.\\(java\\|kt\\)\\'")
    (cl-labels
        ((contents
          (file)
          (with-temp-buffer
            (insert-file-contents file)
            (buffer-string))))
      (list
       (contents java-file)
       (contents kotlin-file)
       (contents stable-file)
       (contents ignored-file)
       (mapcar
        (lambda (file) (and (get-file-buffer file) t))
        (list java-file kotlin-file stable-file))
       (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK ("class Checkout { ModernWidget widget; }\n" "val first = ModernWidget()\nval second = ModernWidget()\n" "class Stable { ModernWidget widget; }\n" "LegacyWidget is documentation text\n" (nil nil nil) ("3 files matched" "Working on: [ORACLE-SANDBOX]/android-project/app/src/main/feature/Cart.kt..." "Refactored 2 matches" "Working on: [ORACLE-SANDBOX]/android-project/app/src/main/feature/Stable.java..." "Refactored 0 matches" "Working on: [ORACLE-SANDBOX]/android-project/app/src/main/Checkout.java..." "Refactored 1 matches"))"#
    ]];
    assert_android_env_parity(elisp_form, expect);
}
