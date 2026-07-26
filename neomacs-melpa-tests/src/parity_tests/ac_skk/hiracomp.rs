use expect_test::expect;

use super::assert_ac_skk_parity;

#[test]
fn ac_skk_prefix_hiracomp_applies_mode_gates_and_each_final_segment_length_rule() {
    let elisp_form = r##"(let ((skk-mode
                    t)
                   (skk-j-mode
                    t)
                   (skk-henkan-mode
                    nil)
                   segments
                   calls)
               (cl-letf
                   (((symbol-function
                      'tseg-segment)
                     (lambda (text)
                       (push
                        text
                        calls)
                       segments)))
                 (list
                  (with-temp-buffer
                    (insert
                     "かな")
                    (setq
                     segments
                     '("かな"))
                    (ac-skk-prefix-hiracomp))
                  (with-temp-buffer
                    (insert
                     "あいう")
                    (setq
                     segments
                     '("あ"
                       "いう"))
                    (ac-skk-prefix-hiracomp))
                  (with-temp-buffer
                    (insert
                     "あいう")
                    (setq
                     segments
                     '("あい"
                       "う"))
                    (ac-skk-prefix-hiracomp))
                  (with-temp-buffer
                    (insert
                     "あいうえ")
                    (setq
                     segments
                     '("あ"
                       "いう"
                       "え"))
                    (ac-skk-prefix-hiracomp))
                  (let ((skk-mode
                         nil))
                    (with-temp-buffer
                      (insert
                       "かな")
                      (ac-skk-prefix-hiracomp)))
                  (let ((skk-j-mode
                         nil))
                    (with-temp-buffer
                      (insert
                       "かな")
                      (ac-skk-prefix-hiracomp)))
                  (let ((skk-henkan-mode
                         'on))
                    (with-temp-buffer
                      (insert
                       "かな")
                      (ac-skk-prefix-hiracomp)))
                  (nreverse
                   calls))))"##;
    let expect = expect![[r#"OK (1 2 1 2 nil nil nil ("かな" "あいう" "あいう" "あいうえ"))"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_prefix_hiracomp_accepts_every_japanese_category_and_strips_text_properties() {
    let elisp_form = r##"(let ((skk-mode
                    t)
                   (skk-j-mode
                    t)
                   (skk-henkan-mode
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'tseg-segment)
                     (lambda (text)
                       (push
                        (list
                         text
                         (text-properties-at
                          0
                          text))
                        calls)
                       (list
                        text))))
                 (list
                  (mapcar
                   (lambda (text)
                     (with-temp-buffer
                       (insert
                        (propertize
                         text
                         'fixture
                         t))
                       (list
                        text
                        (ac-skk-prefix-hiracomp))))
                   '("かな"
                     "カナ"
                     "漢字"
                     "ascii"))
                  (nreverse
                   calls))))"##;
    let expect = expect![[
        r#"OK ((("かな" 1) ("カナ" 1) ("漢字" 1) ("ascii" nil)) (("かな" nil) ("カナ" nil) ("漢字" nil)))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_prefix_hiracomp_requires_segmenter_and_preserves_existing_match_data() {
    let elisp_form = r##"(let ((skk-mode
                    t)
                   (skk-j-mode
                    t)
                   (skk-henkan-mode
                    nil)
                   (original
                    (symbol-function
                     'tseg-segment)))
               (unwind-protect
                   (progn
                     (fmakunbound
                      'tseg-segment)
                     (with-temp-buffer
                       (insert
                        "かな")
                       (string-match
                        "\\(fix\\)"
                        "fixture")
                       (list
                        (ac-skk-prefix-hiracomp)
                        (match-data))))
                 (fset
                  'tseg-segment
                  original)))"##;
    let expect = expect!["OK (nil (0 3 0 3))"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_prefix_hiracomp_limits_matching_to_ten_preceding_japanese_characters() {
    let elisp_form = r##"(let ((skk-mode
                    t)
                   (skk-j-mode
                    t)
                   (skk-henkan-mode
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'tseg-segment)
                     (lambda (text)
                       (push
                        text
                        calls)
                       (list
                        text))))
                 (with-temp-buffer
                   (insert
                    "あいうえおかきくけこさし")
                   (list
                    (ac-skk-prefix-hiracomp)
                    (nreverse
                     calls)))))"##;
    let expect = expect![[r#"OK (3 ("うえおかきくけこさし"))"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_hiracomp_candidates_append_every_split_marker_after_dictionary_results() {
    let elisp_form = r##"(let ((ac-prefix
                    "かな")
                   (skk-jisyo
                    "fixture-jisyo")
                   calls)
               (cl-letf
                   (((symbol-function
                      'skk-search-progs)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       '("辞書"
                         "候補"))))
                 (let ((result
                        (ac-skk-hiracomp-candidates)))
                   (list
                    result
                    (mapcar
                     (lambda (candidate)
                       (list
                        (substring-no-properties
                         candidate)
                        (text-properties-at
                         0
                         candidate)))
                     result)
                    (nreverse
                     calls)))))"##;
    let expect = expect![[
        r#"OK (("辞書" "候補" #("▽かな" 0 3 (action ac-skk-hiracomp-mes)) #("か▽な" 0 3 (action ac-skk-hiracomp-mes))) (("辞書" nil) ("候補" nil) ("▽かな" (action ac-skk-hiracomp-mes)) ("か▽な" (action ac-skk-hiracomp-mes))) (("かな" ((skk-search-jisyo-file skk-jisyo 0 t) (skk-okuri-search-1) (skk-search-katakana)) remove-note)))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}
