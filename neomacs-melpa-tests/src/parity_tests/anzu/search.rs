use expect_test::expect;

use super::assert_anzu_parity;

#[test]
fn anzu_validate_regexp_accepts_complex_patterns_and_rejects_native_invalid_forms() {
    let elisp_form = r##"(mapcar
         (lambda (regexp)
           (list regexp (anzu--validate-regexp regexp)))
         '("" "^$" "\\_<foo\\_>" "\\(?:a\\|b\\)+" "[[:alpha:]]+"
           "[" "\\(" "\\(?invalid" "*bad"))"##;
    let expect = expect![[
        r#"OK (("" t) ("^$" t) ("\\_<foo\\_>" t) ("\\(?:a\\|b\\)+" t) ("[[:alpha:]]+" t) ("[" nil) ("\\(" nil) ("\\(?invalid" nil) ("*bad" t))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_transform_input_distinguishes_literal_regexp_and_symbol_searches() {
    let elisp_form = r##"(list
         (let ((isearch-regexp nil)
               (isearch-regexp-function nil)
               (isearch-word nil)
               (anzu--last-command 'isearch-forward))
           (anzu--transform-input "a.b+c"))
         (let ((isearch-regexp t)
               (isearch-regexp-function nil)
               (isearch-word nil)
               (anzu--last-command 'isearch-forward-regexp))
           (anzu--transform-input "a.b+c"))
         (let ((isearch-regexp nil)
               (isearch-regexp-function 'isearch-symbol-regexp)
               (isearch-word nil)
               (anzu--last-command 'isearch-forward))
           (anzu--transform-input "alpha beta")))"##;
    let expect = expect![[r#"OK ("a\\.b\\+c" "a.b+c" "alpha beta")"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_use_migemo_p_handles_disabled_missing_and_enabled_feature_states() {
    let elisp_form = r##"(list
         (let ((anzu-use-migemo nil))
           (anzu--use-migemo-p))
         (let ((anzu-use-migemo t))
           (condition-case err
               (anzu--use-migemo-p)
             (error (list (car err) (cdr err)))))
         (let ((anzu-use-migemo t)
               (migemo-isearch-enable-p nil))
           (provide 'migemo)
           (prog1 (anzu--use-migemo-p)
             (setq features (delq 'migemo features))))
         (let ((anzu-use-migemo t)
               (migemo-isearch-enable-p t))
           (provide 'migemo)
           (prog1 (anzu--use-migemo-p)
             (setq features (delq 'migemo features)))))"##;
    let expect = expect![[r#"OK (nil (error ("Error: migemo is not loaded")) nil nil)"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_search_all_positions_finds_literal_metacharacters_and_updates_cache() {
    let elisp_form = r##"(with-temp-buffer
         (insert "a.b aXb a.b\nprefix a.b suffix\n")
         (let ((isearch-mode nil)
               (isearch-regexp nil)
               (case-fold-search nil)
               (anzu-search-threshold nil)
               (anzu-use-migemo nil)
               (anzu--last-command 'isearch-forward)
               (anzu--cached-positions 'old))
           (goto-char 7)
           (let ((result (anzu--search-all-position "a.b")))
             (list result
                   anzu--cached-positions
                   (eq result anzu--cached-positions)
                   (point)
                   anzu--last-command))))"##;
    let expect = expect![
        "OK ((:count 3 :overflow nil :positions #1=((1 . 4) (9 . 12) (20 . 23))) (:count 3 :overflow nil :positions #1#) nil 7 isearch-forward)"
    ];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_search_all_positions_applies_regexp_threshold_and_overflow() {
    let elisp_form = r##"(with-temp-buffer
         (insert "cat cot cut cit CAT cet")
         (let ((isearch-mode nil)
               (isearch-regexp t)
               (case-fold-search t)
               (anzu-search-threshold 4)
               (anzu-use-migemo nil)
               (anzu--last-command 'isearch-forward-regexp))
           (anzu--search-all-position "c.t")))"##;
    let expect =
        expect!["OK (:count 4 :overflow t :positions ((1 . 4) (5 . 8) (9 . 12) (13 . 16)))"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_search_all_positions_handles_zero_width_anchors_without_looping() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\ntwo\nthree")
         (let ((isearch-mode nil)
               (isearch-regexp t)
               (case-fold-search nil)
               (anzu-search-threshold nil)
               (anzu-use-migemo nil)
               (anzu--last-command 'isearch-forward-regexp))
           (list (anzu--search-all-position "^")
                 (anzu--search-all-position "$"))))"##;
    let expect = expect![
        "OK ((:count 3 :overflow nil :positions ((1 . 1) (5 . 5) (9 . 9))) (:count 3 :overflow nil :positions ((4 . 4) (8 . 8) (14 . 14))))"
    ];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_invalid_search_reuses_cached_position_object_unchanged() {
    let elisp_form = r##"(with-temp-buffer
         (insert "content")
         (let ((isearch-mode nil)
               (isearch-regexp t)
               (anzu--last-command 'isearch-forward-regexp)
               (anzu--cached-positions
                '(:count 7 :overflow old :positions ((1 . 2)))))
           (let ((result (anzu--search-all-position "[")))
             (list result
                   (eq result anzu--cached-positions)
                   anzu--cached-positions))))"##;
    let expect = expect!["OK (#1=(:count 7 :overflow old :positions ((1 . 2))) t #1#)"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_where_is_here_uses_inclusive_ranges_and_first_matching_index() {
    let elisp_form = r##"(let ((positions '((2 . 4) (4 . 7) (10 . 10))))
         (mapcar
          (lambda (point)
            (list point (anzu--where-is-here positions point)))
          '(1 2 3 4 5 7 8 10 11)))"##;
    let expect = expect!["OK ((1 0) (2 1) (3 1) (4 1) (5 2) (7 2) (8 0) (10 3) (11 0))"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_result_cache_requires_identical_search_state_and_non_toggle_command() {
    let elisp_form = r##"(let ((isearch-regexp nil)
               (isearch-regexp-function nil)
               (isearch-word nil)
               (anzu--last-search-state '(nil))
               (anzu--last-isearch-string "needle"))
         (list
          (let ((last-command 'isearch-repeat-forward))
            (anzu--use-result-cache-p "needle"))
          (let ((last-command 'isearch-repeat-forward)
                (isearch-regexp t))
            (anzu--use-result-cache-p "needle"))
          (let ((last-command 'isearch-repeat-forward))
            (anzu--use-result-cache-p "other"))
          (let ((last-command 'isearch-toggle-case-fold))
            (anzu--use-result-cache-p "needle"))))"##;
    let expect = expect!["OK (t nil nil nil)"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_update_uses_cached_results_updates_all_status_and_forces_mode_line() {
    let elisp_form = r##"(with-temp-buffer
         (insert "zero one two")
         (goto-char 7)
         (let ((isearch-regexp nil)
               (isearch-regexp-function nil)
               (isearch-word nil)
               (last-command 'isearch-repeat-forward)
               (anzu-minimum-input-length 1)
               (anzu--last-search-state '(nil))
               (anzu--last-isearch-string "one")
               (anzu--cached-positions
                '(:count 3 :overflow t
                  :positions ((1 . 2) (6 . 8) (10 . 12))))
               calls)
           (cl-letf (((symbol-function 'force-mode-line-update)
                      (lambda (&rest args)
                        (push args calls)
                        'forced)))
             (list (anzu--update "one")
                   anzu--total-matched anzu--overflow-p
                   anzu--current-position anzu--last-search-state
                   anzu--last-isearch-string
                   (nreverse calls)))))"##;
    let expect = expect![[r#"OK (forced 3 t 2 (nil) "one" (nil))"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_mode_line_default_formats_search_overflow_replace_and_no_match_faces() {
    let elisp_form = r##"(let ((isearch-string "needle"))
         (mapcar
          (lambda (case)
            (let ((anzu--state (nth 0 case))
                  (anzu--overflow-p (nth 1 case)))
              (let ((result
                     (anzu--update-mode-line-default
                      (nth 2 case) (nth 3 case))))
                (list case result
                      (and result
                           (get-text-property 0 'face result))))))
          '((search nil 2 7)
            (search t 0 1000)
            (search t 999 1000)
            (replace-query nil 0 12)
            (replace nil 4 12)
            (nil nil 0 0)
            (search nil 0 0))))"##;
    let expect = expect![[
        r#"OK (((search nil 2 7) #("(2/7)" 0 5 (face anzu-mode-line)) anzu-mode-line) ((search t 0 1000) #("(1000+/1000+)" 0 13 (face anzu-mode-line)) anzu-mode-line) ((search t 999 1000) #("(999/1000+)" 0 11 (face anzu-mode-line)) anzu-mode-line) ((replace-query nil 0 12) #("(12 replace)" 0 12 (face anzu-mode-line)) anzu-mode-line) ((replace nil 4 12) #("(4/12)" 0 6 (face anzu-mode-line)) anzu-mode-line) ((nil nil 0 0) nil nil) ((search nil 0 0) #("(0/0)" 0 5 (face anzu-mode-line-no-match)) anzu-mode-line-no-match))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_update_mode_line_delegates_current_values_to_custom_function() {
    let elisp_form = r##"(let ((anzu--current-position 8)
               (anzu--total-matched 21)
               calls)
         (let ((anzu-mode-line-update-function
                (lambda (&rest args)
                  (push args calls)
                  (format "<%s:%s>" (car args) (cadr args)))))
           (list (anzu--update-mode-line)
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK ("<8:21>" ((8 21)))"#]];
    assert_anzu_parity(elisp_form, expect);
}
