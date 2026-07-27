use expect_test::expect;

use super::assert_ant_parity;

#[test]
fn ant_find_root_finds_nearest_real_build_file_from_deep_directory() {
    let elisp_form = r##"(let ((base (make-temp-file "ant-root-" t)))
         (unwind-protect
             (let* ((project (expand-file-name "project/" base))
                    (nested (expand-file-name "src/main/java/" project))
                    (outer-build (expand-file-name "build.xml" base))
                    (project-build
                     (expand-file-name "build.xml" project)))
               (make-directory nested t)
               (with-temp-file outer-build (insert "<project/>"))
               (with-temp-file project-build
                 (insert "<project name=\"nearest\"/>"))
               (let ((default-directory nested))
                 (let ((found (ant-find-root "build.xml")))
                   (list (file-relative-name found base)
                         (file-equal-p
                          (expand-file-name "build.xml" found)
                          project-build)
                         (file-name-absolute-p found)
                         (string-suffix-p "/" found)))))
           (delete-directory base t)))"##;
    let expect = expect![[r#"OK ("project/" t t t)"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_find_root_supports_custom_build_indicator_in_real_tree() {
    let elisp_form = r##"(let ((base (make-temp-file "ant-custom-" t)))
         (unwind-protect
             (let* ((project (expand-file-name "service/" base))
                    (nested (expand-file-name "module/tests/" project))
                    (indicator (expand-file-name "project-ant.xml" project)))
               (make-directory nested t)
               (with-temp-file indicator (insert "<project/>"))
               (let ((default-directory nested))
                 (let ((found (ant-find-root "project-ant.xml")))
                   (list (file-relative-name found base)
                         (file-exists-p
                          (expand-file-name
                           "project-ant.xml" found))))))
           (delete-directory base t)))"##;
    let expect = expect![[r#"OK ("service/" t)"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_find_root_returns_nil_for_real_tree_without_indicator() {
    let elisp_form = r##"(let ((base (make-temp-file "ant-missing-" t)))
         (unwind-protect
             (let ((nested (expand-file-name "a/b/c/" base)))
               (make-directory nested t)
               (let ((default-directory nested))
                 (ant-find-root "definitely-missing-build.xml")))
           (delete-directory base t)))"##;
    let expect = expect!["OK nil"];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_find_root_ascends_using_literal_parent_segments_before_normalizing() {
    let elisp_form = r##"(let ((default-directory "/work/project/src/main/")
               probes expansions)
         (cl-letf (((symbol-function 'file-exists-p)
                    (lambda (path)
                      (push path probes)
                      (string=
                       path
                       "/work/project/src/main/../../build.xml")))
                   ((symbol-function 'expand-file-name)
                    (lambda (path &optional directory)
                      (push (list path directory) expansions)
                      (cond
                       ((string=
                         path "/work/project/src/main/../")
                        "/work/project/src/")
                       ((string=
                         path "/work/project/src/main/../../")
                        "/work/project/")
                       (t path)))))
           (list (ant-find-root "build.xml")
                 (nreverse probes)
                 (nreverse expansions))))"##;
    let expect = expect![[
        r#"OK ("/work/project/" ("/work/project/src/main/build.xml" "/work/project/src/main/../build.xml" "/work/project/src/main/../../build.xml" "/work/project/src/main/../../build.xml") (("/work/project/src/main/" nil) ("/work/project/src/main/../" nil) ("/work/project/src/main/../../" nil)))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_find_root_at_filesystem_root_checks_once_then_returns_nil() {
    let elisp_form = r##"(let ((default-directory "/")
               probes expansions)
         (cl-letf (((symbol-function 'file-exists-p)
                    (lambda (path)
                      (push path probes)
                      nil))
                   ((symbol-function 'expand-file-name)
                    (lambda (path &optional directory)
                      (push (list path directory) expansions)
                      path)))
           (list (ant-find-root "build.xml")
                 (nreverse probes)
                 (nreverse expansions))))"##;
    let expect = expect![[r#"OK (nil ("/build.xml" "/build.xml") (("/" nil)))"#]];
    assert_ant_parity(elisp_form, expect);
}
