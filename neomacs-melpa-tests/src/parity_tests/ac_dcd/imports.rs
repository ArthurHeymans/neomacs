use expect_test::expect;

use super::assert_ac_dcd_parity;

#[test]
fn ac_dcd_parent_directory_handles_relative_absolute_root_and_nil_inputs() {
    let elisp_form = r##"(let ((root
                    (getenv
                     "NEOMACS_TEST_SANDBOX_ROOT")))
               (list
                (ac-dcd-parent-directory nil)
                (file-relative-name
                 (ac-dcd-parent-directory
                  (expand-file-name
                   "one/two/"
                   root))
                 root)
                (file-relative-name
                 (ac-dcd-parent-directory
                  (expand-file-name
                   "one/two"
                   root))
                 root)
                (ac-dcd-parent-directory "/")
                (let
                    ((default-directory root))
                  (file-relative-name
                   (ac-dcd-parent-directory
                    "relative/child/")
                   root))))"##;
    let expect = expect![[r#"OK (nil "one/" "one/" "/" "relative/")"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_search_file_up_finds_nearest_ancestor_and_preserves_optional_path_quirk() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (project
                     (expand-file-name
                      "search-up/project/"
                      root))
                    (nested
                     (expand-file-name
                      "src/deep/"
                      project))
                    (outer
                     (expand-file-name
                      "dub.json"
                      project))
                    (inner
                     (expand-file-name
                      "src/package.json"
                      project))
                    (local
                     (expand-file-name
                      "local.json"
                      nested)))
               (make-directory nested t)
               (with-temp-file outer
                 (insert "outer"))
               (with-temp-file inner
                 (insert "inner"))
               (with-temp-file local
                 (insert "local"))
               (list
                (file-relative-name
                 (ac-dcd-search-file-up
                  "package.json"
                  nested)
                 root)
                (file-relative-name
                 (ac-dcd-search-file-up
                  "dub.json"
                  nested)
                 root)
                (ac-dcd-search-file-up
                 "missing.json"
                 nested)
                (let
                    ((default-directory
                       nested))
                  (list
                   (ac-dcd-search-file-up
                    "local.json")
                   (condition-case
                       error-data
                       (ac-dcd-search-file-up
                        "missing.json")
                     (error
                      (car error-data)))))))"##;
    let expect = expect![[
        r#"OK ("search-up/project/src/package.json" "search-up/project/dub.json" nil ("local.json" excessive-lisp-nesting))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_find_imports_std_reads_home_configuration_and_filters_non_import_flags() {
    let elisp_form = r##"(let* ((home
                     (getenv "HOME"))
                    (config
                     (expand-file-name
                      "dmd.conf"
                      home)))
               (with-temp-file config
                 (insert
                  "[Environment]\n"
                  "DFLAGS = -I/usr/include/dlang -w -I../relative -version=Test\n"))
               (cl-letf
                   (((symbol-function
                      'executable-find)
                     (lambda (_)
                       nil)))
                 (ac-dcd-find-imports-std)))"##;
    let expect = expect![[r#"OK ("-I/usr/include/dlang" "-I../relative")"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_find_imports_std_discovers_dlang_install_and_returns_nil_without_configs() {
    let elisp_form = r##"(let* ((home
                     (getenv "HOME"))
                    (dlang
                     (expand-file-name
                      "dlang/dmd-2.111.0/linux/bin64/"
                      home))
                    (config
                     (expand-file-name
                      "dmd.conf"
                      dlang)))
               (make-directory dlang t)
               (with-temp-file config
                 (insert
                  "DFLAGS=-I../../src/phobos -I../../src/druntime/import -O\n"))
               (cl-letf
                   (((symbol-function
                      'executable-find)
                     (lambda (_)
                       nil)))
                 (let ((found
                        (ac-dcd-find-imports-std)))
                   (delete-file config)
                   (list
                    found
                    (ac-dcd-find-imports-std)))))"##;
    let expect = expect![[r#"OK (("-I../../src/phobos" "-I../../src/druntime/import") nil)"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_find_imports_dub_and_all_project_imports_preserve_source_order() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'fldd--get-project-dir)
                     (lambda ()
                       (push 'project events)
                       "/project/"))
                    ((symbol-function
                      'fldd--get-dub-package-dirs)
                     (lambda ()
                       (push 'packages events)
                       '("/dep/one" "/dep/two")))
                    ((symbol-function
                      'ac-dcd-find-imports-std)
                     (lambda ()
                       (push 'std events)
                       '("-I/std/one" "-I/std/two"))))
                 (let ((dub
                        (ac-dcd-find-imports-dub))
                       (all
                        (ac-dcd--find-all-project-imports)))
                   (cl-letf
                       (((symbol-function
                          'fldd--get-project-dir)
                         (lambda ()
                           nil)))
                     (list
                      dub
                      all
                      (ac-dcd-find-imports-dub)
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (("-I/dep/one" "-I/dep/two") ("-I/std/one" "-I/std/two" "-I/dep/one" "-I/dep/two") nil (project packages std project packages))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_add_imports_distinguishes_discovered_flags_from_explicit_paths() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-dcd--find-all-project-imports)
                     (lambda ()
                       '("-I/discovered"
                         "-I/already")))
                    ((symbol-function
                      'ac-dcd-call-process)
                     (lambda (args)
                       (push args calls)
                       'sent)))
                 (list
                  (ac-dcd--add-imports)
                  (ac-dcd--add-imports
                   '("/explicit/one"
                     "relative/two"))
                  (ac-dcd-add-import
                   "/interactive/")
                  (interactive-form
                   #'ac-dcd-add-import)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (sent sent sent (interactive "DPath to add to DCD imports: ") (("-I/discovered" "-I/already") ("-I/explicit/one" "-Irelative/two") ("-I/interactive/")))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}
