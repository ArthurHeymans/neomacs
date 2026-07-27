use expect_test::expect;

use super::assert_artbollocks_mode_parity;

#[test]
fn artbollocks_letter_word_syllable_sentence_counts_cover_realistic_punctuation_unicode_and_numbers()
 {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (text-mode)
             (insert text)
             (list
              text
              (artbollocks-count-letters)
              (artbollocks-count-syllables)
              (artbollocks-count-words)
              (artbollocks-count-sentences))))
         '("Simple words work."
           "One sentence! Two questions? Three."
           "naïve façade résumé."
           "日本語 λ discourse."
           "Numbers 123 and snake_case."
           "A sentence without punctuation"
           ""
           "?!..."
           "YELLOW rhythm MYTH."))"##;
    let expect = expect![[
        r#"OK (("Simple words work." 15 4 3 1) ("One sentence! Two questions? Three." 28 9 5 3) ("naïve façade résumé." 17 6 3 1) ("日本語 λ discourse." 13 3 3 1) ("Numbers 123 and snake_case." 22 7 5 1) ("A sentence without punctuation" 27 9 4 0) ("" 0 0 0 0) ("?!..." 0 0 0 0) ("YELLOW rhythm MYTH." 16 4 3 1))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_metric_counts_honor_explicit_ranges_without_moving_point_or_mark() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (insert
          "Before text. "
          "Selected words are useful. "
          "After text!")
         (goto-char
          (point-min))
         (search-forward
          "Selected")
         (let ((start
                (match-beginning 0)))
           (search-forward
            "useful.")
           (let ((end
                  (point)))
             (goto-char
              (point-max))
             (set-mark
              (point-min))
             (let ((point-before
                    (point))
                   (mark-before
                    (mark)))
               (list
                start
                end
                (buffer-substring-no-properties
                 start
                 end)
                (artbollocks-count-letters
                 start
                 end)
                (artbollocks-count-syllables
                 start
                 end)
                (artbollocks-count-words
                 start
                 end)
                (artbollocks-count-sentences
                 start
                 end)
                point-before
                (point)
                mark-before
                (mark))))))"##;
    let expect = expect![[r#"OK (14 40 "Selected words are useful." 22 9 4 1 52 52 1 1)"#]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_metric_counts_default_to_accessible_narrowed_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (insert
          "Outside one. "
          "Inside two words. "
          "Outside three.")
         (goto-char
          (point-min))
         (search-forward
          "Inside")
         (let ((start
                (match-beginning 0)))
           (search-forward
            "words.")
           (narrow-to-region
            start
            (point))
           (list
            (point-min)
            (point-max)
            (buffer-string)
            (artbollocks-count-letters)
            (artbollocks-count-syllables)
            (artbollocks-count-words)
            (artbollocks-count-sentences)
            (artbollocks-automated-readability-index)
            (artbollocks-flesch-reading-ease)
            (artbollocks-flesch-kinkaid-grade-level))))"##;
    let expect = expect![[
        r#"OK (14 31 "Inside two words." 14 5 3 1 2.0500000000000007 62.789000000000016 5.246666666666666)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_syllable_counter_respects_case_fold_search_and_naive_vowel_run_rules() {
    let elisp_form = r##"(mapcar
         (lambda (fold)
           (with-temp-buffer
             (insert
              "AEIOU queue rhythm SKY naïve co-operate")
             (let ((case-fold-search
                    fold))
               (list
                fold
                (artbollocks-count-syllables)
                (save-excursion
                  (goto-char
                   (point-min))
                  (let (runs)
                    (while
                        (re-search-forward
                         "[aeiouy]+"
                         nil
                         t)
                      (push
                       (match-string-no-properties
                        0)
                       runs))
                    (nreverse runs)))))))
         '(nil t))"##;
    let expect = expect![[
        r#"OK ((nil 9 ("ueue" "y" "a" "e" "o" "o" "e" "a" "e")) (t 11 ("AEIOU" "ueue" "y" "Y" "a" "e" "o" "o" "e" "a" "e")))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_word_counter_fallback_uses_how_many_when_builtin_count_words_is_unavailable() {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (insert
          "alpha beta_gamma 123 λ 日本語")
         (let ((original
                (symbol-function
                 'count-words)))
           (unwind-protect
               (progn
                 (fmakunbound
                  'count-words)
                 (list
                  (fboundp
                   'count-words)
                  (artbollocks-count-words)
                  (artbollocks-count-words
                   7
                   17)))
             (fset
              'count-words
              original))))"##;
    let expect = expect!["OK (nil 6 2)"];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn interactive_optional_region_macro_expansion_and_commands_select_active_region_or_accessible_buffer()
 {
    let elisp_form = r##"(list
         (macroexpand
          '(interactive-optional-region))
         (with-temp-buffer
           (text-mode)
           (insert
            "whole buffer has five words")
           (let ((transient-mark-mode
                  t)
                 messages)
             (cl-letf
                 (((symbol-function
                    'message)
                   (lambda (format-string &rest arguments)
                     (push
                      (apply
                       #'format
                       format-string
                       arguments)
                      messages))))
               (goto-char
                (point-min))
               (search-forward
                "buffer")
               (set-mark
                (match-beginning 0))
               (search-forward
                "five")
               (setq mark-active t)
               (let ((region-result
                      (call-interactively
                       'artbollocks-count-words)))
                 (setq mark-active nil)
                 (let ((buffer-result
                        (call-interactively
                         'artbollocks-count-words)))
                   (list
                    region-result
                    buffer-result
                    (nreverse messages)
                    (point)
                    (mark))))))))"##;
    let expect = expect![[
        r#"OK ((interactive (if (use-region-p) (list (region-beginning) (region-end)) (list (point-min) (point-max)))) (3 5 ("Word count: 3" "Word count: 5") 22 7))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_word_and_sentence_aliases_share_exact_function_cells_and_interactive_contracts() {
    let elisp_form = r##"(list
         (eq
          (symbol-function
           'artbollocks-word-count)
          (symbol-function
           'artbollocks-count-words))
         (eq
          (symbol-function
           'artbollocks-sentence-count)
          (symbol-function
           'artbollocks-count-sentences))
         (help-function-arglist
          'artbollocks-word-count
          t)
         (help-function-arglist
          'artbollocks-sentence-count
          t)
         (interactive-form
          'artbollocks-word-count)
         (interactive-form
          'artbollocks-sentence-count)
         (commandp
          'artbollocks-word-count)
         (commandp
          'artbollocks-sentence-count)
         (symbol-file
          'artbollocks-word-count
          'defun)
         (symbol-file
          'artbollocks-sentence-count
          'defun))"##;
    let expect = expect![[
        r#"OK (nil nil (&optional start end) (&optional start end) (interactive #1=(if (use-region-p) (list (region-beginning) (region-end)) (list (point-min) (point-max)))) (interactive #1#) t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/artbollocks-mode/20251211.1624/home/.emacs.d/elpa/artbollocks-mode-20251211.1624/artbollocks-mode.el" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/artbollocks-mode/20251211.1624/home/.emacs.d/elpa/artbollocks-mode-20251211.1624/artbollocks-mode.el")"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_readability_formulas_compute_practical_documents_and_subranges_to_stable_precision()
{
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (text-mode)
             (insert text)
             (list
              text
              (artbollocks-count-letters)
              (artbollocks-count-syllables)
              (artbollocks-count-words)
              (artbollocks-count-sentences)
              (format
               "%.8f"
               (artbollocks-automated-readability-index))
              (format
               "%.8f"
               (artbollocks-flesch-reading-ease))
              (format
               "%.8f"
               (artbollocks-flesch-kinkaid-grade-level)))))
         '("The cat sat. The dog ran."
           "Art criticism deploys contextual narratives. Readers interrogate representation."
           "This sentence is intentionally longer because readability formulas respond to the relationship between letters, syllables, words, and sentence boundaries."
           "One."
           ""))"##;
    let expect = expect![[
        r#"OK (("The cat sat. The dog ran." 18 6 6 2 "-5.80000000" "119.18900000" "-2.62000000") ("Art criticism deploys contextual narratives. Readers interrogate representation." 71 25 8 2 "22.37125000" "-61.60100000" "22.84500000") ("This sentence is intentionally longer because readability formulas respond to the relationship between letters, syllables, words, and sentence boundaries." 132 46 19 1 "20.79210526" "-17.27205263" "20.38842105") ("One." 3 2 1 1 "-6.80000000" "36.61900000" "8.40000000") ("" 0 0 0 0 "0.00000000" "0.00000000" "0.00000000"))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_readability_formulas_use_exact_helper_call_order_arguments_and_equations() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'artbollocks-count-words)
               (lambda (&optional start end)
                 (push
                  (list :words start end)
                  calls)
                 20))
              ((symbol-function
                'artbollocks-count-letters)
               (lambda (&optional start end)
                 (push
                  (list :letters start end)
                  calls)
                 100))
              ((symbol-function
                'artbollocks-count-sentences)
               (lambda (&optional start end)
                 (push
                  (list :sentences start end)
                  calls)
                 4))
              ((symbol-function
                'artbollocks-count-syllables)
               (lambda (&optional start end)
                 (push
                  (list :syllables start end)
                  calls)
                 30)))
           (list
            (format
             "%.8f"
             (artbollocks-automated-readability-index
              11
              99))
            (prog1
                (nreverse calls)
              (setq calls nil))
            (format
             "%.8f"
             (artbollocks-flesch-reading-ease
              11
              99))
            (prog1
                (nreverse calls)
              (setq calls nil))
            (format
             "%.8f"
             (artbollocks-flesch-kinkaid-grade-level
              11
              99))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("4.62000000" ((:words 11 99) (:letters 11 99) (:sentences 11 99)) "74.85900000" ((:words 11 99) (:sentences 11 99) (:syllables 11 99)) "4.06000000" ((:words 11 99) (:sentences 11 99) (:syllables 11 99)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_readability_formulas_return_float_zero_for_empty_wordless_or_sentence_less_ranges() {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (text-mode)
             (insert text)
             (mapcar
              (lambda (function)
                (let ((value
                       (funcall
                        function)))
                  (list
                   function
                   value
                   (type-of value))))
              '(artbollocks-automated-readability-index
                artbollocks-flesch-reading-ease
                artbollocks-flesch-kinkaid-grade-level))))
         '(""
           "words without sentence punctuation"
           "..."
           "123"))"##;
    let expect = expect![
        "OK (((artbollocks-automated-readability-index 0.0 float) (artbollocks-flesch-reading-ease 0.0 float) (artbollocks-flesch-kinkaid-grade-level 0.0 float)) ((artbollocks-automated-readability-index 0.0 float) (artbollocks-flesch-reading-ease 0.0 float) (artbollocks-flesch-kinkaid-grade-level 0.0 float)) ((artbollocks-automated-readability-index 0.0 float) (artbollocks-flesch-reading-ease 0.0 float) (artbollocks-flesch-kinkaid-grade-level 0.0 float)) ((artbollocks-automated-readability-index 0.0 float) (artbollocks-flesch-reading-ease 0.0 float) (artbollocks-flesch-kinkaid-grade-level 0.0 float)))"
    ];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_readability_commands_forward_region_emit_exact_messages_and_return_message_values() {
    let elisp_form = r##"(let (calls
               messages)
         (cl-letf
             (((symbol-function
                'artbollocks-automated-readability-index)
               (lambda (&optional start end)
                 (push
                  (list :index start end)
                  calls)
                 12.3456))
              ((symbol-function
                'artbollocks-flesch-reading-ease)
               (lambda (&optional start end)
                 (push
                  (list :ease start end)
                  calls)
                 78.9))
              ((symbol-function
                'artbollocks-flesch-kinkaid-grade-level)
               (lambda (&optional start end)
                 (push
                  (list :grade start end)
                  calls)
                 6.75))
              ((symbol-function
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
            (artbollocks-readability-index
             3
             40)
            (artbollocks-reading-ease
             5
             42)
            (artbollocks-grade-level
             7
             44)
            (nreverse calls)
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("Readability index: 12.3456" "Reading ease: 78.9" "Grade level: 6.75" ((:index 3 40) (:ease 5 42) (:grade 7 44)) ("Readability index: 12.3456" "Reading ease: 78.9" "Grade level: 6.75"))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}
