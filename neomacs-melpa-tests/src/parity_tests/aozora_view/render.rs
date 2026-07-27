use expect_test::expect;

use super::{assert_aozora_view_parity, assert_aozora_view_signal_parity};

#[test]
fn real_gaiji_annotations_replace_jis_unicode_and_named_entries_in_one_pass() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "JIS=※［＃「朽のつくり」、第4水準2-1-2］ UCS=※［＃雪だるま、UCS-2603、説明］ Named=※［＃「朽のつくり」、第4水準2-1-2］ Unknown=※［＃存在しない外字、説明］")
                     (aozora-view-arrange-replace)
                     (buffer-string))"##;
    let expect = expect![[
        r#"OK #("JIS=丂 UCS=☃ Named=丂 Unknown=※［＃存在しない外字、説明］" 0 1 (line-number 1))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn iteration_marks_are_normalized_without_consuming_surrounding_prose() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "山／＼、時／″＼。既存の〳〵と〴〵。")
                     (aozora-view-arrange-replace)
                     (buffer-string))"##;
    let expect = expect![[r#"OK #("山〳〵、時〴〵。既存の〳〵と〴〵。" 0 1 (line-number 1))"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn every_kenten_style_becomes_a_length_matched_ruby_run() {
    let elisp_form = r##"(mapcar
                      (lambda (entry)
                        (with-temp-buffer
                          (let ((text "青空文庫"))
                            (insert
                             text
                             "［＃「"
                             text
                             "」に"
                             (car entry)
                             "］")
                            (aozora-view-arrange-replace)
                            (list
                             (buffer-string)
                             (get-text-property
                              1
                              'ruby)
                             (get-text-property
                              2
                              'read-only)))))
                      aozora-kenten-alist)"##;
    let expect = expect![[
        r#"OK ((#("青空文庫" 0 1 (ruby #1=(4 . "﹅﹅﹅﹅")) 1 4 (read-only t ruby #1#)) (4 . "﹅﹅﹅﹅") t) (#("青空文庫" 0 1 (ruby #2=(4 . "﹆﹆﹆﹆")) 1 4 (read-only t ruby #2#)) (4 . "﹆﹆﹆﹆") t) (#("青空文庫" 0 1 (ruby #3=(4 . "●●●●")) 1 4 (read-only t ruby #3#)) (4 . "●●●●") t) (#("青空文庫" 0 1 (ruby #4=(4 . "○○○○")) 1 4 (read-only t ruby #4#)) (4 . "○○○○") t) (#("青空文庫" 0 1 (ruby #5=(4 . "○○○○")) 1 4 (read-only t ruby #5#)) (4 . "○○○○") t) (#("青空文庫" 0 1 (ruby #6=(4 . "▲▲▲▲")) 1 4 (read-only t ruby #6#)) (4 . "▲▲▲▲") t) (#("青空文庫" 0 1 (ruby #7=(4 . "△△△△")) 1 4 (read-only t ruby #7#)) (4 . "△△△△") t) (#("青空文庫" 0 1 (ruby #8=(4 . "◎◎◎◎")) 1 4 (read-only t ruby #8#)) (4 . "◎◎◎◎") t) (#("青空文庫" 0 1 (ruby #9=(4 . "◉◉◉◉")) 1 4 (read-only t ruby #9#)) (4 . "◉◉◉◉") t))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn missing_kenten_target_removes_the_annotation_and_reports_the_real_problem() {
    let elisp_form = r##"(with-temp-buffer
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
                         (insert
                          "本文［＃「見つからない」には傍点］")
                         (aozora-view-arrange-replace)
                         (list
                          (buffer-string)
                          (nreverse messages)))))"##;
    let expect =
        expect![[r#"OK (#("本文［＃「見つからない」には傍点］" 0 1 (line-number 1)) nil)"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn kanbun_parenthetical_small_print_and_inline_small_print_keep_exact_display_properties() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "漢［＃レ一上］字 ［＃（注記）］ 本文［＃「本文」は行右小書き］ x［＃「x」は下付き小文字］ y［＃「y」は上付き小文字］")
                     (aozora-view-arrange-replace)
                     (let ((positions nil))
                       (goto-char
                        (point-min))
                       (dolist
                           (needle
                            '("レ" "注" "本" "x" "y"))
                         (search-forward needle)
                         (push
                          (list
                           needle
                           (get-text-property
                            (1-
                             (point))
                            'display))
                          positions))
                       (list
                        (buffer-string)
                        (nreverse positions))))"##;
    let expect = expect![[
        r#"OK (#("漢レ一上字 注記 本文 x y" 0 1 (line-number 1) 1 4 (display ((height 0.5))) 6 8 (display ((height 0.5) (raise 1))) 9 11 (display ((height 0.5))) 12 13 (display ((height 0.5))) 14 15 (display ((height 0.5) (raise 1)))) (("レ" ((height 0.5))) ("注" ((height 0.5) (raise 1))) ("本" ((height 0.5))) ("x" ((height 0.5))) ("y" ((height 0.5) (raise 1)))))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn missing_inline_small_print_targets_signal_each_specific_diagnostic() {
    let elisp_form = r##"(mapcar
                      (lambda (source)
                        (with-temp-buffer
                          (insert source)
                          (condition-case error
                              (progn
                                (aozora-view-arrange-replace)
                                'no-error)
                            (error
                             (error-message-string
                              error)))))
                      '("本文［＃「欠落」は行右小書き］"
                        "本文［＃「x」は下付き小文字］"
                        "本文［＃「y」は上付き小文字］"
                        "本文［＃「欠落」に「ママ」の注記］"))"##;
    let expect = expect![[
        r#"OK ("行右小書き指示の対応テキストが見つかりません！" "下付き指示の対応テキストが見つかりません！" "上付き注記指示の対応テキストが見つかりません！" "ママ注記指示の対応テキストが見つかりません！")"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn mama_and_underline_annotations_apply_to_the_previous_matching_text_only() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "喋［＃「喋」に「ママ」の注記］ 前の語 強調［＃「強調」に傍線］ 後の語［＃「欠落」に傍線］")
                     (aozora-view-arrange-replace)
                     (goto-char
                      (point-min))
                     (search-forward
                      "強調")
                     (list
                      (buffer-string)
                      (get-text-property
                       (-
                        (point)
                        2)
                       'face)
                      (get-text-property
                       1
                       'ruby)
                      (get-text-property
                       1
                       'read-only)))"##;
    let expect = expect![[
        r#"OK (#("喋 前の語 強調 後の語" 0 1 (ruby (1 . "ママ")) 6 8 (face underline)) underline (1 . "ママ") nil)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn accent_separation_is_enabled_only_after_the_work_header_marker() {
    let elisp_form = r##"(progn
                     (require 'ucs-normalize)
                     (mapcar
                      (lambda (source)
                        (with-temp-buffer
                          (insert source)
                          (aozora-view-arrange-replace)
                          (buffer-string)))
                      '("〔〕はアクセント分解記号\n語: 〔xCafe'〕 〔xAE&〕 〔x?!@〕"
                        "見出しなし: 〔xCafe'〕 〔xAE&〕")))"##;
    let expect = expect![[
        r#"OK (#("〔〕はアクセント分解記号\n語: Café Æ ?¡" 0 1 (line-number 1) 13 14 (line-number 2)) #("見出しなし: 〔xCafe'〕 〔xAE&〕" 0 1 (line-number 1)))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn accent_rendering_surfaces_the_packages_unloaded_normalizer_dependency() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "〔〕はアクセント分解記号\n語: 〔xCafe'〕")
                     (aozora-view-arrange-replace))"##;
    let expect = expect!["ERR (void-function ucs-normalize-NFC-region)"];
    assert_aozora_view_signal_parity(elisp_form, expect);
}

#[test]
fn warichu_and_indent_blocks_remove_markers_and_preserve_layout_properties() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "前［＃ここから割り注］割注本文［＃ここで割り注終わり］後\n［＃ここから３字下げ］字下げ一\n字下げ二［＃ここで字下げ終わり］\n終")
                     (aozora-view-arrange-replace)
                     (goto-char
                      (point-min))
                     (search-forward
                      "割注本文")
                     (let ((warichu
                            (get-text-property
                             (-
                              (point)
                              4)
                             'display)))
                       (search-forward
                        "字下げ一")
                       (list
                        (buffer-string)
                        warichu
                        (get-text-property
                         (-
                          (point)
                          4)
                         'left-margin)
                        (get-text-property
                         (point-min)
                         'line-number)
                        (progn
                          (goto-char
                           (point-min))
                          (forward-line 1)
                          (get-text-property
                           (point)
                           'line-number)))))"##;
    let expect = expect![[
        r#"OK (#("前割注本文後\n字下げ一\n字下げ二終" 0 1 (line-number 1) 1 5 (display ((height 0.5) (raise 0.5))) 7 8 (line-number 2 left-margin 6) 8 12 (left-margin 6) 12 13 (line-number 3 left-margin 6) 13 16 (left-margin 6)) ((height 0.5) (raise 0.5)) 6 1 2)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn unterminated_warichu_and_indent_blocks_signal_distinct_errors() {
    let elisp_form = r##"(mapcar
                      (lambda (source)
                        (with-temp-buffer
                          (insert source)
                          (condition-case error
                              (progn
                                (aozora-view-arrange-replace)
                                'no-error)
                            (error
                             (error-message-string
                              error)))))
                      '("前［＃ここから割り注］終端なし"
                        "［＃ここから2字下げ］終端なし"))"##;
    let expect = expect![[
        r#"OK ("割注終了指示が見付かりません。" "[字下げ] instruction does not match!")"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn explicit_and_implicit_ruby_syntax_produce_exact_main_text_and_read_only_ranges() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "｜青空文庫《あおぞらぶんこ》と漢字《かんじ》、abc《エービーシー》。")
                     (aozora-view-arrange-replace)
                     (let ((runs nil)
                           (position
                            (point-min)))
                       (while
                           (setq position
                                 (text-property-not-all
                                  position
                                  (point-max)
                                  'ruby
                                  nil))
                         (let ((ruby
                                (get-text-property
                                 position
                                 'ruby)))
                           (when ruby
                             (push
                              (list
                               position
                               (buffer-substring-no-properties
                                position
                                (+
                                 position
                                 (car ruby)))
                               ruby
                               (get-text-property
                                position
                                'read-only)
                               (get-text-property
                                (1+
                                 position)
                                'read-only))
                              runs)
                             (setq position
                                   (+
                                    position
                                    (car ruby))))))
                       (list
                        (buffer-string)
                        (nreverse runs))))"##;
    let expect = expect![[
        r#"OK (#("青空文庫と漢字、abc。" 0 1 (ruby #1=(4 . "あおぞらぶんこ")) 1 4 (read-only t ruby #1#) 5 6 (ruby #2=(2 . "かんじ")) 6 7 (read-only t ruby #2#) 8 9 (ruby #3=(3 . "エービーシー")) 9 11 (read-only t ruby #3#)) ((1 "青空文庫" (4 . "あおぞらぶんこ") nil t) (6 "漢字" (2 . "かんじ") nil t) (9 "abc" (3 . "エービーシー") nil t)))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn replacement_starts_from_clean_properties_but_leaves_unknown_directives_verbatim() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      (propertize
                       "本文"
                       'face
                       'bold
                       'custom
                       42)
                      "［＃未実装の指示］")
                     (aozora-view-arrange-replace)
                     (list
                      (buffer-string)
                      (text-properties-at
                       (point-min))
                      (get-text-property
                       (point-min)
                       'line-number)))"##;
    let expect =
        expect![[r#"OK (#("本文［＃未実装の指示］" 0 1 (line-number 1)) (line-number 1) 1)"#]];
    assert_aozora_view_parity(elisp_form, expect);
}
