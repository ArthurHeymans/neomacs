use super::assert_assess_discover_parity;
use expect_test::{Expect, expect};

#[test]
fn discovery_library_registers_its_complete_function_and_command_surface() {
    let elisp_form = r##"
(list
 (featurep 'assess-discover)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (commandp symbol)
     (help-function-arglist
      symbol t)
     (file-name-nondirectory
      (or
       (symbol-file
        symbol 'defun)
       ""))))
  '(assess-discover-tests
    assess-discover--load-all-tests
    assess-discover-load-tests
    assess-discover-run-batch
    assess-discover-run-and-exit-batch)))
"##;
    let expect: Expect = expect![[
        r#"OK (t ((assess-discover-tests nil (directory) "assess-discover.el") (assess-discover--load-all-tests nil (directory) "assess-discover.el") (assess-discover-load-tests t nil "assess-discover.el") (assess-discover-run-batch nil (&optional selector) "assess-discover.el") (assess-discover-run-and-exit-batch nil (&optional selector) "assess-discover.el")))"#
    ]];
    assert_assess_discover_parity(elisp_form, expect);
}

#[test]
fn discovery_uses_first_matching_naming_scheme_and_returns_sorted_basename_matches() {
    let elisp_form = r##"
(let* ((root
        (file-name-as-directory
         (assess-test-path
          "discover-priority")))
       (write
        (lambda (relative)
          (let ((path
                 (expand-file-name
                  relative root)))
            (make-directory
             (file-name-directory path)
             t)
            (with-temp-file path
              (insert
               "(setq fixture-loaded t)\n"))))))
  (funcall write "alpha-test.el")
  (funcall write "beta-tests.el")
  (funcall write "test-gamma.el")
  (funcall write "test/delta.el")
  (funcall write "tests/epsilon.el")
  (list
   (assess-discover-tests root)
   (progn
     (delete-file
      (expand-file-name
       "alpha-test.el"
       root))
     (assess-discover-tests root))
   (progn
     (delete-file
      (expand-file-name
       "beta-tests.el"
       root))
     (assess-discover-tests root))
   (progn
     (delete-file
      (expand-file-name
       "test-gamma.el"
       root))
     (mapcar
      (lambda (path)
        (file-relative-name path root))
      (assess-discover-tests root)))))
"##;
    let expect: Expect = expect![[
        r#"OK (("alpha-test.el") ("beta-tests.el") ("test-gamma.el") ("test/delta.el"))"#
    ]];
    assert_assess_discover_parity(elisp_form, expect);
}

#[test]
fn discovery_falls_back_to_tests_directory_only_when_test_directory_is_absent() {
    let elisp_form = r##"
(let* ((root
        (file-name-as-directory
         (assess-test-path
          "discover-tests-directory")))
       (tests
        (expand-file-name
         "tests"
         root))
       (empty
        (file-name-as-directory
         (assess-test-path
          "discover-empty"))))
  (make-directory tests t)
  (make-directory empty t)
  (with-temp-file
      (expand-file-name
       "zeta.el"
       tests)
    (insert "(provide 'zeta)\n"))
  (with-temp-file
      (expand-file-name
       "alpha.el"
       tests)
    (insert "(provide 'alpha)\n"))
  (list
   (mapcar
    (lambda (path)
      (file-relative-name path root))
    (assess-discover-tests root))
   (assess-discover-tests
    empty)))
"##;
    let expect: Expect = expect![[r#"OK (("tests/alpha.el" "tests/zeta.el") nil)"#]];
    assert_assess_discover_parity(elisp_form, expect);
}

#[test]
fn load_all_tests_loads_every_discovered_file_in_order() {
    let elisp_form = r##"
(let (loads)
  (cl-letf
      (((symbol-function
         'assess-discover-tests)
        (lambda (directory)
          (list
           (concat directory
                   "alpha-test.el")
           (concat directory
                   "beta-test.el"))))
       ((symbol-function 'load)
        (lambda (file &rest args)
          (push
           (list file args)
           loads)
          (intern
           (file-name-base file)))))
    (list
     (assess-discover--load-all-tests
      "/fixture/")
     (nreverse loads))))
"##;
    let expect: Expect = expect![[
        r#"OK (("/fixture/alpha-test.el" "/fixture/beta-test.el") (("/fixture/alpha-test.el" nil) ("/fixture/beta-test.el" nil)))"#
    ]];
    assert_assess_discover_parity(elisp_form, expect);
}

#[test]
fn interactive_load_tests_uses_default_directory_and_propagates_loader_result() {
    let elisp_form = r##"
(let ((default-directory
       "/fixture/project/")
      calls)
  (cl-letf
      (((symbol-function
         'assess-discover--load-all-tests)
        (lambda (directory)
          (push directory calls)
          :loaded)))
    (list
     (assess-discover-load-tests)
     calls
     (commandp
      'assess-discover-load-tests))))
"##;
    let expect: Expect = expect![[r#"OK (:loaded ("/fixture/project/") t)"#]];
    assert_assess_discover_parity(elisp_form, expect);
}

#[test]
fn batch_runner_loads_default_directory_then_passes_selector_to_ert() {
    let elisp_form = r##"
(let ((default-directory
       "/fixture/batch/")
      calls)
  (cl-letf
      (((symbol-function
         'assess-discover--load-all-tests)
        (lambda (directory)
          (push
           (list :load directory)
           calls)
          :loaded))
       ((symbol-function
         'ert-run-tests-batch)
        (lambda (selector)
          (push
           (list :run selector)
           calls)
          (list
           :batch-result selector))))
    (list
     (assess-discover-run-batch
      '(tag :fast))
     (nreverse calls))))
"##;
    let expect: Expect =
        expect![[r#"OK ((:batch-result #1=(tag :fast)) ((:load "/fixture/batch/") (:run #1#)))"#]];
    assert_assess_discover_parity(elisp_form, expect);
}

#[test]
fn exiting_batch_runner_loads_default_directory_then_delegates_selector() {
    let elisp_form = r##"
(let ((default-directory
       "/fixture/exit/")
      calls)
  (cl-letf
      (((symbol-function
         'assess-discover--load-all-tests)
        (lambda (directory)
          (push
           (list :load directory)
           calls)
          :loaded))
       ((symbol-function
         'ert-run-tests-batch-and-exit)
        (lambda (selector)
          (push
           (list :exit selector)
           calls)
          (list
           :exit-result selector))))
    (list
     (assess-discover-run-and-exit-batch
      "fixture-selector")
     (nreverse calls))))
"##;
    let expect: Expect = expect![[
        r#"OK ((:exit-result "fixture-selector") ((:load "/fixture/exit/") (:exit "fixture-selector")))"#
    ]];
    assert_assess_discover_parity(elisp_form, expect);
}
