use expect_test::expect;

use super::assert_arxiv_mode_parity;

#[test]
fn read_new_computes_monday_submission_window_sorts_primary_category_and_populates() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arxiv-query)
                    (lambda (category start end
                             &optional offset ascending)
                      (push (list :query category start end
                                  offset ascending)
                            calls)
                      '(((id . "cross")
                         (categories . ("math.NT" "cs.LG")))
                        ((id . "main")
                         (categories . ("cs.LG"))))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda ()
                      (push (list :populate
                                  (mapcar
                                   (lambda (entry)
                                     (alist-get 'id entry))
                                   arxiv-entry-list))
                            calls))))
           (arxiv-read-new
            "cs.LG"
            (encode-time 0 30 21 8 1 2024
                         nil -18000))
           (list arxiv-query-info
                 arxiv-query-data-list
                 arxiv-mode-entry-function
                 (mapcar (lambda (entry)
                           (alist-get 'id entry))
                         arxiv-entry-list)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (" Showing new submissions in cs.LG from 20240105(Fri) to 20240108(Mon)." ((date-start . "202401051900") (date-end . "202401081900") (category . "cs.LG")) arxiv-read-new ("main" "cross") ((:query "cs.LG" "202401051900" "202401081900" nil t) (:populate ("main" "cross"))))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn read_new_before_announcement_uses_previous_available_submission_day() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arxiv-query)
                    (lambda (&rest args)
                      (push args calls)
                      '(((id . "one")
                         (categories . ("hep-th"))))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda () (push 'populate calls))))
           (arxiv-read-new
            "hep-th"
            (encode-time 0 0 10 9 1 2024
                         nil -18000))
           (list arxiv-query-info
                 arxiv-query-data-list
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (" Showing new submissions in hep-th from 20240105(Fri) to 20240108(Mon)." ((date-start . "202401051900") (date-end . "202401081900") (category . "hep-th")) (("hep-th" "202401051900" "202401081900" nil t) populate))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn read_recent_builds_fixed_week_window_and_populates_results() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'current-time)
                    (lambda ()
                      (encode-time 0 0 12 15 3 2024 t)))
                   ((symbol-function 'completing-read)
                    (lambda (&rest _args) "math.NT"))
                   ((symbol-function 'arxiv-query)
                    (lambda (&rest args)
                      (push (cons :query args) calls)
                      '(((id . "recent")))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda () (push :populate calls))))
           (arxiv-read-recent)
           (list arxiv-query-info
                 arxiv-query-data-list
                 arxiv-mode-entry-function
                 arxiv-entry-list
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (" Showing recent submissions in math.NT in the past week (20240308 to 20240315)." ((date-start . "202403080000") (date-end . "202403150000") (category . "math.NT")) arxiv-read-recent (((id . "recent"))) ((:query "math.NT" "202403080000" "202403150000") :populate))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn read_author_with_explicit_author_searches_all_categories_and_sets_sorting() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arxiv-query-general)
                    (lambda (&optional start)
                      (push (list :query start) calls)
                      '(((id . "author-result")))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda () (push :populate calls))))
           (arxiv-read-author "Ada Lovelace")
           (list arxiv-query-data-list
                 arxiv-query-info
                 arxiv-order-info
                 arxiv-query-sorting
                 arxiv-mode-entry-function
                 arxiv-entry-list
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((author t "Ada Lovelace")) "au:Ada Lovelace" "Submission date (newest first)" (:sortby submittedDate :sortorder descending) arxiv-read-author (((id . "author-result"))) ((:query nil) :populate))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn interactive_read_author_combines_prompted_author_and_category() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'read-string)
                    (lambda (&rest _args) "Grace Hopper"))
                   ((symbol-function 'completing-read)
                    (lambda (&rest _args) "cs.SE"))
                   ((symbol-function 'arxiv-query-general)
                    (lambda (&optional start)
                      (push (list :query start) calls)
                      '(((id . "combined")))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda () (push :populate calls))))
           (arxiv-read-author)
           (list arxiv-query-data-list
                 arxiv-query-info
                 arxiv-query-sorting
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((author t "Grace Hopper") (category t "cs.SE")) "author:Grace Hopper+category:cs.SE" (:sortby submittedDate :sortorder descending) ((:query nil) :populate))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn simple_search_handles_blank_and_practical_conditions_without_network() {
    let elisp_form = r##"(let ((answers '("   " "\"editor parity\" runtime"))
               calls
               messages)
         (cl-letf (((symbol-function 'read-string)
                    (lambda (&rest _args)
                      (prog1 (car answers)
                        (setq answers (cdr answers)))))
                   ((symbol-function 'arxiv-query-general)
                    (lambda (&optional start)
                      (push (list :query start) calls)
                      '(((id . "search-result")))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda () (push :populate calls)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (arxiv-search)
           (let ((after-blank
                  (list arxiv-query-data-list
                        (nreverse messages)
                        calls)))
             (setq messages nil)
             (arxiv-search)
             (list after-blank
                   arxiv-query-data-list
                   arxiv-query-info
                   arxiv-order-info
                   arxiv-query-sorting
                   arxiv-mode-entry-function
                   arxiv-entry-list
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((nil ("exit with blank search condition.") nil) ((all t "\"editor parity\" runtime")) "all:\"editor parity\" runtime" "Default" nil arxiv-search (((id . "search-result"))) ((:query nil) :populate))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn complex_search_resets_prior_query_state_and_opens_search_menu() {
    let elisp_form = r##"(let ((arxiv-query-data-list '((stale t "x")))
               (arxiv-query-info "stale")
               (arxiv-query-sorting '(:stale t))
               (arxiv-order-info "stale")
               calls)
         (cl-letf (((symbol-function 'arxiv-search-menu/body)
                    (lambda ()
                      (push (list arxiv-query-data-list
                                  arxiv-query-info
                                  arxiv-query-sorting
                                  arxiv-order-info)
                            calls)
                      'menu)))
           (list (arxiv-complex-search)
                 arxiv-query-data-list
                 arxiv-query-info
                 arxiv-query-sorting
                 arxiv-order-info
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (menu nil "" nil "Default" ((nil "" nil "Default")))"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn refine_search_opens_menu_only_for_search_entry_modes() {
    let elisp_form = r##"(let (calls messages)
         (cl-letf (((symbol-function 'arxiv-search-menu/body)
                    (lambda () (push :menu calls)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (dolist (mode '(arxiv-read-new arxiv-search
                           arxiv-complex-search arxiv-read-author))
             (let ((arxiv-mode-entry-function mode))
               (arxiv-refine-search)))
           (list (nreverse calls)
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ((:menu :menu) ("Refining search is only available in arxiv-search or arxiv-complex-search." "refine search condition: " "refine search condition: " "Refining search is only available in arxiv-search or arxiv-complex-search."))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn query_data_update_builds_inclusive_and_exclusive_conditions_and_rejects_exclusion_first() {
    let elisp_form = r##"(let ((answers '("Ada Lovelace"
                          "20200101" "20241231"
                          "cs.LG"))
               (arxiv-query-data-list nil)
               (arxiv-query-info "")
               calls messages)
         (cl-letf (((symbol-function 'read-string)
                    (lambda (&rest _args)
                      (prog1 (car answers)
                        (setq answers (cdr answers)))))
                   ((symbol-function 'completing-read)
                    (lambda (&rest _args)
                      (prog1 (car answers)
                        (setq answers (cdr answers)))))
                   ((symbol-function 'arxiv-search-menu/body)
                    (lambda () (push :menu calls)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (arxiv-query-data-update 'comment nil)
           (arxiv-query-data-update 'author t)
           (arxiv-query-data-update 'time nil)
           (arxiv-query-data-update 'category t)
           (list arxiv-query-data-list
                 arxiv-query-info
                 (nreverse calls)
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (((category t "cs.LG") (time nil "[202001010000+TO+202412310000]") (author t "Ada Lovelace")) "+author:Ada Lovelace-time:20200101-20241231+category:cs.LG" (:menu :menu :menu :menu) ("Only inclusive searching is allowed as the first keyword."))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn query_order_update_maps_every_menu_label_to_api_sorting() {
    let elisp_form = r##"(let ((answers
                '("Relevance"
                  "Announcement date (newest first)"
                  "Announcement date (oldest first)"
                  "Submission date (newest first)"
                  "Submission date (oldest first)"))
               snapshots)
         (cl-letf (((symbol-function 'completing-read)
                    (lambda (&rest _args)
                      (prog1 (car answers)
                        (setq answers (cdr answers)))))
                   ((symbol-function 'arxiv-search-menu/body)
                    (lambda ()
                      (push (list arxiv-order-info
                                  arxiv-query-sorting)
                            snapshots))))
           (dotimes (_ 5)
             (arxiv-query-order-update))
           (nreverse snapshots)))"##;
    let expect = expect![[
        r#"OK (("Relevance" (:sortby relevance :sortorder descending)) ("Announcement date (newest first)" (:sortby lastUpdatedDate :sortorder descending)) ("Announcement date (oldest first)" (:sortby lastUpdatedDate :sortorder ascending)) ("Submission date (newest first)" (:sortby submittedDate :sortorder descending)) ("Submission date (oldest first)" (:sortby submittedDate :sortorder ascending)))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn perform_search_restores_user_condition_order_queries_and_populates() {
    let elisp_form = r##"(let ((arxiv-query-info
                "+author:Ada-category:cs.LG")
               (arxiv-query-data-list
                '((category t "cs.LG")
                  (author t "Ada")))
               calls)
         (cl-letf (((symbol-function 'arxiv-query-general)
                    (lambda (&optional start)
                      (push (list :query start
                                  arxiv-query-data-list)
                            calls)
                      '(((id . "result")))))
                   ((symbol-function 'arxiv-populate-page)
                    (lambda () (push :populate calls))))
           (arxiv-hydra-perform-search)
           (list arxiv-query-info
                 arxiv-query-data-list
                 arxiv-entry-list
                 arxiv-mode-entry-function
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("author:Ada-category:cs.LG" #1=((author t "Ada") (category t "cs.LG")) (((id . "result"))) arxiv-complex-search ((:query nil #1#) :populate))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn perform_search_with_no_conditions_reports_quit_and_does_not_query() {
    let elisp_form = r##"(let ((arxiv-query-data-list nil)
               calls messages)
         (cl-letf (((symbol-function 'arxiv-query-general)
                    (lambda (&optional start)
                      (push (list :query start) calls)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (list (arxiv-hydra-perform-search)
                 calls
                 (nreverse messages))))"##;
    let expect = expect![[r#"OK (#1=("quit with blank search conditions") nil #1#)"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn show_next_page_dispatches_to_daily_recent_and_general_query_paths() {
    let elisp_form = r##"(let ((base
                '(((id . "old-1")) ((id . "old-2"))))
               calls results)
         (dolist (mode '(arxiv-read-new arxiv-read-recent
                         arxiv-search))
           (let ((arxiv-entry-list (copy-tree base))
                 (arxiv-current-entry 1)
                 (arxiv-entries-per-fetch 2)
                 (arxiv-query-total-results 3)
                 (arxiv-query-results-min 1)
                 (arxiv-query-results-max 2)
                 (arxiv-query-data-list
                  '((date-start . "202401010000")
                    (date-end . "202401020000")
                    (category . "cs.LG")))
                 (arxiv-mode-entry-function mode))
             (with-temp-buffer
               (setq arxiv-buffer (current-buffer))
               (cl-letf (((symbol-function 'arxiv-query)
                          (lambda (&rest args)
                            (push (cons :date args) calls)
                            '(((id . "new")))))
                         ((symbol-function 'arxiv-query-general)
                          (lambda (&optional start)
                            (push (list :general start) calls)
                            '(((id . "new")))))
                         ((symbol-function 'arxiv-fill-page)
                          (lambda (&optional min max)
                            (push (list :fill min max) calls))))
                 (arxiv-show-next-page)
                 (push (list mode
                             (mapcar
                              (lambda (entry)
                                (alist-get 'id entry))
                              arxiv-entry-list)
                             buffer-read-only)
                       results)))))
         (list (nreverse results)
               (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (((arxiv-read-new ("old-1" "old-2" "new") t) (arxiv-read-recent ("old-1" "old-2" "new") t) (arxiv-search ("old-1" "old-2" "new") t)) ((:date "cs.LG" "202401010000" "202401020000" 2 t) (:fill 2 nil) (:date "cs.LG" "202401010000" "202401020000" 2 nil) (:fill 2 nil) (:general 2) (:fill 2 nil)))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn exit_closes_live_abstract_update_and_bibtex_buffers_and_clears_state() {
    let elisp_form = r##"(let* ((update (get-buffer-create "*arXiv-update*"))
                (abstract (get-buffer-create "*arXiv-abstract*"))
                (arxiv-abstract-window (selected-window))
                (arxiv-abstract-buffer abstract)
                (arxiv-frame 'frame-marker)
                calls)
         (cl-letf (((symbol-function 'quit-restore-window)
                    (lambda (window action)
                      (push (list :quit
                                  (if (windowp window)
                                      :window
                                    window)
                                  action)
                            calls)))
                   ((symbol-function 'get-buffer-window)
                    (lambda (buffer &rest _)
                      (push (list :lookup buffer) calls)
                      'update-window)))
           (arxiv-exit)
           (unwind-protect
               (list arxiv-abstract-window
                     arxiv-abstract-buffer
                     arxiv-frame
                     (buffer-live-p abstract)
                     (buffer-live-p update)
                     (nreverse calls))
             (when (buffer-live-p update)
               (kill-buffer update))
             (when (buffer-live-p abstract)
               (kill-buffer abstract)))))"##;
    let expect = expect![[
        r#"OK (nil nil nil nil t ((:quit :window kill) (:lookup "*arXiv-update*") (:quit update-window kill)))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}
