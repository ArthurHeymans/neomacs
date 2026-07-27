use expect_test::expect;

use super::assert_arxiv_mode_parity;

#[test]
fn insert_with_face_preserves_text_and_applies_font_lock_face_to_every_character() {
    let elisp_form = r##"(with-temp-buffer
         (arxiv-insert-with-face "Parity λ" 'arxiv-title-face)
         (list (buffer-string)
               (get-text-property 1 'font-lock-face)
               (get-text-property (1- (point-max))
                                  'font-lock-face)
               (next-single-property-change
                1 'font-lock-face nil (point-max))))"##;
    let expect = expect![[
        r#"OK (#("Parity λ" 0 8 (font-lock-face arxiv-title-face)) arxiv-title-face arxiv-title-face 9)"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn fill_page_renders_real_article_titles_authors_dates_and_categories() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((title . "Deterministic Editors")
                   (author . ("Ada Lovelace" "Grace Hopper"))
                   (date . "2024-01-02 03:04:05")
                   (categories . ("cs.SE" "cs.PL")))
                  ((title . "Parity at Scale")
                   (author . ("Lin Test"))
                   (date . "2024-02-03 00:00:00")
                   (categories . ("math.NT")))))
               (arxiv-author-list-maximum 10))
         (with-temp-buffer
           (arxiv-fill-page)
           (list
            (buffer-string)
            (mapcar
             (lambda (position)
               (get-text-property position 'font-lock-face))
             '(2 26 41 56 67 78 92 107)))))"##;
    let expect = expect![[
        r#"OK (#(" Deterministic Editors\n Ada Lovelace, Grace Hopper\n 2024-01-02  [cs.SE] [cs.PL] \n\n Parity at Scale\n Lin Test\n 2024-02-03  [math.NT] \n\n" 0 24 (font-lock-face arxiv-title-face) 24 36 (font-lock-face arxiv-author-face) 38 50 (font-lock-face arxiv-author-face) 50 64 (font-lock-face arxiv-date-face) 64 72 (font-lock-face arxiv-keyword-face) 72 80 (font-lock-face arxiv-keyword-face) 82 100 (font-lock-face arxiv-title-face) 100 108 (font-lock-face arxiv-author-face) 108 122 (font-lock-face arxiv-date-face) 122 132 (font-lock-face arxiv-keyword-face)) (arxiv-title-face arxiv-author-face arxiv-author-face arxiv-date-face arxiv-keyword-face arxiv-keyword-face arxiv-title-face arxiv-author-face))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn fill_page_honors_slice_bounds_and_truncates_long_author_lists() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((title . "Skip")
                   (author . ("Zero Author"))
                   (date . "2024-01-01 00:00")
                   (categories . ("cs.AI")))
                  ((title . "Keep")
                   (author . ("One A" "Two B" "Three C" "Four D"))
                   (date . "2024-01-02 00:00")
                   (categories . ("cs.LG")))
                  ((title . "After")
                   (author . ("Last Author"))
                   (date . "2024-01-03 00:00")
                   (categories . ("stat.ML")))))
               (arxiv-author-list-maximum 2))
         (with-temp-buffer
           (arxiv-fill-page 1 2)
           (buffer-string)))"##;
    let expect = expect![[
        r#"OK #(" Keep\n One A, Two B, et al.\n 2024-01-02  [cs.LG] \n\n" 0 7 (font-lock-face arxiv-title-face) 7 12 (font-lock-face arxiv-author-face) 14 19 (font-lock-face arxiv-author-face) 27 41 (font-lock-face arxiv-date-face) 41 49 (font-lock-face arxiv-keyword-face))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn zero_author_limit_pins_the_upstream_zero_limit_rendering_behavior() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((title . "One")
                   (author . ("Ada Lovelace"))
                   (date . "2024-01-01 00:00")
                   (categories . ("cs.SE")))))
               (arxiv-author-list-maximum 0))
         (with-temp-buffer
           (arxiv-fill-page)
           (list (buffer-string)
                 arxiv-entry-list)))"##;
    let expect = expect![[
        r#"OK (#(" One\n Ada Lovelace, et al.\n 2024-01-01  [cs.SE] \n\n" 0 6 (font-lock-face arxiv-title-face) 6 18 (font-lock-face arxiv-author-face) 26 40 (font-lock-face arxiv-date-face) 40 48 (font-lock-face arxiv-keyword-face)) (((title . "One") (author "Ada Lovelace") (date . "2024-01-01 00:00") (categories "cs.SE"))))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn abstract_page_formats_complete_metadata_buttons_math_and_subjects() {
    let elisp_form = r##"(let ((entry
                '((id . "2401.01234")
                  (title . "Practical Parity")
                  (author . ("Ada Lovelace" "Grace Hopper"))
                  (abstract . "  We prove $x+y=z$ with editors.  ")
                  (url . "https://arxiv.org/abs/2401.01234")
                  (comment . "12 pages")
                  (categories . ("cs.SE" "cs.LG"))
                  (journal . "Journal of Tests")
                  (doi . "10.1000/test")
                  (date . "2024-01-02 03:04:05 ")
                  (updated . "2024-01-03 04:05:06 "))))
         (with-temp-buffer
           (arxiv-format-abstract-page entry)
           (list
            header-line-format
            (buffer-string)
            (let ((position (point-min))
                  button buttons)
              (while (setq button (next-button position))
                (push (list (button-label button)
                            (button-get button 'help-echo)
                            (button-get button 'follow-link))
                      buttons)
                (setq position (button-end button)))
              (nreverse buttons))
            (let ((position
                   (progn
                     (goto-char (point-min))
                     (search-forward "$x+y=z$")
                     (match-beginning 0))))
              (get-text-property position 'font-lock-face)))))"##;
    let expect = expect![[
        r#"OK (" arXiv:2401.01234" #("\nPractical Parity\n\nAda Lovelace, Grace Hopper\n\n    We prove $x+y=z$ with editors.  \n\nComments: 12 pages\nSubjects: Software Engineering (cs.SE); Machine Learning (cs.LG)\nJournal: Journal of Tests\nSubmitted: 2024-01-02 03:04:05 \nUpdated: 2024-01-03 04:05:06 " 0 1 (font-lock-face arxiv-title-face) 17 19 (font-lock-face arxiv-title-face) 47 51 (font-lock-face arxiv-abstract-face) 51 60 (font-lock-face arxiv-abstract-face wrap-prefix #1="    ") 60 67 (font-lock-face arxiv-abstract-math-face wrap-prefix #1#) 67 83 (font-lock-face arxiv-abstract-face wrap-prefix #1#) 83 103 (font-lock-face arxiv-subfield-face) 103 114 (font-lock-face arxiv-subfield-face) 114 135 (font-lock-face (:inherit arxiv-subfield-face :weight semi-bold)) 135 142 (font-lock-face (:inherit arxiv-subfield-face :weight semi-bold)) 142 144 (font-lock-face arxiv-subfield-face) 144 161 (font-lock-face arxiv-subfield-face wrap-prefix "  ") 161 168 (font-lock-face arxiv-subfield-face wrap-prefix "  ") 168 194 (font-lock-face arxiv-subfield-face) 194 226 (font-lock-face arxiv-subfield-face) 226 256 (font-lock-face arxiv-subfield-face)) (("Practical Parity" "Link: https://arxiv.org/abs/2401.01234" t) ("Ada Lovelace" "Look up author: Ada Lovelace" t) ("Grace Hopper" "Look up author: Grace Hopper" t)) arxiv-abstract-math-face)"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn abstract_page_uses_na_for_missing_comment_and_omits_optional_journal_fields() {
    let elisp_form = r##"(let ((entry
                '((id . "2402.00001")
                  (title . "Minimal")
                  (author . ("Single Author"))
                  (abstract . " Plain abstract ")
                  (url . "https://arxiv.org/abs/2402.00001")
                  (categories . ("math.NT"))
                  (date . "2024-02-01 00:00:00 ")
                  (updated . "2024-02-01 00:00:00 "))))
         (with-temp-buffer
           (arxiv-format-abstract-page entry)
           (let ((position (point-min))
                 button
                 (count 0))
             (while (setq button (next-button position))
               (setq count (1+ count)
                     position (button-end button)))
             (list header-line-format
                   (buffer-string)
                   count))))"##;
    let expect = expect![[
        r#"OK (" arXiv:2402.00001" #("\nMinimal\n\nSingle Author\n\n    Plain abstract \n\nComments: N/A\nSubjects: Number Theory (math.NT)\nSubmitted: 2024-02-01 00:00:00 \nUpdated: 2024-02-01 00:00:00 " 0 1 (font-lock-face arxiv-title-face) 8 10 (font-lock-face arxiv-title-face) 25 29 (font-lock-face arxiv-abstract-face) 29 44 (font-lock-face arxiv-abstract-face wrap-prefix "    ") 44 59 (font-lock-face arxiv-subfield-face) 59 70 (font-lock-face arxiv-subfield-face) 70 84 (font-lock-face (:inherit arxiv-subfield-face :weight semi-bold)) 84 93 (font-lock-face (:inherit arxiv-subfield-face :weight semi-bold)) 93 125 (font-lock-face arxiv-subfield-face) 125 155 (font-lock-face arxiv-subfield-face)) 2)"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn headerline_distinguishes_daily_lists_from_sorted_search_results() {
    let elisp_form = r##"(cl-letf (((symbol-function 'window-total-width)
                    (lambda (&optional _window) 72)))
         (let ((arxiv-current-entry 6)
               (arxiv-query-total-results 123)
               (arxiv-query-info "cs.LG from Mon to Tue")
               (arxiv-order-info "Relevance"))
           (list
            (let ((arxiv-mode-entry-function
                   'arxiv-read-new))
              (arxiv-headerline-format))
            (let ((arxiv-mode-entry-function
                   'arxiv-search))
              (arxiv-headerline-format)))))"##;
    let expect = expect![[
        r#"OK (((-65 "cs.LG from Mon to Tue" #(" " 0 1 (display (space :align-to 65))) "7/123")) ((-65 " Search results for cs.LG from Mon to Tue.  Sorted by: Relevance" #(" " 0 1 (display (space :align-to 65))) "7/123")))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn populate_page_with_no_results_emits_one_stable_message_and_creates_no_buffer() {
    let elisp_form = r##"(let ((arxiv-entry-list nil)
               (arxiv-buffer nil)
               messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (list (arxiv-populate-page)
                 (nreverse messages)
                 arxiv-buffer
                 (get-buffer "*arXiv-update*"))))"##;
    let expect = expect![[r#"OK (#1=("No articles matching the search condition.") #1# nil nil)"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn populate_page_builds_a_read_only_mode_buffer_overlay_and_result_message() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((title . "One Result")
                   (author . ("Ada Lovelace"))
                   (date . "2024-01-02 00:00")
                   (categories . ("cs.SE")))))
               (arxiv-query-results-min 1)
               (arxiv-query-results-max 1)
               (arxiv-query-total-results 1)
               (arxiv-pop-up-new-frame nil)
               (arxiv-startup-with-abstract-window nil)
               (arxiv-buffer nil)
               calls)
         (cl-letf (((symbol-function 'switch-to-buffer)
                    (lambda (buffer &rest _)
                      (push (list :switch (buffer-name buffer))
                            calls)
                      buffer))
                   ((symbol-function 'set-window-dedicated-p)
                    (lambda (window flag)
                      (push (list :dedicated window flag)
                            calls)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (list :message
                                  (apply #'format
                                         format-string args))
                            calls))))
           (arxiv-populate-page)
           (unwind-protect
               (with-current-buffer arxiv-buffer
                 (list (buffer-name)
                       major-mode
                       buffer-read-only
                       arxiv-current-entry
                       (replace-regexp-in-string
                        " +$"
                        (lambda (spaces)
                          (make-string (length spaces) ?␠))
                        (buffer-string))
                       (list (overlay-start
                              arxiv-highlight-overlay)
                             (overlay-end
                              arxiv-highlight-overlay))
                       (nreverse calls)))
             (when (buffer-live-p arxiv-buffer)
               (kill-buffer arxiv-buffer)))))"##;
    let expect = expect![[r##"OK ("*arXiv-update*" arxiv-mode t 0 #(" One Result
 Ada Lovelace
 2024-01-02  [cs.SE]␠

" 0 13 (font-lock-face arxiv-title-face) 13 25 (font-lock-face arxiv-author-face) 25 39 (font-lock-face arxiv-date-face) 39 46 (font-lock-face arxiv-keyword-face)) (1 50) ((:message "Showing results 1-1 of 1") (:switch "*arXiv-update*") (:dedicated nil t)))"##]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn show_abstract_reuses_one_buffer_formats_current_entry_and_dedicates_window() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((id . "2401.1")
                   (title . "First")
                   (author . ("Ada Lovelace"))
                   (abstract . "Abstract one")
                   (url . "https://arxiv.org/abs/2401.1")
                   (categories . ("cs.SE"))
                   (date . "2024-01-01 ")
                   (updated . "2024-01-02 "))))
               (arxiv-current-entry 0)
               (arxiv-abstract-buffer nil)
               (arxiv-abstract-window nil)
               calls)
         (cl-letf (((symbol-function 'display-buffer)
                    (lambda (buffer &optional action)
                      (push (list :display buffer action) calls)
                      (selected-window)))
                   ((symbol-function 'set-window-dedicated-p)
                    (lambda (window flag)
                      (push (list :dedicated
                                  (windowp window) flag)
                            calls))))
           (arxiv-show-abstract)
           (arxiv-show-abstract)
           (unwind-protect
               (with-current-buffer arxiv-abstract-buffer
                 (list (buffer-name)
                       major-mode
                       header-line-format
                       (buffer-string)
                       prettify-symbols-mode
                       (nreverse calls)))
             (when (buffer-live-p arxiv-abstract-buffer)
               (kill-buffer arxiv-abstract-buffer)))))"##;
    let expect = expect![[
        r#"OK ("*arXiv-abstract*" arxiv-abstract-mode " arXiv:2401.1" #("\nFirst\n\nAda Lovelace\n\n    Abstract one\n\nComments: N/A\nSubjects: Software Engineering (cs.SE)\nSubmitted: 2024-01-01 \nUpdated: 2024-01-02 " 0 1 (font-lock-face arxiv-title-face) 6 8 (font-lock-face arxiv-title-face) 22 26 (font-lock-face arxiv-abstract-face) 26 38 (font-lock-face arxiv-abstract-face wrap-prefix "    ") 38 53 (font-lock-face arxiv-subfield-face) 53 64 (font-lock-face arxiv-subfield-face) 64 85 (font-lock-face (:inherit arxiv-subfield-face :weight semi-bold)) 85 92 (font-lock-face (:inherit arxiv-subfield-face :weight semi-bold)) 92 115 (font-lock-face arxiv-subfield-face) 115 136 (font-lock-face arxiv-subfield-face)) t ((:display "*arXiv-abstract*" t) (:dedicated t t) (:dedicated t t)))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn customize_forwards_the_exact_group_without_mutating_mode_state() {
    let elisp_form = r##"(let ((arxiv-current-entry 9)
               calls)
         (cl-letf (((symbol-function 'customize-group)
                    (lambda (group)
                      (push group calls)
                      'customized)))
           (list (arxiv-customize)
                 (nreverse calls)
                 arxiv-current-entry)))"##;
    let expect = expect!["OK (customized (arxiv) 9)"];
    assert_arxiv_mode_parity(elisp_form, expect);
}
