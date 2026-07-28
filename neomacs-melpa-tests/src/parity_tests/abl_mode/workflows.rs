use expect_test::expect;

use super::assert_abl_mode_parity;

/// A developer opens a test module of a git-controlled Python project and turns
/// abl-mode on.  The mode has to find the project base from `pyproject.toml',
/// read the current git branch, derive the per-branch shell buffer name and the
/// virtualenv name, keep all of that buffer local, and install its key map; a
/// file that belongs to no Python project must be refused with the documented
/// message and leave the mode off.
#[test]
fn enabling_abl_mode_derives_project_shell_and_virtualenv_names_from_git() {
    let elisp_form = r##"(let* ((base (abl-test-project))
       (file (expand-file-name "tests/ünïcode_tests.py" base))
       (observed nil))
  (find-file file)
  (abl-mode 1)
  (push (list :enabled abl-mode
              :lighter (assq 'abl-mode minor-mode-alist)
              :base (abl-test-relative abl-package-base)
              :branch abl-mode-branch
              :project abl-mode-project-name
              :shell abl-mode-shell-name
              :virtualenv abl-ve-name
              :buffer-local (list (local-variable-p 'abl-package-base)
                                  (local-variable-p 'abl-mode-branch)
                                  (local-variable-p 'abl-ve-name))
              :keys (list (key-binding (kbd "C-c t"))
                          (key-binding (kbd "C-c u"))
                          (key-binding (kbd "C-c f"))))
        observed)
  (abl-mode -1)
  (push (list :disabled abl-mode :keys (key-binding (kbd "C-c t"))) observed)
  (find-file (abl-test-loose-file))
  (let ((mark (abl-test-message-mark)))
    (abl-mode 1)
    (push (list :outside abl-mode
                :base abl-package-base
                :messages (abl-test-messages-since mark))
          observed))
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:enabled t :lighter (abl-mode " abl-mode") :base "ünïcode-projekt/" :branch "feature/ünïcode-tests" :project "ünïcode-projekt" :shell "ABL-SHELL:ünïcode-projekt_feature/ünïcode-tests" :virtualenv "ünïcode-projekt_feature-ünïcode-tests" :buffer-local (t t t) :keys (abl-mode-run-test-at-point abl-mode-rerun-last-test abl-mode-format-file)) (:disabled nil :keys nil) (:outside nil :base "" :messages ("Could not find project base. Please make sure there is a setup.py or requirements.txt in a higher directory.")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

/// The central abl-mode workflow: point sits inside a test method of a test
/// class, the developer types `C-c t', and abl-mode opens the project shell,
/// changes into the project base and runs the entity `FILE::CLASS::METHOD'
/// through `abl-mode-test-command'.  The recording shell pins the exact command
/// line, the stand-in `python' pins the argument vector and the working
/// directory, and the code buffer must keep its point.
#[test]
fn running_the_test_at_point_sends_one_unittest_command_to_the_project_shell() {
    let elisp_form = r##"(let* ((base (abl-test-project))
       (file (expand-file-name "tests/ünïcode_tests.py" base)))
  (abl-test-setup "python")
  (find-file file)
  (abl-mode 1)
  (setq abl-mode-check-and-activate-ve nil)
  (goto-char (point-min))
  (search-forward "self.assertEqual")
  (let ((shell abl-mode-shell-name)
        (code (current-buffer))
        (mark (abl-test-message-mark)))
    (execute-kbd-macro (kbd "C-c t"))
    (list :ready (abl-test-wait-for-shell shell 1)
          :sent (abl-test-shell-inputs)
          :argv (abl-test-commands)
          :directories (abl-test-directories)
          :shell-text (abl-test-shell-text shell)
          :shell-mode (with-current-buffer shell major-mode)
          :messages (abl-test-messages-since mark)
          :code-point (with-current-buffer code
                        (list (line-number-at-pos) (current-column)))
          :current (buffer-name)
          :windows (length (window-list)))))"##;

    let expect = expect![[
        r#"OK (:ready 1 :sent ("cd [ORACLE-SANDBOX]/ünïcode-projekt/ && python -m unittest tests/ünïcode_tests.py::ÜnicodeTests::test_encodes_a_name") :argv ("python|-m|unittest|tests/ünïcode_tests.py::ÜnicodeTests::test_encodes_a_name") :directories ("ünïcode-projekt") :shell-text "cd [ORACLE-SANDBOX]/ünïcode-projekt/ && python -m unittest tests/ünïcode_tests.py::ÜnicodeTests::test_encodes_a_name\nabl-ready\n" :shell-mode shell-mode :messages ("Running test(s) tests/ünïcode_tests.py::ÜnicodeTests::test_encodes_a_name on ABL-SHELL:ünïcode-projekt_feature/ünïcode-tests" "[ORACLE-SANDBOX]/ünïcode-projekt ") :code-point (7 24) :current "ABL-SHELL:ünïcode-projekt_feature/ünïcode-tests" :windows 1)"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

/// `C-c u' repeats the test that was run last, whatever point now is.  Before
/// anything has been run it only reports that, and starts no shell at all;
/// after running a whole test class from the class line it repeats exactly that
/// class even though point now sits on line 1, where the entity at point would
/// be the whole file.
#[test]
fn rerunning_the_last_test_repeats_the_class_entity_regardless_of_point() {
    let elisp_form = r##"(let* ((base (abl-test-project))
       (file (expand-file-name "tests/ünïcode_tests.py" base))
       (observed nil))
  (abl-test-setup "python")
  (find-file file)
  (abl-mode 1)
  (setq abl-mode-check-and-activate-ve nil)
  (let ((shell abl-mode-shell-name)
        (code (current-buffer))
        (mark (abl-test-message-mark)))
    (execute-kbd-macro (kbd "C-c u"))
    (push (list :nothing-run-yet (abl-test-messages-since mark)
                :argv (abl-test-commands)
                :shell-buffer (and (get-buffer shell) t))
          observed)
    (goto-char (point-min))
    (search-forward "class ÜnicodeTests")
    (execute-kbd-macro (kbd "C-c t"))
    (abl-test-wait-for-shell shell 1)
    (switch-to-buffer code)
    (goto-char (point-min))
    (setq mark (abl-test-message-mark))
    (push (list :entity-at-point (abl-mode-get-test-entity)) observed)
    (execute-kbd-macro (kbd "C-c u"))
    (push (list :ready (abl-test-wait-for-shell shell 2)
                :sent (abl-test-shell-inputs)
                :argv (abl-test-commands)
                :messages (abl-test-messages-since mark))
          observed))
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:nothing-run-yet ("You haven’t run any tests yet.") :argv nothing-recorded :shell-buffer nil) (:entity-at-point "tests/ünïcode_tests.py") (:ready 2 :sent ("cd [ORACLE-SANDBOX]/ünïcode-projekt/ && python -m unittest tests/ünïcode_tests.py::ÜnicodeTests" "cd [ORACLE-SANDBOX]/ünïcode-projekt/ && python -m unittest tests/ünïcode_tests.py::ÜnicodeTests") :argv ("python|-m|unittest|tests/ünïcode_tests.py::ÜnicodeTests" "python|-m|unittest|tests/ünïcode_tests.py::ÜnicodeTests") :messages ("Running test(s) tests/ünïcode_tests.py::ÜnicodeTests on ABL-SHELL:ünïcode-projekt_feature/ünïcode-tests" "[ORACLE-SANDBOX]/ünïcode-projekt ")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

/// A project checks in a `.abl' file so everyone on the team runs pytest with
/// dotted module names instead of `python -m unittest' with file paths.
/// Enabling abl-mode has to read those options out of the project base, apply
/// them (buffer locally where the package declares that) and use them for the
/// next `C-c t'.  The test module lives in a directory whose name contains a
/// space, which the package's unquoted command composition passes on to the
/// shell as two separate arguments.
#[test]
fn a_project_abl_file_switches_the_runner_to_pytest_with_module_names() {
    let elisp_form = r##"(let* ((base (abl-test-project
              (concat "abl-mode-test-command \"pytest -q %s\"\n"
                      "abl-mode-check-and-activate-ve nil\n"
                      "abl-use-test-file-path nil\n")))
       (file (expand-file-name "tests/api layer/service_tests.py" base)))
  (abl-test-setup "pytest")
  (find-file file)
  (abl-mode 1)
  (goto-char (point-min))
  (search-forward "assert True")
  (let ((shell abl-mode-shell-name)
        (code (current-buffer))
        (mark (abl-test-message-mark)))
    (execute-kbd-macro (kbd "C-c t"))
    (list :ready (abl-test-wait-for-shell shell 1)
          :options (with-current-buffer code
                     (list abl-mode-test-command
                           abl-mode-check-and-activate-ve
                           abl-use-test-file-path
                           (local-variable-p 'abl-mode-test-command)
                           (local-variable-p 'abl-use-test-file-path)))
          :sent (abl-test-shell-inputs)
          :argv (abl-test-commands)
          :directories (abl-test-directories)
          :messages (abl-test-messages-since mark))))"##;

    let expect = expect![[
        r#"OK (:ready 1 :options ("pytest -q %s" nil nil t nil) :sent ("cd [ORACLE-SANDBOX]/ünïcode-projekt/ && pytest -q tests.api layer.service_tests::test_service_root") :argv ("pytest|-q|tests.api|layer.service_tests::test_service_root") :directories ("ünïcode-projekt") :messages ("Running test(s) tests.api layer.service_tests::test_service_root on ABL-SHELL:ünïcode-projekt_feature/ünïcode-tests" "[ORACLE-SANDBOX]/ünïcode-projekt "))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

/// The developer keeps one virtualenv per project and branch below
/// `~/.virtualenvs'.  Because the derived virtualenv already exists, abl-mode
/// must not ask anything, and must chain the activation command in front of the
/// test command in the right order.  The virtualenv name is the project name
/// joined to the branch name with the slash replaced, while the shell buffer
/// name keeps the unmangled branch.
#[test]
fn an_existing_virtualenv_is_activated_before_the_test_command_runs() {
    let elisp_form = r##"(let* ((base (abl-test-project))
       (file (expand-file-name "tests/ünïcode_tests.py" base))
       (virtualenv (abl-test-virtualenv "ünïcode-projekt_feature-ünïcode-tests")))
  (abl-test-setup "python" "workon")
  (find-file file)
  (abl-mode 1)
  (goto-char (point-min))
  (search-forward "def test_rejects_empty_input")
  (let ((shell abl-mode-shell-name)
        (code (current-buffer)))
    (execute-kbd-macro (kbd "C-c t"))
    (list :ready (abl-test-wait-for-shell shell 1)
          :virtualenv (abl-test-relative virtualenv)
          :name (with-current-buffer code abl-ve-name)
          :activate (with-current-buffer code abl-mode-ve-activate-command)
          :sent (abl-test-shell-inputs)
          :argv (abl-test-commands)
          :directories (abl-test-directories))))"##;

    let expect = expect![[
        r#"OK (:ready 1 :virtualenv "home/.virtualenvs/ünïcode-projekt_feature-ünïcode-tests" :name "ünïcode-projekt_feature-ünïcode-tests" :activate "workon %s" :sent ("cd [ORACLE-SANDBOX]/ünïcode-projekt/ && workon ünïcode-projekt_feature-ünïcode-tests && python -m unittest tests/ünïcode_tests.py::ÜnicodeTests::test_rejects_empty_input") :argv ("workon|ünïcode-projekt_feature-ünïcode-tests" "python|-m|unittest|tests/ünïcode_tests.py::ÜnicodeTests::test_rejects_empty_input") :directories ("ünïcode-projekt" "ünïcode-projekt"))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

/// `C-c f' formats the visited file and `C-u C-c f' formats the whole project.
/// Both go through `abl-mode-format-command', whose default uses the same
/// numbered format field twice, so `black' and `isort' each have to receive the
/// same target; the reused shell buffer must accumulate both command lines in
/// order and the source file must not be touched by Emacs itself.
#[test]
fn formatting_the_current_file_and_then_the_whole_project_reuses_one_shell() {
    let elisp_form = r##"(let* ((base (abl-test-project))
       (file (expand-file-name "tests/api layer/service_tests.py" base)))
  (abl-test-setup "black" "isort")
  (find-file file)
  (abl-mode 1)
  (setq abl-mode-check-and-activate-ve nil)
  (let ((shell abl-mode-shell-name)
        (code (current-buffer)))
    (execute-kbd-macro (kbd "C-c f"))
    (abl-test-wait-for-shell shell 1)
    (switch-to-buffer code)
    (execute-kbd-macro (kbd "C-u C-c f"))
    (list :ready (abl-test-wait-for-shell shell 2)
          :sent (abl-test-shell-inputs)
          :argv (abl-test-commands)
          :directories (abl-test-directories)
          :modified (with-current-buffer code (buffer-modified-p)))))"##;

    let expect = expect![[
        r#"OK (:ready 2 :sent ("cd [ORACLE-SANDBOX]/ünïcode-projekt/ && black [ORACLE-SANDBOX]/ünïcode-projekt/tests/api layer/service_tests.py && isort --profile black [ORACLE-SANDBOX]/ünïcode-projekt/tests/api layer/service_tests.py" "cd [ORACLE-SANDBOX]/ünïcode-projekt/ && black . && isort --profile black .") :argv ("black|[ORACLE-SANDBOX]/ünïcode-projekt/tests/api|layer/service_tests.py" "isort|--profile|black|[ORACLE-SANDBOX]/ünïcode-projekt/tests/api|layer/service_tests.py" "black|." "isort|--profile|black|.") :directories ("ünïcode-projekt" "ünïcode-projekt" "ünïcode-projekt" "ünïcode-projekt") :modified nil)"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

/// `C-c t' inside a module that has no test class and no test function above
/// point cannot name an entity.  abl-mode has to signal the documented error
/// rather than running something arbitrary: no shell buffer may be created, no
/// command may reach the tools, and point must stay where the developer left
/// it.
#[test]
fn running_a_test_outside_any_test_entity_signals_and_starts_no_shell() {
    let elisp_form = r##"(let* ((base (abl-test-project))
       (file (expand-file-name "conftest.py" base)))
  (abl-test-setup "python")
  (find-file file)
  (abl-mode 1)
  (setq abl-mode-check-and-activate-ve nil)
  (goto-char (point-min))
  (search-forward "SETTINGS")
  (let ((shell abl-mode-shell-name)
        (mark (abl-test-message-mark)))
    (list :signal (condition-case failure
                      (execute-kbd-macro (kbd "C-c t"))
                    (error failure))
          :point (list (line-number-at-pos) (current-column))
          :current (buffer-name)
          :argv (abl-test-commands)
          :shell-buffer (and (get-buffer shell) t)
          :messages (abl-test-messages-since mark))))"##;

    let expect = expect![[
        r#"OK (:signal (error "You do not appear to be in a recognized test entity") :point (4 8) :current "conftest.py" :argv nothing-recorded :shell-buffer nil :messages nil)"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}
