use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_auth_source_provider_handles_function_string_missing_and_invalid_secrets() {
    let elisp_form = r##"(let (queries)
         (cl-letf (((symbol-function 'auth-source-search)
                    (lambda (&rest arguments)
                      (push arguments queries)
                      (get 'advent-test 'auth-result))))
           (list
            (progn
              (put 'advent-test 'auth-result
                   (list
                    (list :host "adventofcode.com"
                          :user "session"
                          :secret (lambda () "from-function"))))
              (advent-session-from-auth-source))
            (progn
              (put 'advent-test 'auth-result
                   (list (list :secret "from-string")))
              (advent-session-from-auth-source))
            (progn
              (put 'advent-test 'auth-result
                   (list (list :secret 42)))
              (advent-session-from-auth-source))
            (progn
              (put 'advent-test 'auth-result nil)
              (advent-session-from-auth-source))
            (nreverse queries))))"##;
    let expect = expect![[
        r#"OK ("from-function" "from-string" nil nil ((:host "adventofcode.com" :user "session" :require #1=(:secret) :max 1) (:host "adventofcode.com" :user "session" :require #1# :max 1) (:host "adventofcode.com" :user "session" :require #1# :max 1) (:host "adventofcode.com" :user "session" :require #1# :max 1)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_session_prompt_and_provider_adapter_preserve_exact_values_and_calls() {
    let elisp_form = r##"(let (prompts provider-calls)
         (cl-letf (((symbol-function 'read-string)
                    (lambda (&rest arguments)
                      (push arguments prompts)
                      " prompted cookie ")))
           (list
            (advent-session-prompt)
            (let ((advent-session-provider
                   (lambda ()
                     (push 'called provider-calls)
                     "provider-cookie")))
              (advent--cookie-get))
            (let ((advent-session-provider
                   (lambda ()
                     (push 'nil-result provider-calls)
                     nil)))
              (advent--cookie-get))
            (let ((advent-session-provider nil))
              (advent--cookie-get))
            (nreverse prompts)
            (nreverse provider-calls))))"##;
    let expect = expect![[
        r#"OK (" prompted cookie " "provider-cookie" nil nil (("Advent of Code session cookie: ")) (called nil-result))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_cookie_lookup_selects_session_cookie_and_checks_expiration() {
    let elisp_form = r##"(let* ((other
                (url-cookie-create
                 :name "other"
                 :value "other-value"
                 :expires "Fri, 25 Dec 2031 00:00:00 GMT"
                 :domain ".adventofcode.com"
                 :localpart "/"
                 :secure t))
               (session-live
                (url-cookie-create
                 :name "session"
                 :value "live-value"
                 :expires "Fri, 25 Dec 2031 00:00:00 GMT"
                 :domain ".adventofcode.com"
                 :localpart "/"
                 :secure t))
               (session-expired
                (url-cookie-create
                 :name "session"
                 :value "expired-value"
                 :expires "Fri, 25 Dec 2020 00:00:00 GMT"
                 :domain ".adventofcode.com"
                 :localpart "/"
                 :secure t))
               retrieve-calls)
         (cl-letf (((symbol-function 'url-cookie-retrieve)
                    (lambda (&rest arguments)
                      (push arguments retrieve-calls)
                      (get 'advent-test 'cookies))))
           (list
            (progn
              (put 'advent-test 'cookies
                   (list other session-live session-expired))
              (advent--cookie-ok-p))
            (progn
              (put 'advent-test 'cookies
                   (list other session-expired))
              (advent--cookie-ok-p))
            (progn
              (put 'advent-test 'cookies nil)
              (advent--cookie-ok-p))
            (nreverse retrieve-calls)
            (mapcar
             (lambda (cookie)
               (list (url-cookie-name cookie)
                     (url-cookie-value cookie)
                     (url-cookie-expired-p cookie)))
             (list other session-live session-expired)))))"##;
    let expect = expect![[
        r#"OK (t nil nil ((".adventofcode.com" "/" t) (".adventofcode.com" "/" t) (".adventofcode.com" "/" t)) (("other" "other-value" nil) ("session" "live-value" nil) ("session" "expired-value" t)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_cookie_guard_covers_present_login_and_declined_workflows() {
    let elisp_form = r##"(let (events cookie-ok answer)
         (cl-letf (((symbol-function 'advent--cookie-ok-p)
                    (lambda () cookie-ok))
                   ((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push (list 'prompt prompt) events)
                      answer))
                   ((symbol-function 'advent-login)
                    (lambda (&optional session)
                      (push (list 'login session) events)
                      'logged-in)))
           (list
            (progn
              (setq cookie-ok t answer nil)
              (advent--ensure-cookie-or-error))
            (progn
              (setq cookie-ok nil answer t)
              (advent--ensure-cookie-or-error))
            (progn
              (setq cookie-ok nil answer nil)
              (condition-case error-data
                  (advent--ensure-cookie-or-error)
                (error
                 (list 'signal
                       (car error-data)
                       (cdr error-data)))))
            (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil logged-in (signal user-error ("No AoC session cookie set; run M-x advent-login")) ((prompt "AoC session cookie missing.  Set it now? ") (login nil) (prompt "AoC session cookie missing.  Set it now? ")))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_login_exercises_explicit_provider_and_prompt_cookie_workflows() {
    let elisp_form = r##"(let (stored refreshed messages prompt-calls provider-calls)
         (cl-letf (((symbol-function 'url-cookie-store)
                    (lambda (&rest arguments)
                      (push arguments stored)
                      'stored))
                   ((symbol-function 'advent--refresh-mode-lines)
                    (lambda ()
                      (push 'refresh refreshed)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages)))
                   ((symbol-function 'advent-session-prompt)
                    (lambda ()
                      (push 'prompt prompt-calls)
                      "prompt-cookie")))
           (let ((advent-session-provider
                  (lambda ()
                    (push 'provider provider-calls)
                    "provider-cookie")))
             (advent-login "explicit-cookie")
             (advent-login nil))
           (let ((advent-session-provider
                  (lambda ()
                    (push 'empty-provider provider-calls)
                    nil)))
             (advent-login nil))
           (list
            (nreverse stored)
            (nreverse refreshed)
            (nreverse messages)
            (nreverse provider-calls)
            (nreverse prompt-calls))))"##;
    let expect = expect![[
        r#"OK ((("session" "explicit-cookie" "Fri, 25 Dec 2031 00:00:00 GMT" ".adventofcode.com" "/" t) ("session" "provider-cookie" "Fri, 25 Dec 2031 00:00:00 GMT" ".adventofcode.com" "/" t) ("session" "prompt-cookie" "Fri, 25 Dec 2031 00:00:00 GMT" ".adventofcode.com" "/" t)) (refresh refresh refresh) ("AoC session cookie stored." "AoC session cookie stored." "AoC session cookie stored.") (provider empty-provider) (prompt))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_login_stores_a_real_retrievable_cookie_and_satisfies_the_guard() {
    let elisp_form = r##"(let ((url-cookie-storage nil)
               (url-cookie-secure-storage nil)
               refreshes messages prompts phase)
         (cl-letf (((symbol-function 'advent--refresh-mode-lines)
                    (lambda ()
                      (push 'refresh refreshes)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (list
                             phase
                             (apply #'format format-string arguments))
                            messages)))
                   ((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push prompt prompts)
                      nil)))
           (setq phase 'login)
           (advent-login "real-session-token")
           (setq phase 'retrieve)
           (let ((cookies
                  (url-cookie-retrieve
                   ".adventofcode.com"
                   "/"
                   t)))
             (list
              (mapcar
               (lambda (cookie)
                 (list
                  (url-cookie-name cookie)
                  (url-cookie-value cookie)
                  (url-cookie-expires cookie)
                  (url-cookie-domain cookie)
                  (url-cookie-localpart cookie)
                  (url-cookie-secure cookie)
                  (url-cookie-expired-p cookie)))
               cookies)
              (progn
                (setq phase 'cookie-ok)
                (advent--cookie-ok-p))
              (progn
                (setq phase 'cookie-status)
                (advent--cookie-status-string))
              (progn
                (setq phase 'cookie-guard)
                (advent--ensure-cookie-or-error))
              (nreverse refreshes)
              (nreverse messages)
              (nreverse prompts)))))"##;
    let expect = expect![[
        r#"OK ((("session" "real-session-token" "Fri, 25 Dec 2031 00:00:00 GMT" ".adventofcode.com" "/" t nil)) t "✓" nil (refresh) ((login "AoC session cookie stored.")) nil)"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_mode_line_refresh_touches_only_enabled_buffers_and_requests_all_frames() {
    let elisp_form = r##"(let ((enabled (generate-new-buffer " *advent-enabled*"))
               (disabled (generate-new-buffer " *advent-disabled*"))
               calls)
         (unwind-protect
             (progn
               (with-current-buffer enabled
                 (setq-local advent-mode t))
               (with-current-buffer disabled
                 (setq-local advent-mode nil))
               (cl-letf (((symbol-function 'buffer-list)
                          (lambda () (list enabled disabled)))
                         ((symbol-function 'force-mode-line-update)
                          (lambda (&optional all)
                            (push (list (buffer-name) all) calls))))
                 (advent--refresh-mode-lines))
               (nreverse calls))
           (kill-buffer enabled)
           (kill-buffer disabled)))"##;
    let expect = expect![[r#"OK ((" *advent-enabled*" t))"#]];
    assert_advent_mode_parity(elisp_form, expect);
}
