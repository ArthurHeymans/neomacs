use expect_test::expect;

use super::assert_anzu_parity;

#[test]
fn anzu_mode_enable_installs_three_local_hooks_and_disable_cleans_status() {
    let elisp_form = r##"(with-temp-buffer
         (let ((anzu--total-matched 9)
               (anzu--current-position 4)
               (anzu--state 'search)
               (anzu--last-command 'isearch-forward)
               (anzu--last-isearch-string "needle")
               (anzu--overflow-p t)
               (anzu--region-noncontiguous '((1 . 3))))
           (anzu-mode 1)
           (let ((enabled
                  (list anzu-mode
                        isearch-update-post-hook
                        isearch-mode-hook
                        isearch-mode-end-hook
                        (local-variable-p 'anzu--state))))
             (anzu-mode -1)
             (list enabled anzu-mode
                   isearch-update-post-hook isearch-mode-hook
                   isearch-mode-end-hook
                   anzu--total-matched anzu--current-position
                   anzu--state anzu--last-command
                   anzu--last-isearch-string anzu--overflow-p
                   anzu--region-noncontiguous))))"##;
    let expect = expect![
        "OK ((t (anzu--update-post-hook t) (anzu--cons-mode-line-search t) (anzu--reset-mode-line t) t) nil nil (multi-isearch-setup) nil 0 0 nil nil nil nil nil)"
    ];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_cons_and_reset_mode_line_are_idempotent_and_preserve_other_entries() {
    let elisp_form = r##"(let ((mode-line-format '("left" mode-line-buffer-identification))
               (anzu-cons-mode-line-p t))
         (anzu--cons-mode-line 'search)
         (anzu--cons-mode-line 'replace)
         (let ((with-anzu (copy-tree mode-line-format)))
           (anzu--reset-mode-line)
           (list with-anzu mode-line-format anzu--state
                 anzu--total-matched anzu--current-position)))"##;
    let expect = expect![[
        r#"OK (((:eval (anzu--update-mode-line)) "left" mode-line-buffer-identification) ("left" mode-line-buffer-identification) nil 0 0)"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_cons_mode_line_disabled_or_nonlist_changes_state_without_insertion() {
    let elisp_form = r##"(list
         (let ((mode-line-format '("base"))
               (anzu-cons-mode-line-p nil)
               anzu--state)
           (anzu--cons-mode-line 'search)
           (list anzu--state mode-line-format))
         (let ((mode-line-format "literal")
               (anzu-cons-mode-line-p t)
               anzu--state)
           (anzu--cons-mode-line 'replace-query)
           (list anzu--state mode-line-format)))"##;
    let expect = expect![[r#"OK ((search ("base")) (replace-query "literal"))"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_turn_on_skips_minibuffer_and_enables_ordinary_buffer() {
    let elisp_form = r##"(mapcar
         (lambda (inside-minibuffer)
           (let (calls)
             (cl-letf (((symbol-function 'minibufferp)
                        (lambda (&optional buffer)
                          (push (list 'minibuffer buffer) calls)
                          inside-minibuffer))
                       ((symbol-function 'anzu-mode)
                        (lambda (&rest args)
                          (push (cons 'mode args) calls)
                          'enabled)))
               (list inside-minibuffer
                     (anzu--turn-on)
                     (nreverse calls)))))
         '(nil t))"##;
    let expect =
        expect![["OK ((nil enabled ((minibuffer nil) (mode 1))) (t nil ((minibuffer nil))))"]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_query_prompt_base_and_defaults_cover_region_regexp_word_and_context() {
    let elisp_form = r##"(let ((anzu--query-defaults
                '(("old.value" . "new-value"))))
         (list
          (let ((current-prefix-arg nil))
            (mapcar
             (lambda (args)
               (apply #'anzu--query-prompt-base args))
             '((nil nil) (t nil) (nil t) (t t))))
          (let ((current-prefix-arg 4))
            (anzu--query-prompt-base t t))
          (let ((current-prefix-arg nil))
            (list
             (anzu--query-prompt nil nil nil nil)
             (anzu--query-prompt t t t nil)
             (anzu--query-prompt t t nil t)))))"##;
    let expect = expect![[
        r#"OK (("Query replace" "Query replace in region" "Query replace regexp" "Query replace regexp in region") "Query replace word regexp in region" ("Query replace (default old.value -> new-value) " "Query replace regexp in region" "Query replace regexp in region"))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_thing_and_region_helpers_cover_bounds_fallback_prefix_and_backward() {
    let elisp_form = r##"(with-temp-buffer
         (insert "first line\nalpha beta\nthird line\n")
         (goto-char 14)
         (set-mark 20)
         (setq mark-active t)
         (list
          (anzu--thing-begin 'word)
          (anzu--thing-end 'word)
          (anzu--thing-begin 'defun)
          (anzu--thing-end 'defun)
          (anzu--begin-thing t nil)
          (anzu--begin-thing t 'line)
          (anzu--begin-thing nil 'word)
          (let ((current-prefix-arg nil))
            (list (anzu--region-begin nil nil nil)
                  (anzu--region-end nil nil nil)))
          (let ((current-prefix-arg 2))
            (list (anzu--region-begin nil nil nil)
                  (anzu--region-end nil nil nil)))
          (list (anzu--region-begin nil nil t)
                (anzu--region-end nil nil t))
          (list (anzu--region-begin t nil nil)
                (anzu--region-end t nil nil))))"##;
    let expect = expect!["OK (12 17 12 34 symbol line nil (14 34) (12 33) (14 1) (14 20))"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_replace_direction_and_argument_constructors_match_builtin_contracts() {
    let elisp_form = r##"(let ((anzu--region-noncontiguous
                '((1 . 4) (8 . 11))))
         (list
          (mapcar #'anzu--replace-backward-p '(nil 0 2 -1 -9))
          (anzu--construct-perform-replace-arguments
           "from" "to" t 3 30 nil t)
          (anzu--construct-perform-replace-arguments
           "from" 'compiled nil 30 3 t nil)
          (anzu--construct-query-replace-arguments
           "from" "to" t 3 30 nil)
          (anzu--construct-query-replace-arguments
           "from" "to" nil 30 3 t)))"##;
    let expect = expect![[
        r#"OK ((nil nil nil t t) ("from" "to" t t t nil nil 3 30 nil #1=((1 . 4) (8 . 11))) ("from" compiled nil t nil nil nil 30 3 t #1#) ("from" "to" t 3 30 nil #1#) ("from" "to" nil 30 3 t #1#))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_public_replace_wrappers_forward_exact_keyword_arguments() {
    let elisp_form = r##"(let ((anzu-replace-at-cursor-thing 'paragraph)
               calls)
         (cl-letf (((symbol-function 'anzu--query-replace-common)
                    (lambda (&rest args)
                      (push args calls)
                      'done))
                   ((symbol-function 'goto-char)
                    (lambda (&rest args)
                      (push (cons 'goto args) calls)))
                   ((symbol-function 'set-marker)
                    (lambda (&rest args)
                      (push (cons 'marker args) calls))))
           (list
            (anzu-query-replace-at-cursor)
            (anzu-query-replace-at-cursor-thing)
            (anzu-query-replace 4)
            (anzu-query-replace-regexp -2)
            (anzu-replace-at-cursor-thing)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (done done done done #1=((marker (:marker 1 "*scratch*") nil)) ((t :at-cursor t) (t :at-cursor t :thing paragraph) (nil :prefix-arg 4) (t :prefix-arg -2) (t :at-cursor t :thing paragraph :query nil) (goto 1) . #1#))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_isearch_replace_common_finishes_search_moves_boundary_and_forwards() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (goto-char 8)
         (let ((isearch-other-end 3)
               (transient-mark-mode nil)
               (mark-active nil)
               calls)
           (cl-letf (((symbol-function 'isearch-done)
                      (lambda (&rest args)
                        (push (cons 'done args) calls)))
                     ((symbol-function 'isearch-clean-overlays)
                      (lambda (&rest args)
                        (push (cons 'clean args) calls)))
                     ((symbol-function 'anzu--query-replace-common)
                      (lambda (&rest args)
                        (push (cons 'replace args) calls)
                        'replaced)))
             (list (anzu--isearch-query-replace-common nil 1)
                   (point)
                   (nreverse calls)))))"##;
    let expect =
        expect!["OK (replaced 3 ((done nil t) (clean) (replace nil :prefix-arg 1 :isearch-p t)))"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_isearch_public_wrappers_choose_literal_and_regexp_modes() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'anzu--isearch-query-replace-common)
                    (lambda (&rest args)
                      (push args calls)
                      'done)))
           (list (anzu-isearch-query-replace 3)
                 (anzu-isearch-query-replace-regexp -4)
                 (nreverse calls))))"##;
    let expect = expect!["OK (done done ((nil 3) (t -4)))"];
    assert_anzu_parity(elisp_form, expect);
}
