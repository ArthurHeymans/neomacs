use expect_test::expect;

use super::assert_atl_long_lines_parity;

#[test]
fn atl_long_lines_minor_mode_initial_metadata_lighter_keymap_and_hook_state_match() {
    let elisp_form = r##"(with-temp-buffer
         (list
          atl-long-lines-mode
          (local-variable-p
           'atl-long-lines-mode)
          (assq
           'atl-long-lines-mode
           minor-mode-alist)
          (assq
           'atl-long-lines-mode
           minor-mode-map-alist)
          (boundp
           'atl-long-lines-mode-hook)
          atl-long-lines-mode-hook
          (atl-long-lines-test-hook-count
           #'atl-long-lines--start-timer
           post-command-hook)
          (local-variable-p
           'post-command-hook)))"##;
    let expect = expect![[
        r#"OK (nil nil (atl-long-lines-mode " ATL-LL") nil t (atl-long-lines-mode--set-explicitly) 0 nil)"#
    ]];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn atl_long_lines_enabling_mode_installs_one_buffer_local_post_command_callback_idempotently() {
    let elisp_form = r##"(with-temp-buffer
         (let ((before
                post-command-hook))
           (atl-long-lines-mode 1)
           (let ((first
                  (list
                   atl-long-lines-mode
                   (local-variable-p
                    'atl-long-lines-mode)
                   (local-variable-p
                    'post-command-hook)
                   (atl-long-lines-test-hook-count
                    #'atl-long-lines--start-timer
                    post-command-hook))))
             (atl-long-lines-mode 1)
             (list
              before
              first
              atl-long-lines-mode
              (atl-long-lines-test-hook-count
               #'atl-long-lines--start-timer
               post-command-hook)
              post-command-hook))))"##;
    let expect = expect!["OK (nil (t t t 1) t 1 (atl-long-lines--start-timer t))"];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn atl_long_lines_disabling_mode_removes_only_its_local_callback() {
    let elisp_form = r##"(with-temp-buffer
         (let ((other
                (lambda () :other)))
           (add-hook
            'post-command-hook
            other
            nil
            t)
           (atl-long-lines-mode 1)
           (atl-long-lines-mode -1)
           (list
            atl-long-lines-mode
            (and
             (memq
              other
              post-command-hook)
             t)
            (atl-long-lines-test-hook-count
             #'atl-long-lines--start-timer
             post-command-hook)
            (local-variable-p
             'post-command-hook)
            (length
             (delq
              t
              (copy-sequence
               post-command-hook))))))"##;
    let expect = expect!["OK (nil t 0 t 1)"];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn atl_long_lines_mode_hook_sequence_for_repeated_enable_and_disable_matches() {
    let elisp_form = r##"(with-temp-buffer
         (let ((transitions nil))
           (add-hook
            'atl-long-lines-mode-hook
            (lambda ()
              (push
               atl-long-lines-mode
               transitions))
            nil
            t)
           (atl-long-lines-mode 1)
           (atl-long-lines-mode 1)
           (atl-long-lines-mode -1)
           (atl-long-lines-mode -1)
           (list
            (nreverse transitions)
            atl-long-lines-mode
            (atl-long-lines-test-hook-count
             #'atl-long-lines--start-timer
             post-command-hook))))"##;
    let expect = expect!["OK ((t t nil nil) nil 0)"];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn atl_long_lines_turn_on_helper_activates_only_the_current_buffer() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *atl-first*"))
               (second
                (generate-new-buffer
                 " *atl-second*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (atl-long-lines--turn-on-atl-long-lines-mode))
               (list
                (buffer-local-value
                 'atl-long-lines-mode
                 first)
                (buffer-local-value
                 'atl-long-lines-mode
                 second)
                (with-current-buffer first
                  (atl-long-lines-test-hook-count
                   #'atl-long-lines--start-timer
                   post-command-hook))
                (with-current-buffer second
                  (atl-long-lines-test-hook-count
                   #'atl-long-lines--start-timer
                   post-command-hook))))
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect!["OK (t nil 1 0)"];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn atl_long_lines_minor_mode_state_and_hooks_are_independent_across_buffers() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *atl-one*"))
               (second
                (generate-new-buffer
                 " *atl-two*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (atl-long-lines-mode 1))
               (with-current-buffer second
                 (atl-long-lines-mode 1)
                 (atl-long-lines-mode -1))
               (list
                (buffer-local-value
                 'atl-long-lines-mode
                 first)
                (buffer-local-value
                 'atl-long-lines-mode
                 second)
                (with-current-buffer first
                  (atl-long-lines-test-hook-count
                   #'atl-long-lines--start-timer
                   post-command-hook))
                (with-current-buffer second
                  (atl-long-lines-test-hook-count
                   #'atl-long-lines--start-timer
                   post-command-hook))))
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect!["OK (t nil 1 0)"];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn global_atl_long_lines_mode_updates_existing_ordinary_buffers_and_cleans_up() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 "atl-global-one"))
               (second
                (generate-new-buffer
                 "atl-global-two")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (fundamental-mode))
               (with-current-buffer second
                 (text-mode))
               (global-atl-long-lines-mode 1)
               (let ((enabled
                      (list
                       global-atl-long-lines-mode
                       (buffer-local-value
                        'atl-long-lines-mode
                        first)
                       (buffer-local-value
                        'atl-long-lines-mode
                        second))))
                 (global-atl-long-lines-mode -1)
                 (list
                  enabled
                  global-atl-long-lines-mode
                  (buffer-local-value
                   'atl-long-lines-mode
                   first)
                  (buffer-local-value
                   'atl-long-lines-mode
                   second)
                  (with-current-buffer first
                    (atl-long-lines-test-hook-count
                     #'atl-long-lines--start-timer
                     post-command-hook))
                  (with-current-buffer second
                    (atl-long-lines-test-hook-count
                     #'atl-long-lines--start-timer
                     post-command-hook)))))
           (global-atl-long-lines-mode -1)
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect!["OK ((t t t) nil nil nil 0 0)"];
    assert_atl_long_lines_parity(elisp_form, expect);
}

#[test]
fn global_atl_long_lines_mode_activates_buffers_created_after_global_enable() {
    let elisp_form = r##"(let (created)
         (unwind-protect
             (progn
               (global-atl-long-lines-mode 1)
               (setq
                created
                (generate-new-buffer
                 "atl-global-future"))
               (with-current-buffer created
                 (fundamental-mode))
               (list
                global-atl-long-lines-mode
                (buffer-local-value
                 'atl-long-lines-mode
                 created)
                (with-current-buffer created
                  (atl-long-lines-test-hook-count
                   #'atl-long-lines--start-timer
                   post-command-hook))))
           (global-atl-long-lines-mode -1)
           (when
               (buffer-live-p created)
             (kill-buffer created))))"##;
    let expect = expect!["OK (t t 1)"];
    assert_atl_long_lines_parity(elisp_form, expect);
}
