use expect_test::expect;

use super::assert_aozora_view_parity;

#[test]
fn draw_prefers_a_live_source_buffer_removes_cr_and_runs_the_render_pipeline_in_order() {
    let elisp_form = r##"(let ((source
                         (generate-new-buffer
                          " *aozora-live-source*")))
                     (unwind-protect
                         (progn
                           (with-current-buffer source
                             (insert
                              "source\r\ntext"))
                           (with-temp-buffer
                             (insert
                              "stale rendered data")
                             (set-buffer-modified-p t)
                             (let ((events nil))
                               (cl-letf
                                   (((symbol-function
                                      'aozora-view-arrange-replace)
                                     (lambda ()
                                       (push
                                        (list
                                         'replace
                                         (buffer-string))
                                        events)
                                       (goto-char
                                        (point-max))
                                       (insert
                                        "|replaced")))
                                    ((symbol-function
                                      'aozora-view-arrange-fill-lines)
                                     (lambda ()
                                       (push
                                        (list
                                         'layout
                                         (buffer-string))
                                        events)
                                       (goto-char
                                        (point-max))
                                       (insert
                                        "|laid-out")))
                                    ((symbol-function
                                      'aozora-view-save-cache)
                                     (lambda (file)
                                       (push
                                        (list
                                         'cache
                                         file
                                         (buffer-string))
                                        events)
                                       'saved)))
                                 (list
                                  (aozora-view-draw
                                   source
                                   "/library/ignored.txt")
                                  (buffer-string)
                                  (buffer-modified-p)
                                  (nreverse events))))))
                       (kill-buffer source)))"##;
    let expect = expect![[
        r#"OK (saved "source\ntext|replaced|laid-out" nil ((replace "source\ntext") (layout "source\ntext|replaced") (cache "/library/ignored.txt" "source\ntext|replaced|laid-out")))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn draw_file_fallback_obeys_the_packages_cp932_first_decoding_priority() {
    let elisp_form = r##"(let* ((file
                          (expand-file-name
                           "aozora-file-source.txt"
                           temporary-file-directory))
                         (dead
                          (generate-new-buffer
                           " *aozora-dead-source*")))
                     (with-temp-file file
                       (insert
                        "外部\r\nファイル"))
                     (kill-buffer dead)
                     (with-temp-buffer
                       (let ((events nil))
                         (cl-letf
                             (((symbol-function
                                'aozora-view-arrange-replace)
                               (lambda ()
                                 (push
                                  (list
                                   'replace
                                   (buffer-string))
                                  events)))
                              ((symbol-function
                                'aozora-view-arrange-fill-lines)
                               (lambda ()
                                 (push
                                  'layout
                                  events)))
                              ((symbol-function
                                'aozora-view-save-cache)
                               (lambda (path)
                                 (push
                                  (list
                                   'cache
                                   (file-name-nondirectory
                                    path))
                                  events))))
                           (list
                            (aozora-view-draw
                             dead file)
                            (buffer-string)
                            (buffer-modified-p)
                            (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (#1=((cache "aozora-file-source.txt")) #("螟夜Κ\n繝輔ぃ繧､繝ｫ" 0 8 (charset cp932-2-byte) 8 9 (charset katakana-sjis) 9 10 (charset cp932-2-byte) 10 11 (charset katakana-sjis)) nil ((replace #("螟夜Κ\n繝輔ぃ繧､繝ｫ" 0 8 (charset cp932-2-byte) 8 9 (charset katakana-sjis) 9 10 (charset cp932-2-byte) 10 11 (charset katakana-sjis))) layout . #1#))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn draw_file_fallback_decodes_a_real_cp932_aozora_text_payload() {
    let elisp_form = r##"(let* ((file
                          (expand-file-name
                           "aozora-cp932-source.txt"
                           temporary-file-directory))
                         (dead
                          (generate-new-buffer
                           " *aozora-cp932-dead-source*")))
                     (with-temp-buffer
                       (set-buffer-multibyte
                        nil)
                       (insert
                        (unibyte-string
                         #x8a #x4f #x95 #x94
                         #x0d #x0a
                         #x83 #x74 #x83 #x40
                         #x83 #x43 #x83 #x8b))
                       (let ((coding-system-for-write
                              'no-conversion))
                         (write-region
                          (point-min)
                          (point-max)
                          file
                          nil
                          'silent)))
                     (kill-buffer dead)
                     (with-temp-buffer
                       (let ((events nil))
                         (cl-letf
                             (((symbol-function
                                'aozora-view-arrange-replace)
                               (lambda ()
                                 (push
                                  (list
                                   'replace
                                   (buffer-string))
                                  events)))
                              ((symbol-function
                                'aozora-view-arrange-fill-lines)
                               (lambda ()
                                 (push
                                  'layout
                                  events)))
                              ((symbol-function
                                'aozora-view-save-cache)
                               (lambda (path)
                                 (push
                                  (list
                                   'cache
                                   (file-name-nondirectory
                                    path))
                                  events))))
                           (list
                            (aozora-view-draw
                             dead file)
                            (buffer-string)
                            (buffer-modified-p)
                            (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (#1=((cache "aozora-cp932-source.txt")) #("外部\nファイル" 0 7 (charset cp932-2-byte)) nil ((replace #("外部\nファイル" 0 7 (charset cp932-2-byte))) layout . #1#))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn draw_signals_when_neither_original_buffer_nor_source_file_exists() {
    let elisp_form = r##"(let ((dead
                         (generate-new-buffer
                          " *aozora-missing-source*")))
                     (kill-buffer dead)
                     (with-temp-buffer
                       (insert "stale")
                       (condition-case error
                           (aozora-view-draw
                            dead
                            "/definitely/missing/book.txt")
                         (error
                          (list
                           (error-message-string
                            error)
                           (buffer-string))))))"##;
    let expect = expect![[r#"OK ("元のバッファまたはテキストファイルが見付かりません！" "")"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn command_rejects_non_text_buffers_but_exposes_the_nil_file_edge_case_in_text_mode() {
    let elisp_form = r##"(mapcar
                      (lambda (mode)
                        (with-temp-buffer
                          (setq
                           major-mode
                           mode
                           buffer-file-name
                           nil)
                          (condition-case error
                              (aozora-view)
                            (error
                             (error-message-string
                              error)))))
                      '(fundamental-mode
                        text-mode))"##;
    let expect =
        expect![[r#"OK ("Buffer is not ‘*.txt’ text-mode." "Wrong type argument: stringp, nil")"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn command_cache_hit_creates_links_enters_mode_and_restores_navigation_without_drawing() {
    let elisp_form = r##"(let ((source
                         (generate-new-buffer
                          " *aozora-command-cache-source*"))
                        (view nil))
                     (unwind-protect
                         (with-current-buffer source
                           (text-mode)
                           (setq buffer-file-name
                                 "/library/cached-book.txt")
                           (let ((events nil))
                             (cl-letf
                                 (((symbol-function
                                    'aozora-view-load-cache)
                                   (lambda (file)
                                     (push
                                      (list
                                       'load-cache
                                       file)
                                      events)
                                     (insert
                                      "cached rendering")
                                     t))
                                  ((symbol-function
                                    'aozora-view-draw)
                                   (lambda
                                     (_buffer _file)
                                     (push
                                      'unexpected-draw
                                      events)))
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
                                      'restore
                                      events))))
                               (aozora-view)
                               (setq view
                                     (current-buffer))
                               (list
                                (buffer-name view)
                                (buffer-string)
                                major-mode
                                (eq
                                 aozora-view-text-buffer
                                 source)
                                aozora-view-text-file
                                (eq
                                 (with-current-buffer source
                                   aozora-view-buffer)
                                 view)
                                (nreverse events))))
                       (when
                           (buffer-live-p view)
                         (kill-buffer view))
                       (when
                           (buffer-live-p source)
                         (kill-buffer source)))))"##;
    let expect = expect!["OK t"];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn repeated_command_reuses_the_live_view_buffer_without_reloading_or_redrawing() {
    let elisp_form = r##"(let ((source
                         (generate-new-buffer
                          " *aozora-command-reuse-source*"))
                        (view nil))
                     (unwind-protect
                         (with-current-buffer source
                           (text-mode)
                           (setq buffer-file-name
                                 "/library/reused-book.txt")
                           (let ((events nil))
                             (cl-letf
                                 (((symbol-function
                                    'aozora-view-load-cache)
                                   (lambda (_file)
                                     (push
                                      'load
                                      events)
                                     nil))
                                  ((symbol-function
                                    'aozora-view-draw)
                                   (lambda (buffer file)
                                     (push
                                      (list
                                       'draw
                                       (eq buffer source)
                                       file)
                                      events)
                                     (insert
                                      "fresh rendering")))
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
                                      'restore
                                      events))))
                               (aozora-view)
                               (setq view
                                     (current-buffer))
                               (switch-to-buffer source)
                               (aozora-view)
                               (list
                                (eq
                                 view
                                 (current-buffer))
                                (buffer-string)
                                (nreverse events)))))
                       (when
                           (buffer-live-p view)
                         (kill-buffer view))
                       (when
                           (buffer-live-p source)
                         (kill-buffer source))))"##;
    let expect = expect![[
        r#"OK (t "fresh rendering" (load (draw t "/library/reused-book.txt") mode restore mode restore))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn end_to_end_command_renders_real_aozora_markup_into_a_read_only_linked_view() {
    let elisp_form = r##"(let ((source
                         (generate-new-buffer
                          " *aozora-real-source*"))
                        (view nil))
                     (unwind-protect
                         (with-current-buffer source
                           (text-mode)
                           (setq buffer-file-name
                                 "/library/real-book.txt")
                           (insert
                            "題名\n\n｜青空文庫《あおぞらぶんこ》へようこそ。\n漢［＃レ］文と強調［＃「強調」に傍線］。\n")
                           (defalias
                             'toggle-read-only
                             (lambda
                               (&optional argument)
                               (setq buffer-read-only
                                     (if argument
                                         (> (prefix-numeric-value
                                             argument)
                                            0)
                                       (not buffer-read-only)))))
                           (let ((aozora-fill-column
                                  40)
                                 (aozora-view-save-cache
                                  nil))
                             (aozora-view)
                             (setq view
                                   (current-buffer))
                             (list
                              (buffer-name)
                              major-mode
                              buffer-read-only
                              view-mode
                              (buffer-string)
                              (get-text-property
                               (point-min)
                               'display)
                              (progn
                                (goto-char
                                 (point-min))
                                (search-forward
                                 "強調")
                                (get-text-property
                                 (-
                                  (point)
                                  2)
                                 'face))
                              (eq
                               aozora-view-text-buffer
                               source)
                              aozora-view-text-file
                              (eq
                               (with-current-buffer source
                                 aozora-view-buffer)
                               view))))
                       (when
                           (buffer-live-p view)
                         (kill-buffer view))
                       (when
                           (buffer-live-p source)
                         (kill-buffer source))))"##;
    let expect = expect![[
        r#"OK ("real-book" aozora-view-mode t t #("\n題名\n\n\nあおぞらぶんこ\n青空文庫へようこそ。\n\n漢レ文と強調。\n" 0 1 (display #1=((height 0.5))) 1 2 (line-number 1) 4 5 (display #1#) 6 13 (display #1#) 13 14 (display #1#) 15 18 (read-only t) 25 26 (display #1#) 26 27 (line-number 4) 27 28 (display ((height 0.5))) 30 32 (face underline)) ((height 0.5)) underline t "/library/real-book.txt" t)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}
