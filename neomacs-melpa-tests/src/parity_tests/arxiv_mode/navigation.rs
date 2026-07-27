use expect_test::expect;

use super::assert_arxiv_mode_parity;

#[test]
fn next_entry_moves_by_prefix_updates_overlay_and_refreshes_visible_abstract() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one-a\none-b\none-c\n\n"
                 "two-a\ntwo-b\ntwo-c\n\n"
                 "three-a\nthree-b\nthree-c\n\n")
         (arxiv-mode)
         (let ((arxiv-entry-list '((one) (two) (three)))
               (arxiv-current-entry 0)
               (arxiv-query-results-max 3)
               (arxiv-query-total-results 3)
               (arxiv-abstract-window 'visible)
               calls)
           (cl-letf (((symbol-function 'arxiv-show-abstract)
                      (lambda ()
                        (push arxiv-current-entry calls))))
             (arxiv-next-entry 2)
             (list arxiv-current-entry
                   (point)
                   (line-number-at-pos)
                   (list (overlay-start
                          arxiv-highlight-overlay)
                         (overlay-end
                          arxiv-highlight-overlay))
                   (nreverse calls)))))"##;
    let expect = expect!["OK (2 39 9 (39 64) (2))"];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn next_entry_clamps_at_final_result_and_reports_end_once() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\n\n\n\n" "two\n\n\n\n")
         (arxiv-mode)
         (let ((arxiv-entry-list '((one) (two)))
               (arxiv-current-entry 1)
               (arxiv-query-results-max 2)
               (arxiv-query-total-results 2)
               (arxiv-abstract-window nil)
               messages)
           (cl-letf (((symbol-function 'message)
                      (lambda (format-string &rest args)
                        (push (apply #'format
                                     format-string args)
                              messages))))
             (arxiv-next-entry 5)
             (list arxiv-current-entry
                   (line-number-at-pos)
                   (nreverse messages)))))"##;
    let expect = expect![[r#"OK (1 5 ("end of search results"))"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn next_entry_fetches_another_page_before_positioning_when_results_remain() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\n\n\n\n" "two\n\n\n\n")
         (arxiv-mode)
         (let ((arxiv-entry-list '((one) (two)))
               (arxiv-current-entry 1)
               (arxiv-query-results-max 2)
               (arxiv-query-total-results 4)
               calls)
           (cl-letf (((symbol-function 'arxiv-show-next-page)
                      (lambda ()
                        (push arxiv-current-entry calls)
                        (setq arxiv-entry-list
                              '((one) (two) (three) (four)))
                        (let ((buffer-read-only nil))
                          (goto-char (point-max))
                          (insert "three\n\n\n\n"
                                  "four\n\n\n\n")))))
             (arxiv-next-entry 1)
             (list arxiv-current-entry
                   (line-number-at-pos)
                   (nreverse calls)
                   (length arxiv-entry-list)))))"##;
    let expect = expect!["OK (2 9 (2) 4)"];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn previous_entry_clamps_at_zero_reports_boundary_and_refreshes_live_abstract() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\n\n\n\n" "two\n\n\n\n")
         (arxiv-mode)
         (let ((arxiv-entry-list '((one) (two)))
               (arxiv-current-entry 1)
               (arxiv-abstract-window (selected-window))
               calls
               messages)
           (cl-letf (((symbol-function 'arxiv-show-abstract)
                      (lambda ()
                        (push arxiv-current-entry calls)))
                     ((symbol-function 'message)
                      (lambda (format-string &rest args)
                        (push (apply #'format
                                     format-string args)
                              messages))))
             (arxiv-prev-entry 9)
             (list arxiv-current-entry
                   (line-number-at-pos)
                   (nreverse messages)
                   (nreverse calls)))))"##;
    let expect = expect![[r#"OK (0 1 ("beginning of search results") (0))"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn select_entry_maps_real_cursor_lines_to_the_four_line_record_layout() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one-a\none-b\none-c\n\n"
                 "two-a\ntwo-b\ntwo-c\n\n"
                 "three-a\nthree-b\nthree-c\n\n")
         (arxiv-mode)
         (let ((arxiv-current-entry 0)
               (arxiv-abstract-window nil))
           (goto-char (point-min))
           (forward-line 5)
           (arxiv-select-entry)
           (list arxiv-current-entry
                 (line-number-at-pos)
                 (list (overlay-start arxiv-highlight-overlay)
                       (overlay-end arxiv-highlight-overlay)))))"##;
    let expect = expect!["OK (1 5 (20 39))"];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn space_toggles_current_entry_but_selects_when_cursor_points_elsewhere() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one-a\none-b\none-c\n\n"
                 "two-a\ntwo-b\ntwo-c\n\n")
         (arxiv-mode)
         (let ((arxiv-current-entry 0)
               calls)
           (cl-letf (((symbol-function 'arxiv-toggle-abstract)
                      (lambda () (push :toggle calls)))
                     ((symbol-function 'arxiv-select-entry)
                      (lambda ()
                        (push :select calls)
                        (setq arxiv-current-entry
                              (/ (line-number-at-pos) 4)))))
             (goto-char (point-min))
             (arxiv-SPC)
             (forward-line 4)
             (arxiv-SPC)
             (list arxiv-current-entry
                   (nreverse calls)))))"##;
    let expect = expect!["OK (1 (:toggle :select))"];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn open_current_url_forwards_the_highlighted_entry_to_browser() {
    let elisp_form = r##"(let ((arxiv-entry-list
                '(((url . "https://arxiv.org/abs/one"))
                  ((url . "https://arxiv.org/abs/two"))))
               (arxiv-current-entry 1)
               calls)
         (cl-letf (((symbol-function 'browse-url)
                    (lambda (url &rest args)
                      (push (cons url args) calls)
                      'opened)))
           (list (arxiv-open-current-url)
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (opened (("https://arxiv.org/abs/two")))"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn click_selection_sets_mouse_point_selects_record_and_displays_abstract() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\n\n\n\n" "two\n\n\n\n")
         (arxiv-mode)
         (let ((arxiv-current-entry 0)
               calls)
           (cl-letf (((symbol-function 'mouse-set-point)
                      (lambda (event)
                        (push (list :mouse event) calls)
                        (goto-char (point-min))
                        (forward-line 4)))
                     ((symbol-function 'arxiv-show-abstract)
                      (lambda ()
                        (push (list :show arxiv-current-entry)
                              calls))))
             (arxiv-click-select-entry 'fixture-click)
             (list arxiv-current-entry
                   (line-number-at-pos)
                   (nreverse calls)))))"##;
    let expect = expect!["OK (1 5 ((:mouse fixture-click) (:show 1)))"];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn toggle_abstract_opens_when_absent_and_closes_a_live_window() {
    let elisp_form = r##"(let ((arxiv-abstract-window nil)
               calls)
         (cl-letf (((symbol-function 'arxiv-show-abstract)
                    (lambda ()
                      (push :show calls)
                      (setq arxiv-abstract-window
                            (selected-window))))
                   ((symbol-function 'delete-window)
                    (lambda (&optional window)
                      (push (list :delete (windowp window))
                            calls))))
           (arxiv-toggle-abstract)
           (arxiv-toggle-abstract)
           (list arxiv-abstract-window
                 (nreverse calls))))"##;
    let expect = expect!["OK (nil (:show (:delete nil)))"];
    assert_arxiv_mode_parity(elisp_form, expect);
}
