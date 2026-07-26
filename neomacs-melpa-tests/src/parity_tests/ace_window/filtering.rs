use super::assert_ace_window_parity;
use expect_test::expect;

#[test]
fn ace_window_make_frame_character_setter_accepts_nil_and_valid_values_and_rejects_conflicts() {
    let elisp_form = r##"(let ((aw-keys '(?a ?b))
             (aw-dispatch-alist
              '((?x fixture-command
                    "Fixture")))
             (aw-make-frame-char ?z))
         (mapcar
          (lambda (value)
            (list
             value
             (condition-case error
                 (list
                  'ok
                  (aw-set-make-frame-char
                   'aw-make-frame-char
                   value))
               (error
                (list 'error error)))
             aw-make-frame-char))
          '(nil 113 "bad" 97 120)))"##;
    let expect = expect![[
        r#"OK ((nil (ok nil) nil) (113 (ok 113) 113) ("bad" (error (user-error "‘aw-make-frame-char’ must be a character, not ‘bad’")) 113) (97 (error (user-error "‘aw-make-frame-char’ is ‘a’; this conflicts with the same character in ‘aw-keys’")) 113) (120 (error (user-error "‘aw-make-frame-char’ is ‘x’; this conflicts with the same character in ‘aw-dispatch-alist’")) 113))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_ignored_predicate_covers_names_modes_child_selected_and_parameters() {
    let elisp_form = r##"(let ((named-buffer
              (generate-new-buffer
               "*ace-window-ignored-name*"))
             (mode-buffer
              (generate-new-buffer
               " *ace-window-mode*"))
             (plain-buffer
              (generate-new-buffer
               " *ace-window-plain*")))
         (unwind-protect
             (progn
               (with-current-buffer
                   mode-buffer
                 (setq major-mode
                       'fixture-mode))
               (setq
                ace-window--test-buffers
                (list
                 (cons 'named named-buffer)
                 (cons 'mode mode-buffer)
                 (cons 'plain plain-buffer)
                 (cons 'child plain-buffer)
                 (cons 'selected plain-buffer)
                 (cons 'no-other plain-buffer)
                 (cons 'no-delete plain-buffer)))
               (cl-letf
                   (((symbol-function
                      'window-buffer)
                     (lambda (window)
                       (cdr
                        (assq
                         window
                         ace-window--test-buffers))))
                    ((symbol-function
                      'window-frame)
                     (lambda (_window)
                       'fixture-frame))
                    ((symbol-function
                      'frame-parent)
                     (lambda (_frame)
                       (and
                        (eq
                         ace-window--test-window
                         'child)
                        'parent-frame)))
                    ((symbol-function
                      'selected-window)
                     (lambda ()
                       'selected))
                    ((symbol-function
                      'window-parameter)
                     (lambda (window parameter)
                       (or
                        (and
                         (eq window 'no-other)
                         (eq parameter
                             'no-other-window))
                        (and
                         (eq window 'no-delete)
                         (eq parameter
                             'no-delete-other-windows))))))
                 (mapcar
                  (lambda (fixture)
                    (setq
                     ace-window--test-window
                     (nth 0 fixture))
                    (let ((aw-ignore-on
                           (nth 1 fixture))
                          (aw-ignore-current
                           (nth 2 fixture))
                          (ignore-window-parameters
                           (nth 3 fixture))
                          (this-command
                           (nth 4 fixture))
                          (aw-ignored-buffers
                           (list
                            (buffer-name
                             named-buffer)
                            'fixture-mode)))
                      (list
                       fixture
                       (and
                        (aw-ignored-p
                         ace-window--test-window)
                        t))))
                  '((named t nil nil
                           ace-select-window)
                    (mode t nil nil
                          ace-select-window)
                    (named nil nil nil
                           ace-select-window)
                    (child nil nil t
                           ace-select-window)
                    (selected nil t t
                              ace-select-window)
                    (no-other nil nil nil
                              ace-select-window)
                    (no-other nil nil nil
                              other-command)
                    (no-delete nil nil nil
                               ace-delete-window)
                    (no-delete nil nil nil
                               ace-delete-other-windows)
                    (no-delete nil nil t
                               ace-delete-window)
                    (plain nil nil nil
                           ace-select-window)))))
           (dolist
               (buffer
                (list
                 named-buffer
                 mode-buffer
                 plain-buffer))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![
        "OK (((named t nil nil ace-select-window) t) ((mode t nil nil ace-select-window) t) ((named nil nil nil ace-select-window) nil) ((child nil nil t ace-select-window) t) ((selected nil t t ace-select-window) t) ((no-other nil nil nil ace-select-window) t) ((no-other nil nil nil other-command) nil) ((no-delete nil nil nil ace-delete-window) t) ((no-delete nil nil nil ace-delete-other-windows) t) ((no-delete nil nil t ace-delete-window) nil) ((plain nil nil nil ace-select-window) nil))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_window_list_uses_each_scope_filters_unusable_windows_and_sorts() {
    let elisp_form = r##"(progn
         (setq
          ace-window--test-frame-windows
          '((visible-a va3 va1 ignored)
            (visible-b vb2 invisible)
            (global-only ga2 ga1 dead terminal)
            (current ca2 ca1))
          ace-window--test-window-frames
          '((va3 . visible-a)
            (va1 . visible-a)
            (ignored . visible-a)
            (vb2 . visible-b)
            (invisible . invisible-frame)
            (ga2 . global-only)
            (ga1 . global-only)
            (dead . dead-frame)
            (terminal . terminal-frame)
            (ca2 . current)
            (ca1 . current)))
         (cl-letf
             (((symbol-function
                'visible-frame-list)
               (lambda ()
                 '(visible-b visible-a)))
              ((symbol-function
                'frame-list)
               (lambda ()
                 '(global-only
                   visible-b
                   visible-a)))
              ((symbol-function
                'window-list)
               (lambda
                   (&optional frame
                    _minibuffer _window)
                 (cdr
                  (copy-sequence
                   (assq
                    (or frame 'current)
                    ace-window--test-frame-windows)))))
              ((symbol-function
                'window-frame)
               (lambda (window)
                 (cdr
                  (assq
                   window
                   ace-window--test-window-frames))))
              ((symbol-function
                'frame-live-p)
               (lambda (frame)
                 (not
                  (eq frame 'dead-frame))))
              ((symbol-function
                'frame-visible-p)
               (lambda (frame)
                 (not
                  (eq frame
                      'invisible-frame))))
              ((symbol-function
                'terminal-name)
               (lambda (frame)
                 (if (eq frame
                         'terminal-frame)
                     "initial_terminal"
                   "fixture-terminal")))
              ((symbol-function 'aw-ignored-p)
               (lambda (window)
                 (eq window 'ignored)))
              ((symbol-function 'aw-window<)
               (lambda (left right)
                 (string-lessp
                  (symbol-name left)
                  (symbol-name right)))))
           (mapcar
            (lambda (scope)
              (let ((aw-scope scope))
                (list
                 scope
                 (condition-case error
                     (list
                      'ok
                      (aw-window-list))
                   (error
                    (list 'error error))))))
            '(visible global frame
                      invalid))))"##;
    let expect = expect![[
        r#"OK ((visible (ok (va1 va3 vb2))) (global (ok (ga1 ga2 va1 va3 vb2))) (frame (ok (ca1 ca2))) (invalid (error (error "Invalid ‘aw-scope’: invalid"))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_ignored_predicate_short_circuits_buffer_checks_when_ignore_is_off() {
    let elisp_form = r##"(let ((aw-ignore-on nil)
             (aw-ignore-current nil)
             (ignore-window-parameters t))
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'window-buffer)
               (lambda (_window)
                 (push 'window-buffer
                       ace-window--test-events)
                 (current-buffer)))
              ((symbol-function
                'window-frame)
               (lambda (_window)
                 'fixture-frame))
              ((symbol-function
                'frame-parent)
               (lambda (_frame)
                 nil)))
           (list
            (aw-ignored-p
             'fixture-window)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_window_parity(elisp_form, expect);
}
