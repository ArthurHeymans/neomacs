use expect_test::expect;

use super::assert_abridge_diff_parity;

#[test]
fn abridge_diff_make_invisible_is_a_noop_at_or_below_the_size_threshold() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 5))
               (with-temp-buffer
                 (insert "0123456789")
                 (list
                  (abridge-diff-make-invisible 1 6)
                  (get-text-property 1 'invisible)
                  (abridge-diff-make-invisible 1 7)
                  (mapcar
                   (lambda (position)
                     (get-text-property position 'invisible))
                   (number-sequence 1 10)))))"##;
    let expect = expect!["OK (nil nil nil (nil nil nil nil nil nil nil nil nil nil))"];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_make_invisible_without_refinement_keeps_the_requested_word_prefix() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 2)
                    (abridge-diff-no-change-line-words 3))
               (with-temp-buffer
                 (insert
                  "+one two three four five six\n"
                  "one two three four five six")
                 (let ((second-line
                        (save-excursion
                          (goto-char (point-min))
                          (forward-line 1)
                          (point))))
                   (abridge-diff-make-invisible
                    (point-min)
                    (1- second-line))
                   (abridge-diff-make-invisible
                    second-line
                    (point-max)))
                 (list
                  (buffer-string)
                  (let (runs
                        (position (point-min)))
                    (while (< position (point-max))
                      (let* ((value
                              (get-text-property
                               position 'invisible))
                             (next
                              (next-single-property-change
                               position 'invisible nil
                               (point-max))))
                        (push
                         (list position next value)
                         runs)
                        (setq position next)))
                    (nreverse runs)))))"##;
    let expect = expect![[
        r#"OK (#("+one two three four five six\none two three four five six" 14 28 (invisible abridge-diff-invisible) 42 56 (invisible abridge-diff-invisible)) ((1 15 nil) (15 29 abridge-diff-invisible) (29 43 nil) (43 57 abridge-diff-invisible)))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_make_invisible_protects_refined_words_and_first_line_words() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 2)
                    (abridge-diff-word-buffer 1)
                    (abridge-diff-first-words-preserve 2))
               (with-temp-buffer
                 (insert
                  "-zero one two three four five six seven eight nine")
                 (let ((first-start
                        (progn
                          (goto-char (point-min))
                          (search-forward "four")
                          (match-beginning 0)))
                       (first-end (match-end 0))
                       second-start
                       second-end)
                   (search-forward "seven")
                   (setq second-start (match-beginning 0)
                         second-end (match-end 0))
                   (let ((first
                          (make-overlay first-start first-end))
                         (second
                          (make-overlay second-start second-end)))
                     (overlay-put first 'diff-mode 'fine)
                     (overlay-put second 'diff-mode 'fine)
                     (abridge-diff-make-invisible
                      (point-min)
                      (point-max))))
                 (list
                  (buffer-string)
                  (mapcar
                   (lambda (word)
                     (goto-char (point-min))
                     (search-forward word)
                     (list
                      word
                      (get-text-property
                       (match-beginning 0)
                       'invisible)))
                   '("zero" "one" "two" "three" "four"
                     "five" "six" "seven" "eight" "nine")))))"##;
    let expect = expect![[
        r#"OK (#("-zero one two three four five six seven eight nine" 9 14 (invisible abridge-diff-invisible) 45 50 (invisible abridge-diff-invisible)) (("zero" nil) ("one" nil) ("two" abridge-diff-invisible) ("three" nil) ("four" nil) ("five" nil) ("six" nil) ("seven" nil) ("eight" nil) ("nine" abridge-diff-invisible)))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_make_invisible_merges_nearby_refined_context_before_hiding() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 4)
                    (abridge-diff-word-buffer 0)
                    (abridge-diff-first-words-preserve 1))
               (with-temp-buffer
                 (insert "aa bb cc dd ee ff gg hh ii jj")
                 (dolist (word '("dd" "ff" "ii"))
                   (goto-char (point-min))
                   (search-forward word)
                   (let ((overlay
                          (make-overlay
                           (match-beginning 0)
                           (match-end 0))))
                     (overlay-put overlay 'diff-mode 'fine)))
                 (abridge-diff-make-invisible
                  (point-min)
                  (point-max))
                 (let (visible hidden)
                   (dolist
                       (word
                        '("aa" "bb" "cc" "dd" "ee"
                          "ff" "gg" "hh" "ii" "jj"))
                     (goto-char (point-min))
                     (search-forward word)
                     (if
                         (get-text-property
                          (match-beginning 0)
                          'invisible)
                         (push word hidden)
                       (push word visible)))
                   (list
                    (nreverse visible)
                    (nreverse hidden)))))"##;
    let expect = expect![[r#"OK (("aa" "dd" "ee" "ff" "ii" "jj") ("bb" "cc" "gg" "hh"))"#]];

    assert_abridge_diff_parity(elisp_form, expect);
}

#[test]
fn abridge_diff_make_invisible_ignores_overlays_without_the_exact_fine_marker() {
    let elisp_form = r##"(let ((abridge-diff-invisible-min 1)
                    (abridge-diff-no-change-line-words 1))
               (with-temp-buffer
                 (insert "one two three four")
                 (dolist (spec
                          '(("two" coarse)
                            ("three" "fine")))
                   (goto-char (point-min))
                   (search-forward (car spec))
                   (let ((overlay
                          (make-overlay
                           (match-beginning 0)
                           (match-end 0))))
                     (overlay-put
                      overlay 'diff-mode (cadr spec))))
                 (abridge-diff-make-invisible
                  (point-min)
                  (point-max))
                 (mapcar
                  (lambda (word)
                    (goto-char (point-min))
                    (search-forward word)
                    (list
                     word
                     (get-text-property
                      (match-beginning 0)
                      'invisible)))
                  '("one" "two" "three" "four"))))"##;
    let expect = expect![[
        r#"OK (("one" nil) ("two" abridge-diff-invisible) ("three" abridge-diff-invisible) ("four" abridge-diff-invisible))"#
    ]];

    assert_abridge_diff_parity(elisp_form, expect);
}
