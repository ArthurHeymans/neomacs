use std::time::Duration;

use expect_test::expect;

use crate::{CachedMelpaOracle, OVERSEER_MELPA_PIN};

use super::batch_support::{ParityBatchCase, assert_oracle_batch_cases};

const PRELUDE: &str = r####"
(require 'cl-lib)
(require 'overseer)

(defun neomacs-overseer-test-root ()
  "Return the deterministic project directory inside the oracle sandbox."
  (file-name-as-directory
   (expand-file-name "overseer-project"
                     (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))

(defun neomacs-overseer-test-reset-project ()
  "Create an empty Cask project with source and test directories."
  (let ((root (neomacs-overseer-test-root)))
    (when (file-exists-p root)
      (delete-directory root t))
    (make-directory (expand-file-name "lisp/nested" root) t)
    (make-directory (expand-file-name "test" root) t)
    (write-region "(source gnu)\n" nil (expand-file-name "Cask" root)
                  nil 'silent)
    root))

(defun neomacs-overseer-test-write (file content)
  "Write CONTENT to FILE and return FILE."
  (make-directory (file-name-directory file) t)
  (write-region content nil file nil 'silent)
  file)

(defun neomacs-overseer-test-wait (buffer)
  "Wait for BUFFER's compilation process and return its final status."
  (let ((process (get-buffer-process buffer)))
    (while (and process (process-live-p process))
      (accept-process-output process 0.1))
    (when process
      (accept-process-output process 0.01)
      (process-status process))))

(defun neomacs-overseer-test-output-rows (buffer)
  "Return stable runner result rows from compilation BUFFER."
  (with-current-buffer buffer
    (cl-remove-if-not
     (lambda (line)
       (string-match-p "\\`\\(?:ARGS\\|RESULT\\|Finished in\\)" line))
     (split-string
      (buffer-substring-no-properties (point-min) (point-max)) "\n" t))))

(defun neomacs-overseer-test-kill-buffer (buffer)
  "Kill BUFFER without leaving a modified-file prompt."
  (when (buffer-live-p buffer)
    (with-current-buffer buffer (set-buffer-modified-p nil))
    (kill-buffer buffer)))
"####;

fn package_contract_and_command_builder_describe_the_installed_runner() -> ParityBatchCase {
    let elisp_form = r####"
(let ((descriptor (cadr (assq 'overseer package-alist))))
  (list
   :package
   (list :name (package-desc-name descriptor)
         :version (package-version-join (package-desc-version descriptor))
         :requirements (package-desc-reqs descriptor)
         :feature (featurep 'overseer)
         :reported-version (overseer-version))
   :defaults
   (list :command overseer-command
         :buffer overseer-buffer-name
         :root-indicators overseer--project-root-indicators)
   :command-lists
   (list
    (overseer--build-runner-cmdlist overseer-command)
    (overseer--build-runner-cmdlist
     '("cask exec ert-runner" ("--verbose" "" "-p" "billing retry"))))))
"####;
    let expected = expect![[
        r#"OK (:package (:name overseer :version "20240109.800" :requirements ((emacs (24)) (dash (2 10 0)) (pkg-info (0 4)) (f (0 18 1))) :feature t :reported-version "20240109.800") :defaults (:command "cask exec ert-runner" :buffer "*overseer*" :root-indicators ("Cask")) :command-lists (("cask" "exec" "ert-runner") ("cask exec ert-runner" "--verbose" "-p" "billing retry")))"#
    ]];
    ParityBatchCase::value(
        "package_contract_and_command_builder_describe_the_installed_runner",
        elisp_form,
        expected,
    )
}

fn nested_project_test_files_activate_the_complete_minor_mode_workflow() -> ParityBatchCase {
    let elisp_form = r####"
(let* ((root (neomacs-overseer-test-reset-project))
       (test-file
        (neomacs-overseer-test-write
         (expand-file-name "test/billing-test.el" root)
         "(ert-deftest billing-retries () (should t))\n"))
       (source-file
        (neomacs-overseer-test-write
         (expand-file-name "lisp/nested/billing.el" root)
         "(defun billing-run () :ok)\n"))
       test-buffer source-buffer)
  (unwind-protect
      (progn
        (setq test-buffer (find-file-noselect test-file)
              source-buffer (find-file-noselect source-file))
        (with-current-buffer test-buffer (emacs-lisp-mode))
        (with-current-buffer source-buffer (emacs-lisp-mode))
        (let ((default-directory (file-name-directory source-file)))
          (list
           :root
           (list :found (equal (overseer-project-root) root)
                 :from (file-relative-name default-directory root))
           :test-buffer
           (with-current-buffer test-buffer
             (list :mode major-mode
                   :overseer overseer-mode
                   :lighter (assq 'overseer-mode minor-mode-alist)
                   :keys
                   (mapcar (lambda (key) (cons key (key-binding (kbd key))))
                           '("C-c , a" "C-c , t" "C-c , b" "C-c , f"
                             "C-c , g" "C-c , p" "C-c , d" "C-c , q"
                             "C-c , v" "C-c , h"))))
           :source-buffer
           (with-current-buffer source-buffer
             (list :mode major-mode :overseer overseer-mode)))))
    (neomacs-overseer-test-kill-buffer test-buffer)
    (neomacs-overseer-test-kill-buffer source-buffer)))
"####;
    let expected = expect![[
        r#"OK (:root (:found t :from "lisp/nested/") :test-buffer (:mode emacs-lisp-mode :overseer t :lighter (overseer-mode " overseer") :keys (("C-c , a" . overseer-test) ("C-c , t" . overseer-test-run-test) ("C-c , b" . overseer-test-this-buffer) ("C-c , f" . overseer-test-file) ("C-c , g" . overseer-test-tags) ("C-c , p" . overseer-test-prompt) ("C-c , d" . overseer-test-debug) ("C-c , q" . overseer-test-quiet) ("C-c , v" . overseer-test-verbose) ("C-c , h" . overseer-help))) :source-buffer (:mode emacs-lisp-mode :overseer nil))"#
    ]];
    ParityBatchCase::value(
        "nested_project_test_files_activate_the_complete_minor_mode_workflow",
        elisp_form,
        expected,
    )
}

fn public_commands_build_real_ert_runner_invocations_from_the_project_root() -> ParityBatchCase {
    let elisp_form = r####"
(let* ((root (neomacs-overseer-test-reset-project))
       (default-directory (expand-file-name "lisp/nested" root))
       (overseer-command "cask exec ert-runner")
       (calls nil))
  (cl-letf (((symbol-function 'overseer-compilation-run)
             (lambda (cmdlist buffer-name)
               (push
                (list :command
                      (mapcar
                       (lambda (argument)
                         (if (file-name-absolute-p argument)
                             (file-relative-name argument root)
                           argument))
                       cmdlist)
                      :buffer buffer-name
                      :at-root (equal default-directory root))
                calls)
               :compilation-buffer)))
    (overseer-test)
    (overseer-help)
    (overseer-test-debug)
    (overseer-test-verbose)
    (overseer-test-quiet)
    (overseer-test-tags "fast,unit")
    (overseer-test-prompt "--pattern invoice.* --verbose")
    (overseer-test-file (expand-file-name "test/payments-test.el" root)))
  (list :calls (nreverse calls)
        :caller-directory-restored
        (equal default-directory (expand-file-name "lisp/nested" root))))
"####;
    let expected = expect![[
        r#"OK (:calls ((:command ("cask exec ert-runner") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "--help") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "--debug") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "--verbose") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "--quiet") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "-t" "fast,unit") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "--pattern invoice.* --verbose") :buffer "*overseer*" :at-root t) (:command ("cask exec ert-runner" "test/payments-test.el") :buffer "*overseer*" :at-root t)) :caller-directory-restored t)"#
    ]];
    ParityBatchCase::value(
        "public_commands_build_real_ert_runner_invocations_from_the_project_root",
        elisp_form,
        expected,
    )
}

fn ert_at_point_and_file_commands_select_practical_test_targets() -> ParityBatchCase {
    let elisp_form = r####"
(let* ((root (neomacs-overseer-test-reset-project))
       (test-file (expand-file-name "test/invoice-test.el" root))
       (source-file (expand-file-name "lisp/invoice.el" root))
       (calls nil)
       (messages nil)
       test-buffer source-buffer)
  (neomacs-overseer-test-write
   test-file
   "(ert-deftest invoice-retries-after-timeout ()\n  (should (= 2 (+ 1 1))))\n\n(defun invoice-helper ()\n  :helper)\n")
  (neomacs-overseer-test-write source-file "(defun invoice () :sent)\n")
  (unwind-protect
      (progn
        (setq test-buffer (find-file-noselect test-file)
              source-buffer (find-file-noselect source-file))
        (with-current-buffer test-buffer (emacs-lisp-mode))
        (with-current-buffer source-buffer (emacs-lisp-mode))
        (cl-letf (((symbol-function 'overseer-execute)
                   (lambda (arguments) (push arguments calls)))
                  ((symbol-function 'message)
                   (lambda (format-string &rest arguments)
                     (let ((text (apply #'format format-string arguments)))
                       (push text messages)
                       text))))
          (with-current-buffer test-buffer
            (goto-char (point-min))
            (search-forward "should")
            (overseer-test-run-test)
            (overseer-test-this-buffer)
            (search-forward ":helper")
            (overseer-test-run-test))
          (with-current-buffer source-buffer
            (overseer-test-this-buffer)))
        (list
         :calls
         (mapcar
          (lambda (arguments)
            (mapcar
             (lambda (argument)
               (if (and (stringp argument) (file-name-absolute-p argument))
                   (file-relative-name argument root)
                 argument))
             arguments))
          (nreverse calls))
         :messages (nreverse messages)))
    (neomacs-overseer-test-kill-buffer test-buffer)
    (neomacs-overseer-test-kill-buffer source-buffer)))
"####;
    let expected = expect![[
        r#"OK (:calls (("-p" "invoice-retries-after-timeout") ("test/invoice-test.el")) :messages ("No test at point" "invoice.el is no test file."))"#
    ]];
    ParityBatchCase::value(
        "ert_at_point_and_file_commands_select_practical_test_targets",
        elisp_form,
        expected,
    )
}

fn real_compilation_cleans_runner_output_and_preserves_unsaved_source() -> ParityBatchCase {
    let elisp_form = r####"
(let* ((root (neomacs-overseer-test-reset-project))
       (runner
        (neomacs-overseer-test-write
         (expand-file-name "bin/fake-ert-runner" root)
         "#!/bin/sh\nprintf 'ert-runner started at ignored\\n'\nprintf 'ARGS:%s\\n' \"$*\"\nprintf '\\033[31mRESULT:failed invoice-retries\\033[0m\\n'\nprintf 'Finished in 0.02 seconds\\n'\nprintf 'ert-runner finished at ignored\\n'\n"))
       (source-file
        (neomacs-overseer-test-write
         (expand-file-name "lisp/invoice.el" root)
         "(defvar invoice-retries 1)\n"))
       (default-directory (expand-file-name "lisp/nested" root))
       (overseer-command runner)
       (compilation-ask-about-save nil)
       source-buffer compilation-buffer command-result process-status)
  (set-file-modes runner #o755)
  (unwind-protect
      (progn
        (setq source-buffer (find-file-noselect source-file))
        (with-current-buffer source-buffer
          (goto-char (point-max))
          (insert "(setq invoice-retries 3)\n"))
        (setq command-result (overseer-test-verbose)
              compilation-buffer (get-buffer overseer-buffer-name)
              process-status
              (neomacs-overseer-test-wait compilation-buffer))
        (list
         :command-result command-result
         :process process-status
         :compilation
         (with-current-buffer compilation-buffer
           (list :name (buffer-name)
                 :mode major-mode
                 :read-only buffer-read-only
                 :at-root (equal default-directory root)
                 :saved-name overseer--buffer-name
                 :regexp-head (car compilation-error-regexp-alist)
                 :filter-hooks
                 (mapcar (lambda (hook) (and (memq hook compilation-filter-hook) t))
                         '(overseer--handle-ansi-color overseer--remove-header))))
         :runner-output
         (neomacs-overseer-test-output-rows compilation-buffer)
         :source
         (with-current-buffer source-buffer
           (list :modified (buffer-modified-p)
                 :buffer-lines (count-lines (point-min) (point-max))))
         :disk (with-temp-buffer
                 (insert-file-contents source-file)
                 (buffer-string))))
    (neomacs-overseer-test-kill-buffer source-buffer)
    (neomacs-overseer-test-kill-buffer compilation-buffer)))
"####;
    let expected = expect![[
        r#"OK (:command-result (overseer--remove-header overseer--handle-ansi-color t) :process exit :compilation (:name "*overseer*" :mode overseer-buffer-mode :read-only t :at-root t :saved-name "*overseer*" :regexp-head overseer :filter-hooks (t t)) :runner-output ("ARGS:--verbose" "RESULT:failed invoice-retries" "Finished in 0.02 seconds") :source (:modified t :buffer-lines 2) :disk "(defvar invoice-retries 1)\n")"#
    ]];
    ParityBatchCase::value(
        "real_compilation_cleans_runner_output_and_preserves_unsaved_source",
        elisp_form,
        expected,
    )
}

fn killing_a_live_runner_buffer_terminates_the_owned_process() -> ParityBatchCase {
    let elisp_form = r####"
(let* ((root (neomacs-overseer-test-reset-project))
       (default-directory root)
       (overseer-command "while :; do :; done")
       (compilation-ask-about-save nil)
       compilation-buffer process hook-before status-before)
  (unwind-protect
      (progn
        (overseer-test)
        (setq compilation-buffer (get-buffer overseer-buffer-name)
              process (get-buffer-process compilation-buffer))
        (with-current-buffer compilation-buffer
          (setq hook-before kill-buffer-hook)
          ;; This probe targets Overseer's kill hook, not the generic query UI.
          (setq-local kill-buffer-query-functions nil))
        (setq status-before (process-status process))
        (kill-buffer compilation-buffer)
        (let ((attempts 0))
          (while (and (process-live-p process) (< attempts 100))
            (setq attempts (1+ attempts))
            (accept-process-output process 0.01)))
        (list :hook hook-before
              :before status-before
              :buffer-live (buffer-live-p compilation-buffer)
              :process-live (process-live-p process)
              :after (process-status process)))
    (when (and process (process-live-p process)) (kill-process process))
    (neomacs-overseer-test-kill-buffer compilation-buffer)))
"####;
    let expected = expect![
        "OK (:hook overseer--kill-any-orphan-proc :before run :buffer-live nil :process-live nil :after signal)"
    ];
    ParityBatchCase::value(
        "killing_a_live_runner_buffer_terminates_the_owned_process",
        elisp_form,
        expected,
    )
}

fn project_commands_signal_when_no_root_indicator_exists() -> ParityBatchCase {
    let elisp_form = r####"
(let* ((root (expand-file-name "overseer-no-root"
                               (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
       (default-directory (file-name-as-directory root))
       (overseer--project-root-indicators '("Cask" ".overseer-root")))
  (when (file-exists-p root) (delete-directory root t))
  (make-directory root t)
  (overseer-project-root))
"####;
    let expected = expect![[r#"ERR (user-error "Overseer unable to identify project root")"#]];
    ParityBatchCase::signal(
        "project_commands_signal_when_no_root_indicator_exists",
        elisp_form,
        expected,
    )
}

#[test]
fn overseer_package_batch() {
    assert_oracle_batch_cases(
        CachedMelpaOracle::new(OVERSEER_MELPA_PIN, "overseer.el")
            .expect("prepare revision-pinned Overseer source below ./tmp")
            .with_timeout(Duration::from_secs(300))
            .with_prelude(PRELUDE),
        "overseer-package-batch",
        "Overseer",
        &[
            package_contract_and_command_builder_describe_the_installed_runner(),
            nested_project_test_files_activate_the_complete_minor_mode_workflow(),
            public_commands_build_real_ert_runner_invocations_from_the_project_root(),
            ert_at_point_and_file_commands_select_practical_test_targets(),
            real_compilation_cleans_runner_output_and_preserves_unsaved_source(),
            killing_a_live_runner_buffer_terminates_the_owned_process(),
            project_commands_signal_when_no_root_indicator_exists(),
        ],
    );
}
