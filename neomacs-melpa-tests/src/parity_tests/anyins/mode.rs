use expect_test::expect;

use super::{assert_anyins_parity, assert_anyins_signal_parity};

#[test]
fn direct_mode_helpers_toggle_read_only_state_and_clear_every_recorded_artifact() {
    let elisp_form = r##"(with-temp-buffer
  (insert "alpha beta\n")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (goto-char 3)
    (anyins-record-current-position)
    (let ((overlay anyins-buffers-overlays))
      (list
       (anyins-turn-on-mode)
       buffer-read-only
       (copy-tree anyins-buffers-positions)
       (length anyins-buffers-overlays)
       (anyins-turn-off-mode)
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays
       (mapcar #'overlay-buffer overlay)))))"##;
    let expect = expect!["OK (t t ((1 2)) 1 nil nil nil nil (nil))"];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn minor_mode_lifecycle_installs_lighter_keymap_messages_and_cleans_on_disable() {
    let elisp_form = r##"(with-temp-buffer
  (insert "alpha beta\ngamma\n")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil)
        messages)
    (cl-letf (((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push (apply #'format format-string arguments) messages))))
      (let ((enable-result (anyins-mode 1)))
        (goto-char 4)
        (anyins-record-current-position)
        (let ((enabled-state
               (list
                enable-result
                anyins-mode
                buffer-read-only
                (assq 'anyins-mode minor-mode-alist)
                (assq 'anyins-mode minor-mode-map-alist)
                (copy-tree anyins-buffers-positions)
                (length anyins-buffers-overlays))))
          (let ((disable-result (anyins-mode 0)))
            (list
             enabled-state
             (list
              disable-result
              anyins-mode
              buffer-read-only
              anyins-buffers-positions
              anyins-buffers-overlays)
             (nreverse messages))))))))"##;
    let expect = expect![[
        r#"OK ((t t t (anyins-mode " Anyins") (anyins-mode keymap (33 . anyins-insert-command) (121 . anyins-yank) (13 . anyins-record-current-position) (113 . anyins-disable-mode)) ((1 3)) 1) (nil nil nil nil nil) ("Anyins mode enabled" "Anyins mode disabled"))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn disable_command_delegates_to_minor_mode_and_restores_a_writable_clean_buffer() {
    let elisp_form = r##"(with-temp-buffer
  (insert "record this point")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil)
        messages)
    (cl-letf (((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push (apply #'format format-string arguments) messages))))
      (anyins-mode 1)
      (goto-char 8)
      (anyins-record-current-position)
      (list
       (anyins-disable-mode)
       anyins-mode
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays
       (nreverse messages)))))"##;
    let expect =
        expect![[r#"OK (nil nil nil nil nil ("Anyins mode enabled" "Anyins mode disabled"))"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn yank_with_an_empty_kill_ring_still_finishes_the_session_without_editing_the_buffer() {
    let elisp_form = r##"(with-temp-buffer
  (insert "unchanged\ntext\n")
  (goto-char (point-min))
  (let ((kill-ring nil)
        (anyins-buffers-positions nil)
        (anyins-buffers-overlays nil)
        messages)
    (cl-letf (((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push (apply #'format format-string arguments) messages))))
      (anyins-mode 1)
      (list
       (anyins-yank)
       (buffer-string)
       anyins-mode
       buffer-read-only
       (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (nil "unchanged\ntext\n" nil nil ("Anyins mode enabled" "Anyins mode disabled"))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn insert_command_propagates_the_exact_shell_failure() {
    let elisp_form = r##"(with-temp-buffer
  (insert "original")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil)))
      (anyins-mode 1)
      (cl-letf (((symbol-function 'shell-command-to-string)
                 (lambda (command)
                   (error "shell failed for %s" command))))
        (anyins-insert-command "broken-command")))))"##;
    let expect = expect![[r#"ERR (error "shell failed for broken-command")"#]];
    assert_anyins_signal_parity(elisp_form, expect);
}

#[test]
fn shell_failure_leaves_the_mode_enabled_but_the_buffer_writable_and_unmodified() {
    let elisp_form = r##"(with-temp-buffer
  (insert "original")
  (goto-char 5)
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil)))
      (anyins-mode 1)
      (anyins-record-current-position)
      (let ((failure
             (condition-case error-data
                 (cl-letf (((symbol-function 'shell-command-to-string)
                            (lambda (command)
                              (error "shell failed for %s" command))))
                   (anyins-insert-command "broken-command"))
               (error error-data))))
        (list
         failure
         (buffer-string)
         anyins-mode
         buffer-read-only
         (copy-tree anyins-buffers-positions)
         (mapcar
          (lambda (overlay)
            (list
             (overlay-start overlay)
             (overlay-end overlay)
             (overlay-get overlay 'face)))
          anyins-buffers-overlays))))))"##;
    let expect = expect![[
        r#"OK ((error "shell failed for broken-command") "original" t nil ((1 4)) ((5 6 anyins-recorded-positions)))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}
