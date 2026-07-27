use expect_test::expect;

use super::assert_aozora_view_parity;

#[test]
fn bookmark_records_the_rendered_source_line_and_updates_only_that_book_key() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "first\nsecond\nthird")
                     (put-text-property
                      1 6
                      'line-number 10)
                     (put-text-property
                      7 13
                      'line-number 20)
                     (put-text-property
                      14 19
                      'line-number 30)
                     (setq
                      major-mode
                      'aozora-view-mode
                      aozora-view-text-file
                      "/library/book.txt"
                      aozora-view-bookmarks
                      '("/library/other.txt"
                        99))
                     (goto-char 11)
                     (let ((first-message
                            (aozora-view-bookmark
                             '(4))))
                       (goto-char 16)
                       (let ((second-message
                              (aozora-view-bookmark
                               nil)))
                         (list
                          first-message
                          second-message
                          aozora-view-bookmarks
                          (lax-plist-get
                           aozora-view-bookmarks
                           "/library/book.txt")
                          (lax-plist-get
                           aozora-view-bookmarks
                           "/library/other.txt")))))"##;
    let expect = expect![[
        r#"OK ("Bookmarked!" "Bookmarked!" ("/library/other.txt" 99 "/library/book.txt" 30) 30 99)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn restore_bookmark_moves_to_the_matching_source_line_or_falls_back_to_start() {
    let elisp_form = r##"(mapcar
                      (lambda (saved)
                        (with-temp-buffer
                          (insert
                           "ruby row\nfirst\nruby row\nsecond")
                          (put-text-property
                           10 15
                           'line-number 1)
                          (put-text-property
                           25 31
                           'line-number 2)
                          (setq
                           major-mode
                           'aozora-view-mode
                           aozora-view-text-file
                           "/library/book.txt"
                           aozora-view-bookmarks
                           (and
                            saved
                            (list
                             "/library/book.txt"
                             saved)))
                          (goto-char
                           (point-max))
                          (aozora-view-restore-bookmark)
                          (list
                           (point)
                           (line-number-at-pos)
                           (thing-at-point
                            'line t))))
                      '(2 7 nil))"##;
    let expect = expect![[r#"OK ((25 4 "second") (1 1 "ruby row\n") (1 1 "ruby row\n"))"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn bookmark_and_restore_are_noops_outside_aozora_view_mode() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "text")
                     (setq
                      major-mode
                      'text-mode
                      aozora-view-text-file
                      "/library/book.txt"
                      aozora-view-bookmarks
                      '("/library/book.txt"
                        8))
                     (goto-char
                      (point-max))
                     (list
                      (aozora-view-bookmark
                       nil)
                      (aozora-view-restore-bookmark)
                      (point)
                      aozora-view-bookmarks))"##;
    let expect = expect![[r#"OK (nil nil 5 ("/library/book.txt" 8))"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn suspend_closes_a_view_window_or_switches_buffers_before_burying() {
    let elisp_form = r##"(mapcar
                      (lambda (windows)
                        (with-temp-buffer
                          (let ((events nil))
                            (cl-letf
                                (((symbol-function
                                   'count-windows)
                                  (lambda
                                    (&optional _minibuffer _all-frames)
                                    windows))
                                 ((symbol-function
                                   'get-buffer-window)
                                  (lambda
                                    (&optional buffer _all-frames)
                                    (push
                                     (list
                                      'lookup
                                      (eq
                                       buffer
                                       (current-buffer)))
                                     events)
                                    'view-window))
                                 ((symbol-function
                                   'delete-window)
                                  (lambda
                                    (&optional window)
                                    (push
                                     (list
                                      'delete
                                      window)
                                     events)))
                                 ((symbol-function
                                   'other-buffer)
                                  (lambda
                                    (&optional _buffer _visible-ok _frame)
                                    'other-buffer))
                                 ((symbol-function
                                   'switch-to-buffer)
                                  (lambda
                                    (buffer-or-name
                                     &optional norecord force-same-window)
                                    (push
                                     (list
                                      'switch
                                      buffer-or-name
                                      norecord
                                      force-same-window)
                                     events)
                                    'switched))
                                 ((symbol-function
                                   'bury-buffer)
                                  (lambda
                                    (&optional buffer-or-name)
                                    (push
                                     (list
                                      'bury
                                      (or
                                       buffer-or-name
                                       'current))
                                     events)
                                    'buried)))
                              (list
                               (aozora-view-suspend)
                               (nreverse events))))))
                      '(2 1))"##;
    let expect = expect![
        "OK ((buried ((lookup t) (delete view-window) (bury (:buffer nil)))) (buried ((switch other-buffer nil nil) (bury (:buffer nil)))))"
    ];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn traditional_conversion_requires_both_view_mode_and_ivs_support() {
    let elisp_form = r##"(mapcar
                      (lambda (scenario)
                        (with-temp-buffer
                          (insert "旧字体")
                          (set-buffer-modified-p t)
                          (setq major-mode
                                (car scenario))
                          (let ((events nil))
                            (cl-letf
                                (((symbol-function
                                   'require)
                                  (lambda
                                    (feature
                                     &optional _filename _noerror)
                                    (push
                                     (list
                                      'require
                                      feature)
                                     events)
                                    (cadr scenario)))
                                 ((symbol-function
                                   'ivs-aj1-trad-region)
                                  (lambda
                                    (start end)
                                    (push
                                     (list
                                      'convert
                                      (buffer-substring
                                       start end))
                                     events)
                                    (let ((inhibit-read-only
                                           t))
                                      (goto-char start)
                                      (delete-region
                                       start end)
                                      (insert "舊字體")))))
                              (list
                               (aozora-view-traditional)
                               (buffer-string)
                               (buffer-modified-p)
                               (nreverse events))))))
                      '((text-mode t)
                        (aozora-view-mode nil)
                        (aozora-view-mode t)))"##;
    let expect = expect![[
        r#"OK (("Not Aozora-View mode!" "旧字体" t nil) ("Not Aozora-View mode!" "旧字体" t ((require ivs-aj1))) (nil "舊字體" nil ((require ivs-aj1) (convert "旧字体"))))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn redraw_rebuilds_content_reenters_mode_restores_file_and_then_navigates() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "old rendered text")
                     (setq
                      major-mode
                      'aozora-view-mode
                      aozora-view-text-file
                      "/library/book.txt"
                      aozora-view-text-buffer
                      'source-buffer)
                     (set-buffer-modified-p t)
                     (let ((events nil))
                       (cl-letf
                           (((symbol-function
                              'aozora-view-draw)
                             (lambda
                               (buffer file)
                               (push
                                (list
                                 'draw
                                 buffer
                                 file)
                                events)
                               (let ((inhibit-read-only
                                      t))
                                 (erase-buffer)
                                 (insert
                                  "new rendered text"))
                               (setq
                                aozora-view-text-file
                                "draw-mutated.txt")))
                            ((symbol-function
                              'aozora-view-mode)
                             (lambda ()
                               (push
                                'mode
                                events)
                               (setq major-mode
                                     'aozora-view-mode)))
                            ((symbol-function
                              'aozora-view-restore-bookmark)
                             (lambda ()
                               (push
                                (list
                                 'restore
                                 aozora-view-text-file)
                                events)
                               'restored)))
                         (list
                          (aozora-view-redraw)
                          (buffer-string)
                          (buffer-modified-p)
                          aozora-view-text-file
                          (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (restored "new rendered text" nil "/library/book.txt" ((draw source-buffer "/library/book.txt") mode (restore "/library/book.txt")))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn redraw_and_traditional_report_the_same_mode_error_outside_the_viewer() {
    let elisp_form = r##"(with-temp-buffer
                     (setq major-mode
                           'fundamental-mode)
                     (let ((messages nil))
                       (cl-letf
                           (((symbol-function
                              'message)
                             (lambda
                               (format-string
                                &rest arguments)
                               (let ((text
                                      (apply
                                       #'format
                                       format-string
                                       arguments)))
                                 (push text messages)
                                 text))))
                         (list
                          (aozora-view-redraw)
                          (aozora-view-traditional)
                          (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK ("Not Aozora-View mode!" "Not Aozora-View mode!" ("Not Aozora-View mode!" "Not Aozora-View mode!"))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}
