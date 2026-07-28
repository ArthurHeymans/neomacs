use expect_test::expect;

use super::assert_auto_complete_pcmp_parity;

#[test]
fn auto_complete_pcmp_completions_advice_captures_non_nil_return_once() {
    let elisp_form = r##"(let ((ac-pcmp--active-p t)
             (ac-pcmp--candidates nil)
             (pcomplete-index 0)
             (pcomplete-last 0)
             (pcomplete-command-completion-function
              (lambda () '("alpha" "beta" "gamma"))))
         (cl-letf (((symbol-function 'pcomplete-parse-arguments)
                    (lambda (&optional _expand) t)))
           (let ((first (pcomplete-completions)))
             (setq pcomplete-command-completion-function
                   (lambda () '("later" "ignored")))
             (let ((second (pcomplete-completions)))
               (list
                first
                second
                ac-pcmp--candidates)))))"##;
    let expect = expect![[r#"OK (#1=("alpha" "beta" "gamma") ("later" "ignored") #1#)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_completions_advice_ignores_inactive_requests() {
    let elisp_form = r##"(let ((ac-pcmp--active-p nil)
             (ac-pcmp--candidates '("preserved"))
             (pcomplete-index 0)
             (pcomplete-last 0)
             (pcomplete-command-completion-function
              (lambda () '("returned"))))
         (cl-letf (((symbol-function 'pcomplete-parse-arguments)
                    (lambda (&optional _expand) t)))
           (list
            (pcomplete-completions)
            ac-pcmp--candidates
            ac-pcmp--status)))"##;
    let expect = expect![[r#"OK (("returned") ("preserved") nil)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_show_completions_advice_suppresses_ui_and_captures_input() {
    let elisp_form = r##"(let ((ac-pcmp--active-p t)
             (ac-pcmp--candidates nil)
             (pcomplete-last-window-config :untouched)
             (pcomplete-window-restore-timer nil))
         (list
          (pcomplete-show-completions
           '("zeta" "alpha" "middle"))
          ac-pcmp--candidates
          pcomplete-last-window-config
          (get-buffer "*Completions*")))"##;
    let expect = expect![[r#"OK (nil ("zeta" "alpha" "middle") :untouched nil)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_show_completions_advice_preserves_first_capture() {
    let elisp_form = r##"(let ((ac-pcmp--active-p t)
             (ac-pcmp--candidates '("already")))
         (list
          (pcomplete-show-completions '("new" "ignored"))
          ac-pcmp--candidates
          (pcomplete-show-completions nil)
          ac-pcmp--candidates))"##;
    let expect = expect![[r#"OK (nil #1=("already") nil #1#)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_stub_advice_captures_original_candidate_collection() {
    let elisp_form = r##"(let ((ac-pcmp--active-p t)
             (ac-pcmp--candidates nil)
             (ac-pcmp--status 'none))
         (with-temp-buffer
           (insert "ca")
           (let ((result
                  (pcomplete-stub
                   "ca"
                   '("cargo" "cache" "cat"))))
             (list
              result
              ac-pcmp--candidates
              ac-pcmp--status
              (buffer-string)
              (point)))))"##;
    let expect = expect![[r#"OK (nil ("cargo" "cache" "cat") nil "ca" 3)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_stub_advice_maps_real_completion_outcomes_to_status() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (insert (nth 0 case))
             (let ((ac-pcmp--active-p t)
                   (ac-pcmp--candidates nil)
                   (ac-pcmp--status 'none)
                   (pcomplete-termination-string " ")
                   (pcomplete-suffix-list '(?/ ?:)))
               (list
                case
                (pcomplete-stub (nth 0 case) (nth 1 case))
                ac-pcmp--status
                ac-pcmp--candidates
                (buffer-string)))))
         '(("fo" ("foo"))
           ("fo" ("foobar" "foobaz"))
           ("foo" ("foo" "foobar"))
           ("zz" ("alpha" "beta"))))"##;
    let expect = expect![[
        r#"OK ((("fo" #1=("foo")) nil sole #1# "fo") (("fo" #2=("foobar" "foobaz")) nil nil #2# "fo") (("foo" #3=("foo" "foobar")) nil nil #3# "foo") (("zz" #4=("alpha" "beta")) nil nil #4# "zz"))"#
    ]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_stub_advice_does_not_overwrite_existing_candidates() {
    let elisp_form = r##"(with-temp-buffer
         (insert "al")
         (let ((ac-pcmp--active-p t)
               (ac-pcmp--candidates '("first" "capture"))
               (ac-pcmp--status 'none))
           (list
            (pcomplete-stub "al" '("alpha" "alpine"))
            ac-pcmp--candidates
            ac-pcmp--status
            (buffer-string))))"##;
    let expect = expect![[r#"OK (nil ("first" "capture") nil "al")"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}

#[test]
fn auto_complete_pcmp_stub_advice_inactive_path_preserves_native_return_and_state() {
    let elisp_form = r##"(with-temp-buffer
         (insert "fo")
         (let ((ac-pcmp--active-p nil)
               (ac-pcmp--candidates '("outer"))
               (ac-pcmp--status 'outer)
               (pcomplete-termination-string " ")
               (pcomplete-suffix-list '(?/ ?:)))
           (list
            (pcomplete-stub "fo" '("foo"))
            ac-pcmp--candidates
            ac-pcmp--status
            (buffer-string)
            (point))))"##;
    let expect = expect![[r#"OK ((sole . "foo") ("outer") outer "fo" 3)"#]];
    assert_auto_complete_pcmp_parity(elisp_form, expect);
}
