use expect_test::expect;

use super::{assert_ariadne_parity, assert_ariadne_with_legacy_cl_parity};

#[test]
fn goto_visits_real_file_widens_and_uses_one_based_line_and_column() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name
                  "ariadne-navigation/Main.hs"
                  temporary-file-directory))
                buffer)
         (make-directory (file-name-directory file) t)
         (with-temp-file file
           (insert "module Main where\n"
                   "alpha = 1\n"
                   "target = alpha + 2\n"
                   "omega = target\n"))
         (setq buffer (find-file-noselect file))
         (unwind-protect
             (with-current-buffer buffer
               (narrow-to-region
                (line-beginning-position 2)
                (point-max))
               (ariadne-goto file 3 10)
               (list
                (file-name-nondirectory
                 (buffer-file-name))
                (line-number-at-pos)
                (current-column)
                (char-after)
                (point-min)
                (point-max)
                (buffer-size)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[r#"OK ("Main.hs" 3 9 97 1 63 62)"#]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn goto_first_position_and_past_end_lines_follow_emacs_motion_contract() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name
                  "ariadne-navigation/Bounds.hs"
                  temporary-file-directory))
                buffer first past)
         (make-directory (file-name-directory file) t)
         (with-temp-file file
           (insert "one\ntwo\nthree\n"))
         (setq buffer (find-file-noselect file))
         (unwind-protect
             (with-current-buffer buffer
               (ariadne-goto file 1 1)
               (setq first
                     (list (point)
                           (line-number-at-pos)
                           (current-column)))
               (setq past
                     (condition-case error
                         (progn
                           (ariadne-goto file 99 4)
                           (list :ok
                                 (point)
                                 (line-number-at-pos)
                                 (current-column)
                                 (eobp)))
                       (error
                        (list :error error
                              (point)
                              (line-number-at-pos)
                              (current-column)
                              (eobp)))))
               (list first past))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect!["OK ((1 1 0) (:error (end-of-buffer) 15 4 0 t))"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn current_line_reports_absolute_position_even_inside_narrowing() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\ntwo\nthree\nfour\nfive\n")
         (goto-char (point-min))
         (forward-line 2)
         (let ((start (point)))
           (forward-line 2)
           (narrow-to-region start (point))
           (goto-char (point-min))
           (forward-line 1)
           (list (line-number-at-pos)
                 (ariadne-current-line)
                 (point-min)
                 (point-max))))"##;
    let expect = expect!["OK (2 4 9 20)"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn goto_definition_without_file_does_not_connect_send_or_push_mark() {
    let elisp_form = r##"(let ((ariadne-process nil)
               calls)
         (cl-letf (((symbol-function 'ariadne-connect)
                    (lambda () (push :connect calls)))
                   ((symbol-function 'ariadne-send)
                    (lambda (&rest args)
                      (push (cons :send args) calls)))
                   ((symbol-function 'push-mark)
                    (lambda (&rest args)
                      (push (cons :mark args) calls))))
           (with-temp-buffer
             (insert "name")
             (goto-char 3)
             (list (ariadne-goto-definition)
                   (nreverse calls)
                   ariadne-process))))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn goto_definition_connects_then_sends_practical_file_line_column_request() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name
                  "ariadne-definition/src/Main.hs"
                  temporary-file-directory))
                (ariadne-process nil)
                calls buffer)
         (make-directory (file-name-directory file) t)
         (with-temp-file file
           (insert "module Main where\n"
                   "first = 1\n"
                   "answer = first + 41\n"))
         (setq buffer (find-file-noselect file))
         (unwind-protect
             (cl-letf
                 (((symbol-function 'ariadne-connect)
                   (lambda ()
                     (push :connect calls)
                     (setq ariadne-process 'socket)))
                  ((symbol-function 'ariadne-send)
                   (lambda (object process)
                     (push (list :send object process)
                           calls))))
               (with-current-buffer buffer
                 (goto-char (point-min))
                 (forward-line 2)
                 (forward-char 9)
                 (let ((before (point)))
                   (ariadne-goto-definition)
                   (list
                    before
                    (point)
                    (mark t)
                    (nreverse calls)
                    ariadne-process))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (38 38 38 (:connect (:send [call ariadne find ("[ORACLE-TMPDIR]/ariadne-definition/src/Main.hs" 3 9)] socket)) socket)"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn goto_definition_reuses_existing_connection_without_reconnecting() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name
                  "ariadne-definition/Reuse.hs"
                  temporary-file-directory))
                (ariadne-process 'existing-socket)
                calls buffer)
         (make-directory (file-name-directory file) t)
         (with-temp-file file
           (insert "module Reuse where\nvalue = 42\n"))
         (setq buffer (find-file-noselect file))
         (unwind-protect
             (cl-letf
                 (((symbol-function 'ariadne-connect)
                   (lambda () (push :unexpected-connect calls)))
                  ((symbol-function 'ariadne-send)
                   (lambda (object process)
                     (push (list object process) calls))))
               (with-current-buffer buffer
                 (goto-char (point-max))
                 (ariadne-goto-definition)
                 (nreverse calls)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (([call ariadne find ("[ORACLE-TMPDIR]/ariadne-definition/Reuse.hs" 3 0)] existing-socket))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn unsuccessful_connection_attempt_does_not_push_mark_or_send() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name
                  "ariadne-definition/Offline.hs"
                  temporary-file-directory))
                (ariadne-process nil)
                calls buffer)
         (make-directory (file-name-directory file) t)
         (with-temp-file file
           (insert "offline = target\n"))
         (setq buffer (find-file-noselect file))
         (unwind-protect
             (cl-letf
                 (((symbol-function 'ariadne-connect)
                   (lambda ()
                     (push :connect calls)
                     nil))
                  ((symbol-function 'ariadne-send)
                   (lambda (&rest args)
                     (push (cons :send args) calls)))
                  ((symbol-function 'push-mark)
                   (lambda (&rest args)
                     (push (cons :mark args) calls))))
               (with-current-buffer buffer
                 (goto-char 5)
                 (list (ariadne-goto-definition)
                       (nreverse calls)
                       ariadne-process)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect!["OK (nil (:connect) nil)"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn send_frames_real_bert_request_and_writes_one_exact_socket_payload() {
    let elisp_form = r##"(let* ((object
                 (vector 'call 'ariadne 'find
                         '("/workspace/src/Main.hs"
                           22 8)))
                calls)
         (cl-letf (((symbol-function 'process-send-string)
                    (lambda (process bytes)
                      (push (list process bytes) calls))))
           (ariadne-send object 'socket)
           (let* ((entry (car calls))
                  (bytes (cadr entry))
                  (header (substring bytes 0 4))
                  (body (substring bytes 4)))
             (with-temp-buffer
               (set-buffer-multibyte nil)
               (insert header)
               (goto-char (point-min))
               (list
                (car entry)
                (length calls)
                (length header)
                (ariadne-decode-length)
                (length body)
                (bert-unpack body)
                (equal (bert-unpack body)
                       object))))))"##;
    let expect =
        expect!["OK (socket 1 4 64 64 [call ariadne find (\"/workspace/src/Main.hs\" 22 8)] t)"];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}
