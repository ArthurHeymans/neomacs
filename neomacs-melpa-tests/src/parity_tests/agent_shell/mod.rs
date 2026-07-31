use std::time::Duration;

use crate::{AGENT_SHELL_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod completion;
mod content;
mod list_edit;
mod markdown;
mod workflows;

const AGENT_SHELL_TEST_TIMEOUT: Duration = Duration::from_secs(15);
const AGENT_SHELL_PRELUDE: &str = r##"
(dolist (entry (directory-files package-user-dir t "\\`[^.]"))
  (when (file-directory-p entry)
    (add-to-list 'load-path entry)))

(require 'acp-fakes)

(defvar neomacs-agent-shell-test-messages nil)
(defvar neomacs-agent-shell-test-sent-requests nil)
(defvar neomacs-agent-shell-test-client-senders nil)
(defvar neomacs-agent-shell-test-last-client nil)

(defun neomacs-agent-shell-test-request-sender (&rest arguments)
  (let* ((client (plist-get arguments :client))
         (request (plist-get arguments :request))
         (sender (cdr (assq client
                            neomacs-agent-shell-test-client-senders))))
    (push request neomacs-agent-shell-test-sent-requests)
    (apply sender arguments)))

(defun neomacs-agent-shell-test-make-client (buffer)
  (let* ((client (acp-fakes-make-client
                  neomacs-agent-shell-test-messages))
         (sender (map-elt client :request-sender)))
    (setf (map-elt client :context-buffer) buffer)
    (push (cons client sender)
          neomacs-agent-shell-test-client-senders)
    (setf (map-elt client :request-sender)
          #'neomacs-agent-shell-test-request-sender)
    (setq neomacs-agent-shell-test-last-client client)
    client))

(defun neomacs-agent-shell-test-session-messages
    (notifications &optional prompt-result)
  (append
   '(((:direction . outgoing)
      (:kind . request)
      (:object (jsonrpc . "2.0")
               (method . "initialize")
               (id . 1)))
     ((:direction . incoming)
      (:kind . response)
      (:object
       (jsonrpc . "2.0")
       (id . 1)
       (result
        (protocolVersion . 1)
        (agentCapabilities
         (loadSession . :false)
         (promptCapabilities
          (image . t)
          (audio . :false)
          (embeddedContext . t))))))
     ((:direction . outgoing)
      (:kind . request)
      (:object (jsonrpc . "2.0")
               (method . "session/new")
               (id . 2)))
     ((:direction . incoming)
      (:kind . response)
      (:object
       (jsonrpc . "2.0")
       (id . 2)
       (result
        (sessionId . "parity-session")
        (configOptions
         . [((id . "model")
             (name . "Model")
             (category . "model")
             (type . "select")
             (currentValue . "sonnet")
             (options
              . [((value . "sonnet") (name . "Sonnet"))
                 ((value . "opus") (name . "Opus"))]))]))))
     ((:direction . outgoing)
      (:kind . request)
      (:object (jsonrpc . "2.0")
               (method . "session/prompt")
               (id . 3))))
   notifications
   (list
    (list
     (cons :direction 'incoming)
     (cons :kind 'response)
     (cons :object
           (list
            '(jsonrpc . "2.0")
            '(id . 3)
            (cons 'result
                  (or prompt-result
                      '((stopReason . "end_turn"))))))))))

(defun neomacs-agent-shell-test-start (messages)
  (setq neomacs-agent-shell-test-messages messages
        neomacs-agent-shell-test-sent-requests nil
        neomacs-agent-shell-test-client-senders nil
        neomacs-agent-shell-test-last-client nil)
  (agent-shell--start
   :config
   (agent-shell-make-agent-config
    :identifier 'parity-agent
    :mode-line-name "Parity"
    :buffer-name "Parity"
    :shell-prompt "Parity> "
    :shell-prompt-regexp "Parity> "
    :client-maker #'neomacs-agent-shell-test-make-client
    :install-instructions "The parity fixture uses the installed acp-fakes client")
   :no-focus t
   :new-session t
   :session-strategy 'new))

(defun neomacs-agent-shell-test-kill (shell)
  (when (buffer-live-p shell)
    (with-current-buffer shell
      ;; Batch Emacs reports process sentinels after their owning buffer has
      ;; disappeared.  The workflow asserts both pending-request sets are
      ;; empty before this external-boundary cleanup.
      (when-let* ((process (get-buffer-process shell)))
        (set-process-sentinel process #'ignore))
      (when-let* ((client (map-elt agent-shell--state :client))
                  (process (map-elt client :process)))
        (set-process-sentinel process #'ignore)))
    (kill-buffer shell)))

(defun neomacs-agent-shell-test-normalize-transcript (text)
  (replace-regexp-in-string
   "20[0-9][0-9]-[0-9][0-9]-[0-9][0-9] [0-9][0-9]:[0-9][0-9]:[0-9][0-9]"
   "TIME"
   text
   t
   t))

(defun neomacs-agent-shell-test-visible-buffer-string ()
  (let ((position (point-min))
        (result ""))
    (while (< position (point-max))
      (unless (invisible-p position)
        (setq result
              (concat result (char-to-string (char-after position)))))
      (setq position (1+ position)))
    (string-trim-right result)))
"##;

fn agent_shell_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGENT_SHELL_MELPA_PIN, source_file)
        .expect("prepare pinned agent-shell source below ./tmp")
        .with_prelude(AGENT_SHELL_PRELUDE)
        .with_timeout(AGENT_SHELL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed agent-shell parity test")
        .into()
}

fn assert_agent_shell_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agent_shell_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agent-shell parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_agent_shell_parity(elisp_form: &str, expected: Expect) {
    assert_agent_shell_source_parity("agent-shell.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_agent_shell_parity` cases (2a).
pub(crate) fn assert_agent_shell_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        agent_shell_oracle("agent-shell.el"),
        &name,
        "agent_shell_parity",
        cases,
    );
}
