use expect_test::expect;

use super::assert_artbollocks_mode_parity;

#[test]
fn artbollocks_minor_mode_lifecycle_routes_enable_disable_and_toggle_through_keywords_and_flush() {
    let elisp_form = r##"(with-temp-buffer
         (let (calls)
           (cl-letf
               (((symbol-function
                  'artbollocks-add-keywords)
                 (lambda ()
                   (push
                    (list :add)
                    calls)
                   :added))
                ((symbol-function
                  'artbollocks-remove-keywords)
                 (lambda ()
                   (push
                    (list :remove)
                    calls)
                   :removed))
                ((symbol-function
                  'font-lock-flush)
                 (lambda (&rest arguments)
                   (push
                    (cons :flush arguments)
                    calls)
                   :flushed)))
             (list
              artbollocks-mode
              (artbollocks-mode 1)
              artbollocks-mode
              (artbollocks-mode 1)
              artbollocks-mode
              (artbollocks-mode -1)
              artbollocks-mode
              (artbollocks-mode nil)
              artbollocks-mode
              (nreverse calls)))))"##;
    let expect = expect![
        "OK (nil t t t t nil nil t t ((:add) (:flush) (:add) (:flush) (:remove) (:flush) (:add) (:flush)))"
    ];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_minor_mode_is_buffer_local_and_independent_across_real_text_and_lisp_buffers() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *artbollocks-first*"))
               (second
                (generate-new-buffer
                 " *artbollocks-second*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (text-mode)
                 (insert
                  "many contextual works")
                 (artbollocks-mode 1))
               (with-current-buffer second
                 (emacs-lisp-mode)
                 (insert
                  ";; many contextual works")
                 (artbollocks-mode -1))
               (let ((initial
                      (list
                       (with-current-buffer first
                         (list
                          artbollocks-mode
                          (local-variable-p
                           'artbollocks-mode)
                          major-mode))
                       (with-current-buffer second
                         (list
                          artbollocks-mode
                          (local-variable-p
                           'artbollocks-mode)
                          major-mode)))))
                 (with-current-buffer second
                   (artbollocks-mode 1))
                 (list
                  initial
                  (with-current-buffer first
                    artbollocks-mode)
                  (with-current-buffer second
                    artbollocks-mode)
                  (default-value
                   'artbollocks-mode))))
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect!["OK (((t t text-mode) (nil t emacs-lisp-mode)) t t nil)"];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_minor_mode_hook_runs_after_body_with_final_state_on_every_invocation() {
    let elisp_form = r##"(with-temp-buffer
         (let ((artbollocks-mode-hook
                nil)
               events)
           (add-hook
            'artbollocks-mode-hook
            (lambda ()
              (push
               (list
                artbollocks-mode
                (buffer-name)
                major-mode)
               events)))
           (cl-letf
               (((symbol-function
                  'font-lock-flush)
                 (lambda (&rest _)
                   nil)))
             (artbollocks-mode 1)
             (artbollocks-mode -1)
             (artbollocks-mode nil)
             (list
              artbollocks-mode
              (nreverse events)
              (local-variable-p
               'artbollocks-mode-hook)))))"##;
    let expect = expect![[
        r#"OK (t ((t " *temp*" fundamental-mode) (nil " *temp*" fundamental-mode) (t " *temp*" fundamental-mode)) nil)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_minor_mode_lighter_keymap_and_minor_mode_registry_contracts_activate_in_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (artbollocks-mode 1)
         (list
          (assq
           'artbollocks-mode
           minor-mode-alist)
          (let ((entry
                 (assq
                  'artbollocks-mode
                  minor-mode-map-alist)))
            (list
             (car entry)
             (keymapp
              (cdr entry))
             (eq
              (cdr entry)
              artbollocks-mode-keymap)))
          (key-binding
           (kbd
            "C-c ["))
          (key-binding
           (kbd
            "C-c ]"))
          (key-binding
           (kbd
            "C-c \\"))
          (key-binding
           (kbd
            "C-c /"))
          (key-binding
           (kbd
            "C-c ="))
          (key-binding
           (kbd
            "C-c x"))
          (format-mode-line
           minor-mode-alist)))"##;
    let expect = expect![[
        r#"OK ((artbollocks-mode " AB") (artbollocks-mode t t) artbollocks-word-count artbollocks-sentence-count artbollocks-readability-index artbollocks-reading-ease artbollocks-grade-level nil "")"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_keybindings_execute_real_metric_commands_on_active_region_and_emit_results() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (insert
          "Ignore this sentence. "
          "The cat sat. The dog ran. "
          "Ignore this too.")
         (artbollocks-mode 1)
         (goto-char
          (point-min))
         (search-forward
          "The cat")
         (set-mark
          (match-beginning 0))
         (search-forward
          "ran.")
         (let ((transient-mark-mode
                t)
               (mark-active
                t)
               messages)
           (cl-letf
               (((symbol-function
                  'message)
                 (lambda (format-string &rest arguments)
                   (let ((rendered
                          (apply
                           #'format
                           format-string
                           arguments)))
                     (push rendered messages)
                     rendered))))
             (list
              (call-interactively
               (key-binding
                (kbd
                 "C-c [")))
              (call-interactively
               (key-binding
                (kbd
                 "C-c ]")))
              (call-interactively
               (key-binding
                (kbd
                 "C-c \\")))
              (call-interactively
               (key-binding
                (kbd
                 "C-c /")))
              (call-interactively
               (key-binding
                (kbd
                 "C-c =")))
              (nreverse messages)
              (buffer-string)
              (region-beginning)
              (region-end)))))"##;
    let expect = expect![[
        r#"OK (6 2 "Readability index: -5.800000000000001" "Reading ease: 119.18900000000002" "Grade level: -2.619999999999999" ("Word count: 6" "Sentence count: 2" "Readability index: -5.800000000000001" "Reading ease: 119.18900000000002" "Grade level: -2.619999999999999") "Ignore this sentence. The cat sat. The dog ran. Ignore this too." 23 48)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_enable_preserves_text_point_mark_narrowing_modified_state_and_major_mode() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; outside\n"
          ";; many contextual works\n"
          ";; tail\n")
         (set-buffer-modified-p
          nil)
         (goto-char
          (point-min))
         (forward-line 1)
         (let ((start
                (point)))
           (forward-line 1)
           (narrow-to-region
            start
            (point))
           (goto-char
            (point-max))
           (set-mark
            (point-min))
           (let ((before
                  (list
                   (buffer-string)
                   (point)
                   (mark)
                   (point-min)
                   (point-max)
                   (buffer-modified-p)
                   major-mode)))
             (artbollocks-mode 1)
             (list
              before
              artbollocks-mode
              (buffer-string)
              (point)
              (mark)
              (point-min)
              (point-max)
              (buffer-modified-p)
              major-mode))))"##;
    let expect = expect![[
        r#"OK ((";; many contextual works\n" 37 12 12 37 nil emacs-lisp-mode) t ";; many contextual works\n" 37 12 12 37 nil emacs-lisp-mode)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_repeated_real_enable_disable_cycles_do_not_accumulate_duplicate_font_lock_entries()
 {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; many contextual works were completed\n")
         (let (states)
           (dotimes (_ 3)
             (artbollocks-mode 1)
             (font-lock-ensure)
             (push
              (list
               :enabled
               (artbollocks-test-face-runs)
               (length
                font-lock-keywords))
              states)
             (artbollocks-mode -1)
             (font-lock-ensure)
             (push
              (list
               :disabled
               (artbollocks-test-face-runs)
               (length
                font-lock-keywords))
              states))
           (nreverse states)))"##;
    let expect = expect![[
        r#"OK ((:enabled (("many" artbollocks-weasel-words-face) ("contextual" artbollocks-face) ("works" artbollocks-face)) 24) (:disabled nil 20) (:enabled (("many" artbollocks-weasel-words-face) ("contextual" artbollocks-face) ("works" artbollocks-face)) 24) (:disabled nil 20) (:enabled (("many" artbollocks-weasel-words-face) ("contextual" artbollocks-face) ("works" artbollocks-face)) 24) (:disabled nil 20))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}
