use expect_test::expect;

use super::{assert_ac_skk_parity, assert_ac_skk_signal_parity};

#[test]
fn ac_skk_kakutei_replaces_active_text_starts_after_selected_count_and_confirms() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "headINPUT")
               (let ((skk-katakana
                      nil)
                     (skk-henkan-start-point
                      5)
                     (ac-skk-selected-candidate
                      (ac-skk-make-cand
                       "候補"
                       'ac-skk-kakutei
                       "かな"
                       2))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ac-skk-start-henkan)
                       (lambda (count)
                         (push
                          (list
                           'start
                           count)
                          calls)
                         'started))
                      ((symbol-function
                        'skk-kakutei)
                       (lambda ()
                         (push
                          'confirmed
                          calls)
                         'confirmed-result)))
                   (list
                    (ac-skk-kakutei)
                    (buffer-string)
                    (point)
                    (nreverse
                     calls)))))"##;
    let expect = expect![[r#"OK (confirmed-result "headかな" 7 ((start 3) confirmed))"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_kakutei_rejects_katakana_before_reading_candidate_or_mutating_buffer() {
    let elisp_form = r##"(let ((skk-katakana
                    t))
               (ac-skk-kakutei))"##;
    let expect = expect![[r#"ERR (error "No Support skk-katakana mode.")"#]];

    assert_ac_skk_signal_parity(elisp_form, expect);
}

#[test]
fn ac_skk_henkan_forward_reads_count_from_completed_buffer_then_advances_once() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "prefix")
               (let ((ac-point
                      (point)))
                 (insert
                  (propertize
                   "候補"
                   'skk-count
                   4))
                 (let (calls)
                   (cl-letf
                       (((symbol-function
                          'ac-skk-start-henkan)
                         (lambda (count)
                           (push
                            (list
                             'start
                             count)
                            calls)
                           'started))
                        ((symbol-function
                          'skk-start-henkan)
                         (lambda (count)
                           (push
                            (list
                             'forward
                             count)
                            calls)
                           'forwarded)))
                     (list
                      (ac-skk-henkan-forward)
                      (buffer-string)
                      (point)
                      (nreverse
                       calls))))))"##;
    let expect =
        expect![[r#"OK (forwarded #("prefix候補" 6 8 (skk-count 4)) 9 ((start 4) (forward 1)))"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_start_henkan_calls_exact_count_with_annotations_disabled_and_restores_binding() {
    let elisp_form = r##"(let ((skk-show-annotation
                    'outer)
                   calls)
               (cl-letf
                   (((symbol-function
                      'skk-start-henkan)
                     (lambda (count)
                       (push
                        (list
                         count
                         skk-show-annotation)
                        calls)
                       'started)))
                 (list
                  (ac-skk-start-henkan
                   3)
                  (ac-skk-start-henkan
                   0)
                  (ac-skk-start-henkan
                   -2)
                  skk-show-annotation
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (nil nil nil outer ((1 nil) (1 nil) (1 nil)))"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_hiracomp_mes_rewrites_hiragana_marker_then_sets_point_and_restarts_completion() {
    let elisp_form = r##"(mapcar
               (lambda (text)
                 (with-temp-buffer
                   (insert
                    text)
                   (let (calls)
                     (cl-letf
                         (((symbol-function
                            'skk-set-henkan-point-subr)
                           (lambda ()
                             (push
                              (list
                               'set-point
                               (point)
                               (buffer-string))
                              calls)
                             'set))
                          ((symbol-function
                            'ac-start)
                           (lambda (&rest arguments)
                             (push
                              (list
                               'start
                               arguments
                               (point)
                               (buffer-string))
                              calls)
                             'started)))
                       (string-match
                        "fixture"
                        "fixture")
                       (list
                        (ac-skk-hiracomp-mes)
                        (buffer-string)
                        (point)
                        (match-data)
                        (nreverse
                         calls))))))
               '("prefix▽かな"
                 "prefix▽カナ"
                 "plain"))"##;
    let expect = expect![[
        r#"OK ((started "prefixかな" 9 (0 7) ((set-point 7 "prefix") (start (:force-init t) 9 "prefixかな"))) (started "prefix▽カナ" 10 (0 7) ((set-point 10 "prefix▽カナ") (start (:force-init t) 10 "prefix▽カナ"))) (started "plain" 6 (0 7) ((set-point 6 "plain") (start (:force-init t) 6 "plain"))))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}
