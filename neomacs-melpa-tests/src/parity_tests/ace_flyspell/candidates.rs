use expect_test::expect;

use super::{assert_ace_flyspell_parity, assert_ace_flyspell_signal_parity};

#[test]
fn ace_flyspell_overlay_predicate_stops_at_the_first_flyspell_overlay() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'flyspell-overlay-p)
                     (lambda (overlay)
                       (push
                        overlay
                        calls)
                       (eq
                        overlay
                        'hit))))
                 (list
                  (ace-flyspell--has-flyspell-overlay-p
                   '(miss hit ignored))
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (t (miss hit))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_overlay_predicate_handles_empty_atoms_and_improper_tails() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'flyspell-overlay-p)
                     (lambda (overlay)
                       (push
                        overlay
                        calls)
                       nil)))
                 (list
                  (ace-flyspell--has-flyspell-overlay-p
                   nil)
                  (ace-flyspell--has-flyspell-overlay-p
                   'atom)
                  (ace-flyspell--has-flyspell-overlay-p
                   '(one two . ignored-tail))
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (nil nil nil (one two))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_overlay_predicate_normalizes_truthy_matches_to_t() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'flyspell-overlay-p)
                 (lambda (_overlay)
                   'truthy)))
               (list
                (ace-flyspell--has-flyspell-overlay-p
                 '(overlay))
                (eq
                 (ace-flyspell--has-flyspell-overlay-p
                  '(overlay))
                 t)))"##;
    let expect = expect!["OK (t t)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_filters_visible_words_and_restores_point_and_restriction() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "bad good ugly")
               (goto-char
                6)
               (let ((words
                      '(("bad" 1 3)
                        ("good" 5 8)
                        ("ugly" 10 13)
                        nil))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'window-start)
                       (lambda ()
                         (push
                          'window-start
                          calls)
                         1))
                      ((symbol-function
                        'selected-window)
                       (lambda ()
                         (push
                          'selected-window
                          calls)
                         'selected))
                      ((symbol-function
                        'window-end)
                       (lambda (window update)
                         (push
                          (list
                           'window-end
                           window
                           update)
                          calls)
                         (point-max)))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional following)
                         (push
                          (list
                           'word
                           following
                           (point))
                          calls)
                         (pop
                          words)))
                      ((symbol-function
                        'overlays-at)
                       (lambda (position)
                         (push
                          (list
                           'overlays-at
                           position)
                          calls)
                         (pcase
                             position
                           (1 '(plain))
                           (5 '(spelling))
                           (10 '(other spelling))
                           (_ nil))))
                      ((symbol-function
                        'flyspell-overlay-p)
                       (lambda (overlay)
                         (eq
                          overlay
                          'spelling))))
                   (list
                    (ace-flyspell--collect-candidates)
                    (point)
                    (point-min)
                    (point-max)
                    (nreverse
                     calls)))))"##;
    let expect = expect![
        "OK ((5 10) 6 1 14 (window-start selected-window (window-end selected t) (word t 1) (overlays-at 1) (word t 4) (overlays-at 5) (word t 9) (overlays-at 10)))"
    ];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_preserves_source_order_without_duplicates() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "one two three")
               (let ((words
                      '(("one" 1 3)
                        ("two" 5 7)
                        ("three" 9 13)
                        nil)))
                 (cl-letf
                     (((symbol-function
                        'window-start)
                       (lambda ()
                         1))
                      ((symbol-function
                        'selected-window)
                       (lambda ()
                         'selected))
                      ((symbol-function
                        'window-end)
                       (lambda (_window _update)
                         (point-max)))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional _following)
                         (pop
                          words)))
                      ((symbol-function
                        'overlays-at)
                       (lambda (position)
                         (if
                             (= position 5)
                             '(spelling spelling)
                           '(spelling))))
                      ((symbol-function
                        'flyspell-overlay-p)
                       (lambda (overlay)
                         (eq
                          overlay
                          'spelling))))
                   (ace-flyspell--collect-candidates))))"##;
    let expect = expect!["OK (1 5 9)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_returns_nil_after_one_empty_word_probe() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "visible text")
               (let ((word-calls 0)
                     (overlay-calls 0))
                 (cl-letf
                     (((symbol-function
                        'window-start)
                       (lambda ()
                         1))
                      ((symbol-function
                        'selected-window)
                       (lambda ()
                         'selected))
                      ((symbol-function
                        'window-end)
                       (lambda (_window _update)
                         (point-max)))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional _following)
                         (setq word-calls
                               (1+
                                word-calls))
                         nil))
                      ((symbol-function
                        'overlays-at)
                       (lambda (_position)
                         (setq overlay-calls
                               (1+
                                overlay-calls))
                         nil)))
                   (list
                    (ace-flyspell--collect-candidates)
                    word-calls
                    overlay-calls))))"##;
    let expect = expect!["OK (nil 1 0)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_uses_the_selected_window_end_with_update() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "abcdef")
               (let (arguments)
                 (cl-letf
                     (((symbol-function
                        'window-start)
                       (lambda ()
                         2))
                      ((symbol-function
                        'selected-window)
                       (lambda ()
                         'chosen-window))
                      ((symbol-function
                        'window-end)
                       (lambda (&rest args)
                         (setq arguments
                               args)
                         5))
                      ((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional _following)
                         nil)))
                   (list
                    (ace-flyspell--collect-candidates)
                    arguments
                    (point-min)
                    (point-max)))))"##;
    let expect = expect!["OK (nil (chosen-window t) 1 7)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_rejects_a_word_tuple_without_a_start_position() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "bad")
               (cl-letf
                   (((symbol-function
                      'window-start)
                     (lambda ()
                       1))
                    ((symbol-function
                      'selected-window)
                     (lambda ()
                       'selected))
                    ((symbol-function
                      'window-end)
                     (lambda (_window _update)
                       (point-max)))
                    ((symbol-function
                      'flyspell-get-word)
                     (lambda (&optional _following)
                       '("bad" nil 3))))
                 (ace-flyspell--collect-candidates)))"##;
    let expect = expect!["ERR (wrong-type-argument integer-or-marker-p nil)"];

    assert_ace_flyspell_signal_parity(elisp_form, expect);
}
