use expect_test::expect;

use super::assert_auto_complete_pcmp_parity;

#[test]
fn auto_complete_pcmp_candidates_capture_direct_pcomplete_result() {
    let elisp_form = r##"(with-temp-buffer
         (insert "git ch")
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (setq ac-pcmp--candidates
                            '("checkout" "cherry-pick" "cherry")))))
           (list
            (ac-pcmp/get-ac-candidates)
            (point)
            (ac-pcmp-test-state))))"##;
    let expect = expect![[
        r#"OK (("checkout" "cherry-pick" "cherry") 7 (:active nil :candidates nil :status none :point 7 :last-length nil :last-stub nil))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidates_capture_show_completions_advice_path() {
    let elisp_form = r##"(with-temp-buffer
         (insert "cargo ")
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (pcomplete-show-completions
                       '("build" "check" "clippy" "nextest")))))
           (list
            (ac-pcmp/get-ac-candidates)
            (buffer-string)
            (ac-pcmp-test-state))))"##;
    let expect = expect![[
        r#"OK (("build" "check" "clippy" "nextest") "cargo " (:active nil :candidates nil :status none :point 7 :last-length nil :last-stub nil))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidates_capture_stub_advice_and_real_insertion() {
    let elisp_form = r##"(with-temp-buffer
         (insert "sta")
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (pcomplete-stub
                       "sta"
                       '("status" "stash" "stage")))))
           (let ((candidates (ac-pcmp/get-ac-candidates)))
             (list
              candidates
              (buffer-string)
              (point)
              ac-pcmp--status
              (ac-pcmp-test-state)))))"##;
    let expect = expect![[
        r#"OK (("status" "stash" "stage") "sta" 4 nil (:active nil :candidates nil :status nil :point 4 :last-length nil :last-stub nil))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidates_capture_pcomplete_completions_return_value() {
    let elisp_form = r##"(with-temp-buffer
         (insert "deploy ")
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (let ((pcomplete-index 0)
                            (pcomplete-last 0)
                            (pcomplete-command-completion-function
                             (lambda ()
                               '("staging" "production" "preview"))))
                        (cl-letf
                            (((symbol-function 'pcomplete-parse-arguments)
                              (lambda (&optional _expand) t)))
                          (pcomplete-completions))))))
           (list
            (ac-pcmp/get-ac-candidates)
            ac-pcmp--status
            ac-pcmp--point)))"##;
    let expect = expect![[r#"OK (("staging" "production" "preview") none 8)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_first_candidate_capture_wins_across_multiple_advice_paths() {
    let elisp_form = r##"(with-temp-buffer
         (insert "tool ")
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (pcomplete-show-completions '("first" "choice"))
                      (pcomplete-show-completions '("second" "ignored"))
                      (pcomplete-stub "" '("third" "ignored")))))
           (list
            (ac-pcmp/get-ac-candidates)
            ac-pcmp--status
            (buffer-string))))"##;
    let expect = expect![[r#"OK (("first" "choice") nil "tool ")"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidate_request_resets_stale_global_state_each_time() {
    let elisp_form = r##"(progn
         (setq ac-pcmp--candidates '("stale")
               ac-pcmp--status 'sole
               ac-pcmp--point 999)
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (setq ac-pcmp--candidates '("fresh")))))
           (with-temp-buffer
             (insert "abc")
             (let ((first (ac-pcmp/get-ac-candidates)))
               (erase-buffer)
               (insert "longer input")
               (cl-letf (((symbol-function 'call-interactively)
                          (lambda (_command) nil)))
                 (list
                  first
                  (ac-pcmp/get-ac-candidates)
                  (ac-pcmp-test-state)))))))"##;
    let expect = expect![[
        r#"OK (("fresh") nil (:active nil :candidates ("stale") :status none :point 13 :last-length nil :last-stub nil))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidate_request_records_exact_point_without_mutating_text() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (insert (car case))
             (goto-char (cdr case))
             (let ((before (buffer-string)))
               (cl-letf (((symbol-function 'call-interactively)
                          (lambda (_command)
                            (setq ac-pcmp--candidates '("candidate")))))
                 (list
                  case
                  (ac-pcmp/get-ac-candidates)
                  ac-pcmp--point
                  (point)
                  (equal before (buffer-string)))))))
         '(("abc" . 1)
           ("abc" . 2)
           ("long command" . 8)))"##;
    let expect = expect![[
        r#"OK ((("abc" . 1) #1=("candidate") 1 1 t) (("abc" . 2) #1# 2 2 t) (("long command" . 8) #1# 8 8 t))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidate_errors_become_prefixed_messages_and_nil_results() {
    let elisp_form = r##"(cl-letf (((symbol-function 'call-interactively)
                                    (lambda (_command)
                                      (error "fixture failure %s" 17))))
         (with-temp-buffer
           (insert "broken")
           (let ((result (ac-pcmp/get-ac-candidates)))
             (list
              result
              (current-message)
              ac-pcmp--status
              ac-pcmp--point
              (buffer-string)))))"##;
    let expect = expect![[r#"OK (nil nil none 7 "broken")"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_candidate_dynamic_activation_does_not_leak_after_call() {
    let elisp_form = r##"(let ((ac-pcmp--active-p nil)
             (ac-pcmp--candidates '("outer")))
         (cl-letf (((symbol-function 'call-interactively)
                    (lambda (_command)
                      (list
                       ac-pcmp--active-p
                       ac-pcmp--candidates))))
           (let ((result (ac-pcmp/get-ac-candidates)))
             (list
              result
              ac-pcmp--active-p
              ac-pcmp--candidates
              ac-pcmp--status))))"##;
    let expect = expect![[r#"OK (nil nil ("outer") none)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_empty_pcomplete_result_is_distinct_from_error() {
    let elisp_form = r##"(cl-letf (((symbol-function 'call-interactively)
                                    (lambda (_command) nil)))
         (with-temp-buffer
           (insert "nothing")
           (let ((result (ac-pcmp/get-ac-candidates)))
             (list
              result
              (current-message)
              ac-pcmp--status
              ac-pcmp--point))))"##;
    let expect = expect!["OK (nil nil none 8)"];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}
