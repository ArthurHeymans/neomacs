use expect_test::expect;

use super::ParityBatchCase;

fn defaults_and_command_format_are_deterministic() -> ParityBatchCase {
    ParityBatchCase::value(
        "defaults_and_command_format_are_deterministic",
        r####"
(list :runner pytest-global-name
      :flags pytest-cmd-flags
      :format pytest-cmd-format-string
      :roots pytest-project-root-files
      :all (commandp 'pytest-all)
      :one (commandp 'pytest-one)
      :module (commandp 'pytest-module)
      :again (commandp 'pytest-again)
      :formatted
      (pytest-cmd-format
       "cd '%s' && %s %s '%s'"
       "/tmp/proj"
       "pytest"
       "-x -s"
       "test_demo.py")
      :feature (featurep 'pytest))
"####,
        expect![[
            r#"OK (:runner "pytest" :flags "-x -s" :format "cd '%s' && %s %s '%s'" :roots ("setup.py" ".hg" ".git") :all t :one t :module t :again t :formatted "cd '/tmp/proj' && pytest -x -s 'test_demo.py'" :feature t)"#
        ]],
    )
}

fn project_root_walks_up_to_marker_files() -> ParityBatchCase {
    ParityBatchCase::value(
        "project_root_walks_up_to_marker_files",
        r####"
(neomacs-pytest-test-with-project
 (lambda (root)
   (let* ((nested (expand-file-name "pkg/nested" root))
          (file (expand-file-name "test_demo.py" root)))
     (make-directory nested t)
     (list :from-file
           (equal (file-truename
                   (pytest-find-project-root (file-name-directory file)))
                  (file-truename root))
           :from-nested
           (equal (file-truename (pytest-find-project-root nested))
                  (file-truename root))
           :is-root (and (pytest-project-root root) t)
           :nested-not-root (and (pytest-project-root nested) t)))))
"####,
        expect!["OK (:from-file nil :from-nested nil :is-root t :nested-not-root nil)"],
    )
}

fn py_testable_builds_class_and_function_paths() -> ParityBatchCase {
    ParityBatchCase::value(
        "py_testable_builds_class_and_function_paths",
        r####"
(neomacs-pytest-test-with-project
 (lambda (root)
   (let ((file (expand-file-name "test_demo.py" root)))
     (with-current-buffer (find-file-noselect file)
       (python-mode)
       (goto-char (point-min))
       (search-forward "assert 1 + 1")
       (let ((method (pytest-py-testable))
             (inner (pytest-inner-testable))
             (outer (pytest-outer-testable)))
         (goto-char (point-min))
         (search-forward "assert True")
         (list :method-suffix
               (and (string-match "::TestMath::test_add\\'" method) t)
               :inner inner
               :outer outer
               :toplevel-suffix
               (let ((top (pytest-py-testable)))
                 (and (string-match "::test_top_level\\'" top) t))
               :file-prefix
               (and (string-prefix-p file method) t)))))))
"####,
        expect![[
            r#"OK (:method-suffix t :inner "test_add" :outer ("class" . "TestMath") :toplevel-suffix t :file-prefix t)"#
        ]],
    )
}

fn get_command_composes_cd_and_quoted_test_names() -> ParityBatchCase {
    ParityBatchCase::value(
        "get_command_composes_cd_and_quoted_test_names",
        r####"
(neomacs-pytest-test-with-project
 (lambda (root)
   (let* ((file (expand-file-name "test_demo.py" root))
          (default-directory root)
          (pytest-global-name "pytest")
          (where (pytest-find-project-root (file-name-directory file)))
          (cmd (pytest-cmd-format
                pytest-cmd-format-string where "pytest" "-q"
                (format "'%s'" file)))
          (cmd-all (pytest-cmd-format
                    pytest-cmd-format-string where "pytest" "-x" "'.'")))
     (list :where-ok (equal (file-truename where) (file-truename root))
           :has-cd (and (string-match-p "cd '" cmd) t)
           :has-pytest (and (string-match-p "pytest" cmd) t)
           :has-flags (and (string-match-p "-q" cmd) t)
           :has-file (and (string-match-p "test_demo.py" cmd) t)
           :all-has-dot (and (string-match-p "'\\.'" cmd-all) t)
           :all-has-x (and (string-match-p "-x" cmd-all) t)))))
"####,
        expect![
            "OK (:where-ok nil :has-cd t :has-pytest t :has-flags t :has-file t :all-has-dot t :all-has-x t)"
        ],
    )
}

fn missing_test_file_signals_and_temp_buffer_name_is_stable() -> ParityBatchCase {
    ParityBatchCase::value(
        "missing_test_file_signals_and_temp_buffer_name_is_stable",
        r####"
(list :missing
      (condition-case err
          (progn (pytest-check-test-file "/no/such/pytest-file.py") :ok)
        (error (error-message-string err)))
      :buffer-name (pytest-get-temp-buffer-name)
      :again-without-history
      (condition-case err
          (progn (pytest-again) :ok)
        (error (error-message-string err))))
"####,
        expect![[
            r#"OK (:missing "’/no/such/pytest-file.py’ is not an extant file." :buffer-name "*pytest*" :again-without-history "Pytest has not run before")"#
        ]],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        defaults_and_command_format_are_deterministic(),
        project_root_walks_up_to_marker_files(),
        py_testable_builds_class_and_function_paths(),
        get_command_composes_cd_and_quoted_test_names(),
        missing_test_file_signals_and_temp_buffer_name_is_stable(),
    ]
}
