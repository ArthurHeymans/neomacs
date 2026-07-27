use expect_test::expect;

use super::assert_anzu_parity;

#[test]
fn anzu_make_overlay_and_clear_overlays_preserve_unrelated_overlays() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let ((first (anzu--make-overlay 2 5 'anzu-match-1 1001))
               (second (anzu--make-overlay 6 9 'anzu-replace-highlight 1000))
               (other (make-overlay 1 3)))
           (anzu--clear-overlays (current-buffer) 4 8)
           (list
            (mapcar
             (lambda (overlay)
               (list (overlay-start overlay)
                     (overlay-end overlay)
                     (overlay-get overlay 'face)
                     (overlay-get overlay 'priority)
                     (overlay-get overlay 'anzu-overlay)
                     (overlay-buffer overlay)))
             (list first second other))
            (length (overlays-in (point-min) (point-max))))))"##;
    let expect = expect![
        "OK (((nil nil anzu-match-1 1001 t nil) (nil nil anzu-replace-highlight 1000 t nil) (1 3 nil nil nil (:buffer nil))) 1)"
    ];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_add_overlay_builds_group_and_replacement_overlays_from_match_data() {
    let elisp_form = r##"(with-temp-buffer
         (insert "prefix abc-123 suffix")
         (goto-char (point-min))
         (re-search-forward "\\(abc\\)-\\([0-9]+\\)")
         (anzu--add-overlay (match-beginning 0) (match-end 0))
         (mapcar
          (lambda (overlay)
            (list (overlay-start overlay) (overlay-end overlay)
                  (overlay-get overlay 'face)
                  (overlay-get overlay 'priority)
                  (overlay-get overlay 'anzu-overlay)
                  (overlay-get overlay 'anzu-replace)
                  (overlay-get overlay 'from-string)
                  (buffer-substring-no-properties
                   (overlay-start overlay) (overlay-end overlay))))
          (sort (overlays-in (point-min) (point-max))
                (lambda (a b)
                  (if (= (overlay-start a) (overlay-start b))
                      (< (overlay-end a) (overlay-end b))
                    (< (overlay-start a) (overlay-start b)))))))"##;
    let expect = expect![[
        r#"OK ((8 11 anzu-match-1 1001 t nil nil "abc") (8 15 anzu-replace-highlight 1000 t t "abc-123" "abc-123") (12 15 anzu-match-2 1001 t nil nil "123"))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_count_and_highlight_matches_forward_casefold_and_overlay_limit() {
    let elisp_form = r##"(with-temp-buffer
         (insert "foo FOO food foo\nfoo")
         (let ((isearch-mode nil)
               (case-fold-search t)
               (replace-lax-whitespace nil)
               (anzu--region-noncontiguous nil)
               (anzu--cached-count 0))
           (let ((overlayed
                  (anzu--count-and-highlight-matched
                   (current-buffer) "foo" (point-min) (point-max)
                   nil 13 nil)))
             (list overlayed anzu--cached-count
                   (mapcar
                    (lambda (overlay)
                      (list (overlay-start overlay)
                            (overlay-end overlay)
                            (overlay-get overlay 'from-string)))
                    (sort
                     (cl-remove-if-not
                      (lambda (overlay)
                        (overlay-get overlay 'anzu-replace))
                      (overlays-in (point-min) (point-max)))
                     #'anzu--overlay-sort))))))"##;
    let expect = expect![[r#"OK (3 5 ((1 4 "foo") (5 8 "FOO") (9 12 "foo")))"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_count_and_highlight_backward_and_noncontiguous_regions_are_exact() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one x one x one x one")
         (let ((isearch-mode nil)
               (case-fold-search nil)
               (replace-lax-whitespace nil)
               (anzu--region-noncontiguous
                '((1 . 4) (9 . 12) (21 . 24)))
               (anzu--cached-count 0))
           (let ((overlayed
                  (anzu--count-and-highlight-matched
                   (current-buffer) "one" (point-max) (point-min)
                   nil 7 t)))
             (list overlayed anzu--cached-count
                   (mapcar
                    (lambda (overlay)
                      (list (overlay-start overlay)
                            (overlay-end overlay)))
                    (sort
                     (cl-remove-if-not
                      (lambda (overlay)
                        (overlay-get overlay 'anzu-replace))
                      (overlays-in (point-min) (point-max)))
                     #'anzu--overlay-sort))))))"##;
    let expect = expect!["OK (0 1 nil)"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_marker_lifecycle_tracks_matches_indexes_and_cleans_every_marker() {
    let elisp_form = r##"(with-temp-buffer
         (insert "cat dog cat cat")
         (let ((anzu--replaced-markers nil))
           (anzu--set-replaced-markers "cat" (point-min) (point-max) nil)
           (let ((before
                  (mapcar
                   (lambda (marker)
                     (list (marker-position marker)
                           (marker-buffer marker)))
                   anzu--replaced-markers)))
             (let ((indexes
                    (mapcar #'anzu--current-replaced-index
                            '(1 9 13 4))))
               (let ((markers (copy-sequence anzu--replaced-markers)))
                 (anzu--cleanup-markers)
                 (list before indexes
                       anzu--replaced-markers
                       (mapcar #'marker-position markers)))))))"##;
    let expect = expect![
        "OK (((13 (:buffer nil)) (9 (:buffer nil)) (1 (:buffer nil))) (3 2 1 nil) nil (nil nil nil))"
    ];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_lax_whitespace_conversion_covers_literal_and_regexp_switches() {
    let elisp_form = r##"(let ((search-whitespace-regexp "[ \t\n]+"))
         (list
          (let ((replace-lax-whitespace nil)
                (replace-regexp-lax-whitespace nil))
            (list (anzu--convert-for-lax-whitespace "a  b.c" nil)
                  (anzu--convert-for-lax-whitespace "a  b.c" t)))
          (let ((replace-lax-whitespace t)
                (replace-regexp-lax-whitespace t))
            (list (anzu--convert-for-lax-whitespace "a  b.c" nil)
                  (anzu--convert-for-lax-whitespace "a  b.c" t)))))"##;
    let expect = expect![[r#"OK (("a  b\\.c" "a  b.c") ("a[ \11\n]+b\\.c" "a[ \11\n]+b.c"))"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_overlay_range_predicate_and_threshold_sorting_cover_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (insert "01234567890123456789")
         (let ((one (anzu--make-overlay 8 10 'a 1))
               (two (anzu--make-overlay 2 4 'b 1))
               (three (anzu--make-overlay 14 17 'c 1))
               (other (make-overlay 5 7)))
           (dolist (overlay (list one two three))
             (overlay-put overlay 'anzu-replace t))
           (list
            (let ((anzu--region-noncontiguous nil))
              (mapcar
               (lambda (bounds)
                 (apply #'anzu2--put-overlay-p bounds))
               '((2 4 2 17) (1 4 2 17) (14 18 2 17))))
            (let ((anzu--region-noncontiguous
                   '((2 . 4) (14 . 17))))
              (mapcar
               (lambda (bounds)
                 (apply #'anzu2--put-overlay-p bounds))
               '((2 4 2 17) (8 10 2 17) (14 17 2 17))))
            (let ((anzu-replace-threshold 2))
              (mapcar
               (lambda (overlay)
                 (list (overlay-start overlay)
                       (overlay-end overlay)))
               (anzu--overlays-in-range 1 20)))
            (overlay-buffer other))))"##;
    let expect = expect!["OK ((t nil nil) (t nil t) ((2 4) (8 10)) (:buffer nil))"];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_literal_replacement_and_to_string_properties_pin_case_behavior() {
    let elisp_form = r##"(with-temp-buffer
         (insert "FOO foo")
         (let ((first (make-overlay 1 4))
               (second (make-overlay 5 8))
               (anzu-replace-to-string-separator " => "))
           (list
            (let ((case-fold-search t))
              (list (anzu--replaced-literal-string
                     first "bar" "foo")
                    (anzu--replaced-literal-string
                     second "BAR" "foo")))
            (let ((case-fold-search nil))
              (list (anzu--replaced-literal-string
                     first "bar" "foo")
                    (anzu--replaced-literal-string
                     second "BAR" "foo")))
            (let ((result (anzu--propertize-to-string "replacement")))
              (list result (text-properties-at 0 result))))))"##;
    let expect = expect![[
        r#"OK (("BAR" "BAR") (nil "BAR") (#(" => replacement" 0 15 (face anzu-replace-to)) (face anzu-replace-to)))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_compile_and_evaluate_regexp_replacements_cover_literals_groups_and_eval() {
    let elisp_form = r##"(with-temp-buffer
         (insert "name=ada count=7")
         (let ((overlay (make-overlay 1 (point-max))))
           (overlay-put overlay 'from-string (buffer-string))
           (list
            (mapcar #'anzu--compile-replace-text
                    '("literal" "\\1-updated"
                      "\\,(upcase \\1)" "\\,(error \"boom\")"))
            (anzu--evaluate-occurrence
             overlay "\\1-\\2" 0 nil
             "name=\\([a-z]+\\) count=\\([0-9]+\\)")
            (anzu--evaluate-occurrence
             overlay "\\,(upcase \\1)" 0 nil
             "name=\\([a-z]+\\) count=[0-9]+")
            (anzu--evaluate-occurrence
             overlay "\\,(error \"boom\")" 0 nil
             "name=\\([a-z]+\\) count=[0-9]+"))))"##;
    let expect = expect![[
        r#"OK (("literal" "\\1-updated" (replace-eval-replacement replace-quote (upcase (match-string 1))) (replace-eval-replacement replace-quote (error "boom"))) "ada-7" "ADA" "")"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_append_replaced_string_updates_sorted_overlays_and_caches_input() {
    let elisp_form = r##"(with-temp-buffer
         (insert "foo xx foo yy foo")
         (goto-char (point-min))
         (while (re-search-forward "foo" nil t)
           (let ((overlay
                  (make-overlay (match-beginning 0) (match-end 0))))
             (overlay-put overlay 'anzu-replace t)
             (overlay-put overlay 'from-string
                          (match-string-no-properties 0))))
         (let ((isearch-mode nil)
               (case-fold-search nil)
               (replace-lax-whitespace nil)
               (anzu-replace-threshold 2)
               (anzu-replace-to-string-separator " -> ")
               (anzu--last-replace-input ""))
           (anzu--append-replaced-string
            "bar" (current-buffer) 1 (point-max) nil
            (point-max) "foo")
           (let ((first
                  (mapcar
                   (lambda (overlay)
                     (list (overlay-start overlay)
                           (overlay-get overlay 'after-string)))
                   (sort
                    (cl-remove-if-not
                     (lambda (overlay)
                       (overlay-get overlay 'anzu-replace))
                     (overlays-in (point-min) (point-max)))
                    #'anzu--overlay-sort))))
             (anzu--append-replaced-string
              "bar" (current-buffer) 1 (point-max) nil
              (point-max) "foo")
             (list first anzu--last-replace-input))))"##;
    let expect = expect![[
        r#"OK (((1 #(" -> bar" 0 7 (face anzu-replace-to))) (8 #(" -> bar" 0 7 (face anzu-replace-to))) (15 nil)) "bar")"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_query_from_cursor_builds_symbol_regexp_counts_and_signals_without_symbol() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "alpha alphabet alpha")
           (goto-char 3)
           (let (calls)
             (cl-letf (((symbol-function
                         'anzu--count-and-highlight-matched)
                        (lambda (&rest args)
                          (push args calls)
                          (setq anzu--cached-count 2)
                          2))
                       ((symbol-function 'force-mode-line-update)
                        #'ignore))
               (list
                (anzu--query-from-at-cursor
                 (current-buffer) 1 (point-max) (point-max))
                anzu--total-matched
                (nreverse calls)))))
         (with-temp-buffer
           (insert "   ")
           (goto-char 2)
           (condition-case err
               (anzu--query-from-at-cursor
                (current-buffer) 1 (point-max) (point-max))
             (error (list (car err) (cdr err))))))"##;
    let expect = expect![[
        r#"OK (("\\_<alpha\\_>" 2 (((:buffer nil) "\\_<alpha\\_>" 1 21 t 21 t))) (error ("No symbol at cursor!!")))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_query_from_isearch_counts_updates_history_and_returns_exact_input() {
    let elisp_form = r##"(with-temp-buffer
         (insert "red blue red")
         (let ((isearch-string "red")
               (query-replace-from-history-variable
                'query-replace-history)
               (query-replace-history nil)
               calls)
           (cl-letf (((symbol-function 'anzu--count-and-highlight-matched)
                      (lambda (&rest args)
                        (push args calls)
                        (setq anzu--cached-count 2)
                        2))
                     ((symbol-function 'force-mode-line-update)
                      (lambda (&rest args)
                        (push (cons 'force args) calls))))
             (list
              (anzu--query-from-isearch-string
               (current-buffer) 1 (point-max) nil (point-max))
              anzu--total-matched query-replace-history
              (nreverse calls)))))"##;
    let expect = expect![[r#"OK ("red" 2 ("red") (((:buffer nil) "red" 1 13 nil 13 t) (force)))"#]];
    assert_anzu_parity(elisp_form, expect);
}

#[test]
fn anzu_query_from_string_handles_empty_defaults_invalid_regexp_and_history() {
    let elisp_form = r##"(let ((query-replace-from-history-variable
                'query-replace-history)
               (query-replace-history nil)
               calls)
         (cl-letf (((symbol-function 'anzu--read-from-string)
                    (lambda (&rest args)
                      (push args calls)
                      (pop anzu--history)))
                   ((symbol-function 'anzu--query-validate-from-regexp)
                    (lambda (&rest args)
                      (push (cons 'validate args) calls))))
           (list
            (let ((anzu--history '(""))
                  (anzu--query-defaults
                   '(("old" . "new"))))
              (anzu--query-from-string
               "Prompt" 1 9 nil 9))
            (let ((anzu--history '("foo"))
                  (anzu--query-defaults nil)
                  (anzu--total-matched 3))
              (anzu--query-from-string
               "Prompt" 1 9 t 9))
            (let ((anzu--history '("["))
                  (anzu--query-defaults nil))
              (condition-case err
                  (anzu--query-from-string
                   "Prompt" 1 9 t 9)
                (error (list (car err) (cdr err)))))
            query-replace-history
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("old" . "new") "foo" (error ("’[’ is an invalid regular expression")) ("[" "foo") (("Prompt" 1 9 nil 9) ("Prompt" 1 9 t 9) (validate "foo") ("Prompt" 1 9 t 9)))"#
    ]];
    assert_anzu_parity(elisp_form, expect);
}
