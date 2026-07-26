use super::assert_ace_window_posframe_parity;
use expect_test::expect;

#[test]
fn ace_window_posframe_surface_defaults_arglists_mode_metadata_and_sources_match() {
    let elisp_form = r##"(list
         (featurep 'ace-window)
         (featurep
          'ace-window-posframe)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (help-function-arglist
              symbol
              t)
             (commandp symbol)
             (interactive-form symbol)
             (and
              (documentation symbol)
              t)
             (file-name-nondirectory
              (symbol-file
               symbol
               'defun))))
          '(aw--lead-overlay-posframe
            aw--remove-leading-chars-posframe
            ace-window-posframe-enable
            ace-window-posframe-disable
            ace-window-posframe-mode))
         aw--posframe-frames
         aw-posframe-position-handler
         ace-window-posframe-mode
         (default-value
          'ace-window-posframe-mode)
         (let ((standard
                (get
                 'ace-window-posframe-mode
                 'standard-value)))
           (list
            (and standard t)
            (and standard
                 (eval
                  (car standard)
                  t))))
         (get
          'ace-window-posframe-mode
          'custom-type)
         (get
          'ace-window-posframe-mode
          'custom-group))"##;
    let expect = expect![[
        r#"OK (t t ((aw--lead-overlay-posframe (path leaf) nil nil nil "ace-window-posframe.el") (aw--remove-leading-chars-posframe nil nil nil nil "ace-window-posframe.el") (ace-window-posframe-enable nil nil nil nil "ace-window-posframe.el") (ace-window-posframe-disable nil nil nil nil "ace-window-posframe.el") (ace-window-posframe-mode (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) t "ace-window-posframe.el")) nil posframe-poshandler-window-center nil nil (t nil) boolean nil)"#
    ]];
    assert_ace_window_posframe_parity(elisp_form, expect);
}

#[test]
fn ace_window_posframe_lead_renderer_forwards_path_window_face_and_inherited_colors() {
    let elisp_form = r##"(save-window-excursion
         (let* ((origin
                 (selected-window))
                (target
                 (split-window
                  origin
                  nil
                  'below))
                (aw--posframe-frames nil)
                (aw-posframe-position-handler
                 'fixture-handler))
           (setq ace-window--test-events nil)
           (setq ace-window--test-target-window
                 target)
           (cl-letf
               (((symbol-function 'posframe-show)
                 (lambda
                     (buffer
                      &rest arguments)
                   (push
                    (list
                     'show
                     (eq
                      (selected-window)
                      ace-window--test-target-window)
                     buffer
                     arguments)
                    ace-window--test-events)
                   'show-result))
                ((symbol-function 'face-font)
                 (lambda (face
                          &optional frame
                          inherit)
                   (push
                    (list
                     'font
                     (eq
                      (selected-window)
                      ace-window--test-target-window)
                     face
                     frame
                     inherit)
                    ace-window--test-events)
                   "fixture-font"))
                ((symbol-function
                  'face-foreground)
                 (lambda (face
                          &optional frame
                          inherit)
                   (push
                    (list
                     'foreground
                     (eq
                      (selected-window)
                      ace-window--test-target-window)
                     face
                     frame
                     inherit)
                    ace-window--test-events)
                   "fixture-foreground"))
                ((symbol-function
                  'face-background)
                 (lambda (face
                          &optional frame
                          inherit)
                   (push
                    (list
                     'background
                     (eq
                      (selected-window)
                      ace-window--test-target-window)
                     face
                     frame
                     inherit)
                    ace-window--test-events)
                   "fixture-background")))
             (list
              (aw--lead-overlay-posframe
               '(97 98)
               (cons 1 target))
              (eq
               (selected-window)
               origin)
              aw--posframe-frames
              (nreverse
               ace-window--test-events)))))"##;
    let expect = expect![[
        r#"OK (show-result t (" *aw-posframe-buffer-(97 98)*") ((font t aw-leading-char-face nil nil) (foreground t aw-leading-char-face nil t) (background t aw-leading-char-face nil t) (show t " *aw-posframe-buffer-(97 98)*" (:string "ab" :poshandler fixture-handler :font "fixture-font" :foreground-color "fixture-foreground" :background-color "fixture-background"))))"#
    ]];
    assert_ace_window_posframe_parity(elisp_form, expect);
}

#[test]
fn ace_window_posframe_cleanup_hides_every_reusable_frame_buffer_and_clears_state() {
    let elisp_form = r##"(let ((aw--posframe-frames
              '("frame-a"
                "frame-b"
                "frame-a")))
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function 'posframe-hide)
               (lambda (buffer)
                 (push buffer
                       ace-window--test-events)
                 'hidden)))
           (list
            (aw--remove-leading-chars-posframe)
            aw--posframe-frames
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect![[r#"OK (nil nil ("frame-a" "frame-b" "frame-a"))"#]];
    assert_ace_window_posframe_parity(elisp_form, expect);
}

#[test]
fn ace_window_posframe_enable_requires_available_workable_backend_before_switching_functions() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-require
            (nth 0 fixture)
            ace-window--test-workable
            (nth 1 fixture))
           (let ((aw--lead-overlay-fn
                  'original-lead)
                 (aw--remove-leading-chars-fn
                  'original-remove))
             (cl-letf
                 (((symbol-function 'require)
                   (lambda
                       (feature
                        &optional filename
                        noerror)
                     (push
                      (list
                       'require
                       feature
                       filename
                       noerror)
                      ace-window--test-events)
                     ace-window--test-require))
                  ((symbol-function
                    'posframe-workable-p)
                   (lambda ()
                     (push 'workable
                           ace-window--test-events)
                     ace-window--test-workable)))
               (list
                fixture
                (condition-case error
                    (list
                     'ok
                     (ace-window-posframe-enable))
                  (error
                   (list 'error error)))
                aw--lead-overlay-fn
                aw--remove-leading-chars-fn
                (nreverse
                 ace-window--test-events)))))
         '((nil nil)
           (t nil)
           (t t)))"##;
    let expect = expect![[
        r#"OK (((nil nil) (error (error "Posframe is not workable")) original-lead original-remove ((require posframe nil t))) ((t nil) (error (error "Posframe is not workable")) original-lead original-remove ((require posframe nil t) workable)) ((t t) (ok aw--remove-leading-chars-posframe) aw--lead-overlay-posframe aw--remove-leading-chars-posframe ((require posframe nil t) workable)))"#
    ]];
    assert_ace_window_posframe_parity(elisp_form, expect);
}

#[test]
fn ace_window_posframe_disable_restores_overlay_backend_functions() {
    let elisp_form = r##"(let ((aw--lead-overlay-fn
              'fixture-lead)
             (aw--remove-leading-chars-fn
              'fixture-remove))
         (list
          (ace-window-posframe-disable)
          aw--lead-overlay-fn
          aw--remove-leading-chars-fn))"##;
    let expect = expect!["OK (aw--remove-leading-chars aw--lead-overlay aw--remove-leading-chars)"];
    assert_ace_window_posframe_parity(elisp_form, expect);
}

#[test]
fn ace_window_posframe_mode_lifecycle_calls_enable_disable_and_hooks_on_repeated_arguments() {
    let elisp_form = r##"(progn
         (ace-window-posframe-mode -1)
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'ace-window-posframe-enable)
               (lambda ()
                 (push
                  (list
                   'enable
                   ace-window-posframe-mode)
                  ace-window--test-events)
                 'enabled))
              ((symbol-function
                'ace-window-posframe-disable)
               (lambda ()
                 (push
                  (list
                   'disable
                   ace-window-posframe-mode)
                  ace-window--test-events)
                 'disabled))
              ((symbol-function
                'ace-window--test-posframe-hook)
               (lambda ()
                 (push
                  (list
                   'hook
                   ace-window-posframe-mode)
                  ace-window--test-events))))
           (let ((ace-window-posframe-mode-hook
                  '(ace-window--test-posframe-hook)))
             (unwind-protect
                 (mapcar
                  (lambda (argument)
                    (ace-window-posframe-mode
                     argument)
                    (list
                     argument
                     ace-window-posframe-mode
                     (nreverse
                      (prog1
                          ace-window--test-events
                        (setq
                         ace-window--test-events
                         nil)))))
                  '(1 1 toggle nil -1))
               (ace-window-posframe-mode
                -1)))))"##;
    let expect = expect![
        "OK ((1 t ((enable t) (hook t))) (1 t ((enable t) (hook t))) (toggle nil ((disable nil) (hook nil))) (nil t ((enable t) (hook t))) (-1 nil ((disable nil) (hook nil))))"
    ];
    assert_ace_window_posframe_parity(elisp_form, expect);
}
