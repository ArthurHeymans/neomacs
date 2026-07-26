use expect_test::expect;

use super::assert_ac_skk_parity;

#[test]
fn ac_skk_prefix_requires_skk_mode_and_the_exact_on_conversion_state() {
    let elisp_form = r##"(let ((skk-henkan-start-point
                    17))
               (mapcar
                (lambda (fixture)
                  (let ((skk-mode
                         (car
                          fixture))
                        (skk-henkan-mode
                         (cadr
                          fixture)))
                    (ac-skk-prefix)))
                '((t on)
                  (nil on)
                  (t off)
                  (active on)
                  (t t)
                  (t nil))))"##;
    let expect = expect!["OK (17 nil nil 17 nil nil)"];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_make_cand_copies_text_and_installs_exact_action_key_and_count_properties() {
    let elisp_form = r##"(let* ((source
                     "候補")
                    (candidate
                     (ac-skk-make-cand
                      source
                      'fixture-action
                      "かな"
                      7)))
               (list
                (eq
                 source
                 candidate)
                source
                (text-properties-at
                 0
                 source)
                candidate
                (substring-no-properties
                 candidate)
                (text-properties-at
                 0
                 candidate)
                (get-text-property
                 0
                 'action
                 candidate)
                (get-text-property
                 0
                 'henkan-key
                 candidate)
                (get-text-property
                 0
                 'skk-count
                 candidate)))"##;
    let expect = expect![[
        r#"OK (nil "候補" nil #("候補" 0 2 (action fixture-action henkan-key "かな" skk-count 7)) "候補" (action fixture-action henkan-key "かな" skk-count 7) fixture-action "かな" 7)"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_make_cand_list_forwards_search_arguments_numbers_results_and_discards_forward_candidate()
{
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'skk-search-progs)
                     (lambda (&rest arguments)
                       (push
                        (list
                         'search
                         arguments)
                        calls)
                       '("第一"
                         "second"
                         "第一")))
                    ((symbol-function
                      'ac-skk-make-cand)
                     (lambda (&rest arguments)
                       (push
                        (list
                         'make
                         arguments)
                        calls)
                       (cons
                        'made
                        arguments))))
                 (let ((result
                        (ac-skk-make-cand-list
                         "かな"
                         '((prog-a)
                           (prog-b 2)))))
                   (list
                    result
                    (nreverse
                     calls)))))"##;
    let expect = expect![[
        r#"OK (((made . #1=("第一" ac-skk-kakutei "かな" 0)) (made . #2=("second" ac-skk-kakutei "かな" 1)) (made . #3=("第一" ac-skk-kakutei "かな" 2))) ((search ("かな" ((prog-a) (prog-b 2)) remove-note)) (make #1#) (make #2#) (make #3#) (make ("かな" ac-skk-henkan-forward "かな" 3))))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_candidates_require_conversion_and_build_prefix_and_okuri_programs_in_exact_order() {
    let elisp_form = r##"(let ((skk-henkan-mode
                    'on)
                   (skk-search-prog-list
                    '((first-search)
                      (second-search 2)
                      (skk-okuri-search)
                      (ignored-search)))
                   (skk-jisyo
                    "fixture-jisyo")
                   (ac-prefix
                    "かな")
                   calls)
               (cl-letf
                   (((symbol-function
                      'skk-comp-get-all-candidates)
                     (lambda (&rest arguments)
                       (push
                        (list
                         'complete
                         arguments)
                        calls)
                       '("仮名"
                         "かな")))
                    ((symbol-function
                      'ac-skk-make-cand-list)
                     (lambda (midasi
                              programs)
                       (push
                        (list
                         'make
                         midasi
                         programs)
                        calls)
                       (list
                        (list
                         'result
                         midasi
                         programs)))))
                 (let ((without-auto-okuri
                        (let ((skk-auto-okuri-process
                               nil))
                          (ac-skk-candidates)))
                       (with-auto-okuri
                        (let ((skk-auto-okuri-process
                               t))
                          (ac-skk-candidates))))
                   (list
                    without-auto-okuri
                    with-auto-okuri
                    (nreverse
                     calls)))))"##;
    let expect = expect![[
        r#"OK (((result "かな" #1=(#2=(first-search) #3=(second-search 2))) (result "仮名" #1#) (result "かな" #1#)) ((result "かな" #4=(#2# #3# (skk-okuri-search-1))) (result "仮名" #4#) (result "かな" #4#)) ((complete ("かな" nil #5=((skk-comp-from-jisyo skk-jisyo)))) (make "かな" #1#) (make "仮名" #1#) (make "かな" #1#) (complete ("かな" nil #5#)) (make "かな" #4#) (make "仮名" #4#) (make "かな" #4#)))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_candidates_use_the_whole_search_program_list_when_okuri_marker_is_absent() {
    let elisp_form = r##"(let ((skk-henkan-mode
                    'on)
                   (skk-search-prog-list
                    '((first-search)
                      (last-search)))
                   (skk-auto-okuri-process
                    t)
                   (skk-jisyo
                    "fixture-jisyo")
                   (ac-prefix
                    "かな")
                   calls)
               (cl-letf
                   (((symbol-function
                      'skk-comp-get-all-candidates)
                     (lambda (&rest arguments)
                       (push
                        (list
                         'complete
                         arguments)
                        calls)
                       nil))
                    ((symbol-function
                      'ac-skk-make-cand-list)
                     (lambda (midasi
                              programs)
                       (push
                        (list
                         'make
                         midasi
                         programs)
                        calls)
                       (list
                        midasi))))
                 (list
                  (ac-skk-candidates)
                  (nreverse
                   calls))))"##;
    let expect = expect![[
        r#"OK (("かな") ((complete ("かな" nil ((skk-comp-from-jisyo skk-jisyo)))) (make "かな" ((first-search) (last-search) (skk-okuri-search-1)))))"#
    ]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_candidates_short_circuit_all_completion_work_outside_on_conversion_state() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'skk-comp-get-all-candidates)
                 (lambda (&rest _arguments)
                   (error
                    "completion called")))
                ((symbol-function
                  'ac-skk-make-cand-list)
                 (lambda (&rest _arguments)
                   (error
                    "candidate builder called"))))
             (list
              (let ((skk-henkan-mode
                     nil))
                (ac-skk-candidates))
              (let ((skk-henkan-mode
                     'off))
                (ac-skk-candidates))
              (let ((skk-henkan-mode
                     t))
                (ac-skk-candidates))))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_ac_skk_parity(elisp_form, expect);
}
