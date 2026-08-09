;;; neo-term-interactive-test.el --- Interactive xdotool test for neo-term -*- lexical-binding: t -*-

;; Usage: ./target/release/neomacs -Q -l test/neomacs/neo-term-interactive-test.el
;;
;; This test is driven by xdotool from run-neo-term-interactive-test.sh.
;; xdotool invokes M-x neo-term and then sends keystrokes to the terminal.
;; All test state is rooted below the repository-local directory supplied by
;; run-neo-term-interactive-test.sh.

;;; Code:

(require 'cl-lib)

(defvar neo-term-itest--results-file
  (or (getenv "NEO_TERM_ITEST_RESULTS")
      (error "NEO_TERM_ITEST_RESULTS is not set"))
  "File to write test results to.")

(defvar neo-term-itest--marker-dir
  (or (getenv "NEO_TERM_ITEST_MARKER_DIR")
      (error "NEO_TERM_ITEST_MARKER_DIR is not set"))
  "Directory containing phase marker files.")

(defvar neo-term-itest--terminal-id nil
  "Active terminal ID for testing.")

(defun neo-term-itest--log (fmt &rest args)
  "Log a message to results file."
  (let ((msg (apply #'format fmt args)))
    (append-to-file (concat msg "\n") nil neo-term-itest--results-file)
    (message "neo-term-itest: %s" msg)))

(defun neo-term-itest--check (name condition)
  "Record a test result."
  (if condition
      (neo-term-itest--log "PASS: %s" name)
    (neo-term-itest--log "FAIL: %s" name)))

(defun neo-term-itest--wait-for-text (terminal-id predicate &optional timeout)
  "Wait until TERMINAL-ID text satisfies PREDICATE, then return the text.
TIMEOUT defaults to five seconds."
  (let ((deadline (+ (float-time) (or timeout 5)))
        text)
    (while (and (< (float-time) deadline)
                (not (and (setq text
                                (ignore-errors
                                  (neomacs-terminal-get-text terminal-id)))
                          (stringp text)
                          (funcall predicate text))))
      (sit-for 0.05))
    (and (stringp text) (funcall predicate text) text)))

(defun neo-term-itest--wait-until-destroyed (terminal-id &optional timeout)
  "Wait until TERMINAL-ID disappears from renderer-owned terminal state."
  (let ((deadline (+ (float-time) (or timeout 5)))
        destroyed)
    (while (and (< (float-time) deadline) (not destroyed))
      (setq destroyed
            (condition-case nil
                (null (neomacs-terminal-get-text terminal-id))
              (error t)))
      (unless destroyed (sit-for 0.05)))
    destroyed))

(defun neo-term-itest--marker (name)
  "Return the absolute marker path for NAME."
  (expand-file-name name neo-term-itest--marker-dir))

(defun neo-term-itest--run ()
  "Set up observation for an interactively created terminal."
  ;; Clear results file
  (when (file-exists-p neo-term-itest--results-file)
    (delete-file neo-term-itest--results-file))

  (neo-term-itest--log "=== Neo-Term Interactive Test ===")
  (neo-term-itest--log "Date: %s" (format-time-string "%Y-%m-%d %H:%M:%S"))

  ;; Check FFI
  (neo-term-itest--log "Checking FFI availability...")
  (neo-term-itest--check "FFI available" (fboundp 'neomacs-terminal-create))

  (unless (fboundp 'neomacs-terminal-create)
    (neo-term-itest--log "ABORT: FFI not available")
    (neo-term-itest--log "=== DONE ===")
    (kill-emacs 1))

  ;; Phase 1 is driven through the real M-x command by the shell script.
  (neo-term-itest--log "")
  (neo-term-itest--log "--- Phase 1: Waiting for M-x neo-term ---")

  ;; Set up a timer to check text extraction after xdotool sends input
  ;; The shell script will signal us by creating a marker file
  (run-with-timer 1 1 #'neo-term-itest--poll-phases))

(defvar neo-term-itest--phase 0
  "Current test phase.")

(defun neo-term-itest--poll-phases ()
  "Poll for phase progression signals from the shell script."
  (condition-case err
      (cond
       ;; Phase 1: xdotool invoked M-x neo-term.
       ((= neo-term-itest--phase 0)
        (let ((terminal-buffer
               (cl-find-if
                (lambda (buffer)
                  (with-current-buffer buffer
                    (eq major-mode 'neo-term-mode)))
                (buffer-list))))
          (when terminal-buffer
            (setq neo-term-itest--phase 1)
            (with-current-buffer terminal-buffer
              (setq neo-term-itest--terminal-id neo-term--id)
              (neo-term-itest--check
               "M-x neo-term created a terminal"
               (and neo-term-itest--terminal-id
                    (> neo-term-itest--terminal-id 0)))
              (neo-term-itest--check
               "Selected buffer is neo-term-mode"
               (and (eq (current-buffer) (window-buffer (selected-window)))
                    (eq major-mode 'neo-term-mode)))
              (neo-term-itest--log "Terminal ID: %s" neo-term-itest--terminal-id)
              (neo-term-itest--log "READY_FOR_INPUT")))))

       ;; Phase 2: After xdotool typed "echo hello" + Return
       ((and (= neo-term-itest--phase 1)
             (file-exists-p (neo-term-itest--marker "phase2")))
        (setq neo-term-itest--phase 2)
        (neo-term-itest--log "")
        (neo-term-itest--log "--- Phase 2: Text extraction after typing ---")
        (let ((text (neo-term-itest--wait-for-text
                     neo-term-itest--terminal-id
                     (lambda (value) (string-match-p "hello" value)))))
          (neo-term-itest--check "get-text returns string" (stringp text))
          (if (stringp text)
              (progn
                (neo-term-itest--log "Terminal text (first 200 chars): %s"
                                     (substring text 0 (min 200 (length text))))
                (neo-term-itest--check "Text contains 'hello'"
                                       (string-match-p "hello" text)))
            (neo-term-itest--check "Text contains 'hello'" nil))))

       ;; Phase 3: After xdotool typed a command with ANSI colors
       ((and (= neo-term-itest--phase 2)
             (file-exists-p (neo-term-itest--marker "phase3")))
        (setq neo-term-itest--phase 3)
        (neo-term-itest--log "")
        (neo-term-itest--log "--- Phase 3: ANSI color output ---")
        (let ((text (neo-term-itest--wait-for-text
                     neo-term-itest--terminal-id
                     (lambda (value)
                       (and (string-match-p "RED" value)
                            (string-match-p "GREEN" value)
                            (string-match-p "BLUE" value)
                            (string-match-p "COLOR_TEST" value))))))
          (neo-term-itest--check "ANSI terminal text returns string" (stringp text))
          (if (stringp text)
              (progn
                (neo-term-itest--log "Terminal text (first 300 chars): %s"
                                     (substring text 0 (min 300 (length text))))
                (neo-term-itest--check
                 "Text contains complete color test output"
                 (and (string-match-p "RED" text)
                      (string-match-p "GREEN" text)
                      (string-match-p "BLUE" text)
                      (string-match-p "COLOR_TEST" text))))
            (neo-term-itest--check "Text contains complete color test output" nil))))

       ;; Phase 4: Resize test
       ((and (= neo-term-itest--phase 3)
             (file-exists-p (neo-term-itest--marker "phase4")))
        (setq neo-term-itest--phase 4)
        (neo-term-itest--log "")
        (neo-term-itest--log "--- Phase 4: Terminal resize ---")
        (condition-case err
            (progn
              (neomacs-terminal-resize neo-term-itest--terminal-id 120 40)
              (neo-term-itest--check "Resize API call succeeded" t)
              ;; Renderer commands are FIFO, so stty observes the PTY only
              ;; after the preceding resize has actually been applied.
              (neomacs-terminal-write
               neo-term-itest--terminal-id
               "printf '__NEO_SIZE__'; stty size\r")
              (let ((text (neo-term-itest--wait-for-text
                           neo-term-itest--terminal-id
                           (lambda (value)
                             (string-match-p
                              "__NEO_SIZE__[[:space:]]*40 120"
                              value)))))
                (neo-term-itest--check "Text extraction works after resize"
                                        (and (stringp text) (> (length text) 0)))
                (neo-term-itest--check
                 "PTY reports 40 rows by 120 columns"
                 (and (stringp text)
                      (string-match-p "__NEO_SIZE__[[:space:]]*40 120" text)))
                (when (stringp text)
                  (neo-term-itest--log "After resize text length: %d" (length text)))))
          (error
           (neo-term-itest--log "ERROR during resize: %s" (error-message-string err))
           (neo-term-itest--check "Resize API call succeeded" nil))))

       ;; Phase 5: Floating terminal
       ((and (= neo-term-itest--phase 4)
             (file-exists-p (neo-term-itest--marker "phase5")))
        (setq neo-term-itest--phase 5)
        (neo-term-itest--log "")
        (neo-term-itest--log "--- Phase 5: Floating terminal ---")
        (condition-case err
            (let ((float-id (neo-term-floating 150 150 60 20)))
              (neo-term-itest--check "Floating terminal created" (and float-id (> float-id 0)))
              (when float-id
                (neo-term-itest--log "Floating terminal ID: %s" float-id)
                ;; Write to floating terminal
                (neomacs-terminal-write float-id "echo FLOATING_OK\r")
                (let ((text (neo-term-itest--wait-for-text
                             float-id
                             (lambda (value) (string-match-p "FLOATING_OK" value)))))
                  (neo-term-itest--check "Floating get-text returns string" (stringp text))
                  (if (stringp text)
                      (progn
                        (neo-term-itest--log "Floating text: %s"
                                             (substring text 0 (min 200 (length text))))
                        (neo-term-itest--check "Floating terminal has output"
                                               (string-match-p "FLOATING_OK" text)))
                    (neo-term-itest--check "Floating terminal has output" nil)))
                ;; Destroy floating terminal
                (neomacs-terminal-destroy float-id)
                (neo-term-itest--check
                 "Floating terminal removed from renderer state"
                 (neo-term-itest--wait-until-destroyed float-id))))
          (error
           (neo-term-itest--log "ERROR with floating terminal: %s" (error-message-string err))
           (neo-term-itest--check "Floating terminal created" nil))))

       ;; Phase 6: Cleanup and done
       ((and (= neo-term-itest--phase 5)
             (file-exists-p (neo-term-itest--marker "phase6")))
        (setq neo-term-itest--phase 6)
        (neo-term-itest--log "")
        (neo-term-itest--log "--- Phase 6: Cleanup ---")
        ;; Destroy main terminal
        (when neo-term-itest--terminal-id
          (condition-case err
              (progn
                (neomacs-terminal-destroy neo-term-itest--terminal-id)
                (neo-term-itest--check
                 "Main terminal removed from renderer state"
                 (neo-term-itest--wait-until-destroyed
                  neo-term-itest--terminal-id)))
            (error
             (neo-term-itest--log "ERROR destroying: %s" (error-message-string err)))))
        (neo-term-itest--log "")
        (neo-term-itest--log "=== DONE ===")))
    (error
     (neo-term-itest--log "POLL ERROR: %s" (error-message-string err)))))

;; Start the test
(neo-term-itest--run)

;;; neo-term-interactive-test.el ends here
