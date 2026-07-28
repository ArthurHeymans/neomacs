use expect_test::expect;

use super::assert_auto_package_update_parity;

#[test]
fn auto_package_update_now_refresh_decision_and_pipeline_order_cover_all_prompt_preview_modes() {
    let elisp_form = r##"(let (run-case)
                           (setq
                            run-case
                            (lambda
                                   (prompt preview async)
                                 (let
                                     ((auto-package-update-prompt-before-update
                                       prompt)
                                      (auto-package-update-show-preview
                                       preview)
                                      events)
                                   (let
                                       ((before
                                         (lambda ()
                                           (push
                                            :before
                                            events)))
                                        (after
                                         (lambda ()
                                           (push
                                            :after
                                            events))))
                                     (let
                                         ((auto-package-update-before-hook
                                           (list before))
                                          (auto-package-update-after-hook
                                           (list after)))
                                       (cl-letf
                                           (((symbol-function
                                              'package-refresh-contents)
                                             (lambda
                                                 (&optional async-argument)
                                               (push
                                                (list
                                                 :refresh
                                                 async-argument)
                                                events)))
                                            ((symbol-function
                                              'apu--packages-to-install)
                                             (lambda ()
                                               (push
                                                :select
                                                events)
                                               '(alpha beta)))
                                            ((symbol-function
                                              'apu--filter-quelpa-packages)
                                             (lambda (packages)
                                               (push
                                                (list
                                                 :filter
                                                 packages)
                                                events)
                                               '(alpha)))
                                            ((symbol-function
                                              'apu--safe-install-packages)
                                             (lambda (packages)
                                               (push
                                                (list
                                                 :install
                                                 packages)
                                                events)
                                               '("alpha up to date.")))
                                            ((symbol-function
                                              'apu--write-current-day)
                                             (lambda ()
                                               (push
                                                :write-day
                                                events)))
                                            ((symbol-function
                                              'apu--write-results-buffer)
                                             (lambda (contents)
                                               (push
                                                (list
                                                 :results
                                                 contents)
                                                events))))
                                         (list
                                          (auto-package-update-now
                                           async)
                                          (nreverse events))))))))
                           (list
                            (funcall run-case
                                     nil nil nil)
                            (funcall run-case
                                     t nil :async)
                            (funcall run-case
                                     nil t :async)
                            (funcall run-case
                                     t t :async)))"##;
    let expect = expect![[
        r#"OK ((nil (:before (:refresh nil) :select (:filter #1=(alpha beta)) (:install #2=(alpha)) :write-day (:results "[PACKAGES UPDATED]:\nalpha up to date.") :after)) (nil (:before (:refresh :async) :select (:filter #1#) (:install #2#) :write-day (:results "[PACKAGES UPDATED]:\nalpha up to date.") :after)) (nil (:before (:refresh :async) :select (:filter #1#) (:install #2#) :write-day (:results "[PACKAGES UPDATED]:\nalpha up to date.") :after)) (nil (:before :select (:filter #1#) (:install #2#) :write-day (:results "[PACKAGES UPDATED]:\nalpha up to date.") :after)))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_practical_update_selects_only_outdated_downloads_and_cleans_old_version() {
    let elisp_form = r##"(let*
                             ((root
                               (auto-package-update-test-root
                                "practical-update"))
                              (alpha-old-dir
                               (auto-package-update-test-path
                                root
                                "elpa/alpha-1/"))
                              (alpha-old
                               (auto-package-update-test-desc
                                'alpha
                                '(1 0)
                                nil
                                alpha-old-dir))
                              (alpha-new
                               (auto-package-update-test-desc
                                'alpha
                                '(2 0)
                                '((dep (1 0)))))
                              (beta
                               (auto-package-update-test-desc
                                'beta
                                '(3 0)))
                              (gamma-old
                               (auto-package-update-test-desc
                                'gamma
                                '(1 0)))
                              (gamma-new
                               (auto-package-update-test-desc
                                'gamma
                                '(4 0)))
                              (package-alist
                               `((alpha ,alpha-old)
                                 (beta ,beta)
                                 (gamma ,gamma-old)))
                              (package-archive-contents
                               `((alpha ,alpha-new)
                                 (beta ,beta)
                                 (gamma ,gamma-new)))
                              (package--builtins nil)
                              (package-activated-list
                               '(alpha beta gamma))
                              (auto-package-update-excluded-packages
                               '(gamma))
                              (auto-package-update-delete-old-versions
                               t)
                              (auto-package-update-hide-results
                               t)
                              (auto-package-update-buffer-name
                               " *apu-practical-results*")
                              (auto-package-update-last-update-day-path
                               (auto-package-update-test-path
                                root
                                "state/last-day"))
                              events
                              downloads
                              refreshes)
                           (auto-package-update-test-write
                            (auto-package-update-test-path
                             alpha-old-dir
                             "alpha.el")
                            "old alpha")
                           (make-directory
                            (file-name-directory
                             auto-package-update-last-update-day-path)
                            t)
                           (let
                               ((before
                                 (lambda ()
                                   (push :before events)))
                                (after
                                 (lambda ()
                                   (push :after events))))
                             (let
                                 ((auto-package-update-before-hook
                                   (list before))
                                  (auto-package-update-after-hook
                                   (list after)))
                               (unwind-protect
                                   (cl-letf
                                       (((symbol-function
                                          'package-installed-p)
                                         (lambda (package)
                                           (memq
                                            package
                                            '(alpha beta gamma))))
                                        ((symbol-function
                                          'package-refresh-contents)
                                         (lambda
                                             (&optional async)
                                           (push
                                            async
                                            refreshes)
                                           :refreshed))
                                        ((symbol-function
                                          'package-compute-transaction)
                                         (lambda
                                             (packages requirements)
                                           (push
                                            (list
                                             :compute
                                             (mapcar
                                              #'package-desc-name
                                              packages)
                                             requirements)
                                            downloads)
                                           packages))
                                        ((symbol-function
                                          'package-download-transaction)
                                         (lambda (transaction)
                                           (push
                                            (list
                                             :download
                                             (mapcar
                                              #'package-desc-name
                                              transaction))
                                            downloads)
                                           :downloaded))
                                        ((symbol-function
                                          'apu--today-day)
                                         (lambda () 4321)))
                                     (let ((result
                                            (auto-package-update-now)))
                                       (with-current-buffer
                                           auto-package-update-buffer-name
                                         (list
                                          result
                                          (nreverse refreshes)
                                          (nreverse downloads)
                                          (nreverse events)
                                          (auto-package-update-test-read
                                           auto-package-update-last-update-day-path)
                                          (buffer-string)
                                          buffer-read-only
                                          auto-package-update-minor-mode
                                          (file-exists-p
                                           alpha-old-dir)
                                          apu--old-versions-dirs-list))))
                                 (auto-package-update-test-kill-buffers
                                  auto-package-update-buffer-name)))))"##;
    let expect = expect![[
        r#"OK (nil (nil) ((:compute (alpha) ((dep (1 0)))) (:download (alpha))) (:before :after) "4321" "[PACKAGES UPDATED]:\nalpha up to date." t t nil nil)"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_prompt_preview_workflow_refreshes_once_prompts_hides_then_updates() {
    let elisp_form = r##"(let
                             ((auto-package-update-prompt-before-update
                               t)
                              (auto-package-update-show-preview
                               t)
                              (auto-package-preview-buffer-name
                               " *apu-workflow-preview*")
                              (auto-package-update-last-update-day-path
                               "/fixture/missing-day")
                              refreshes
                              events)
                           (unwind-protect
                               (cl-letf
                                   (((symbol-function
                                      'file-exists-p)
                                     (lambda (_file) nil))
                                    ((symbol-function
                                      'package-refresh-contents)
                                     (lambda (&rest arguments)
                                       (push arguments refreshes)
                                       :refreshed))
                                    ((symbol-function
                                      'apu--packages-to-install)
                                     (lambda ()
                                       '(alpha beta)))
                                    ((symbol-function
                                      'y-or-n-p)
                                     (lambda (question)
                                       (push
                                        (list :prompt question)
                                        events)
                                       t))
                                    ((symbol-function
                                      'apu--hide-preview)
                                     (lambda ()
                                       (push :hide events)
                                       (auto-package-update-test-kill-buffers
                                        auto-package-preview-buffer-name)))
                                    ((symbol-function
                                      'apu--safe-install-packages)
                                     (lambda (packages)
                                       (push
                                        (list
                                         :install
                                         packages)
                                        events)
                                       '("alpha done"
                                         "beta done")))
                                    ((symbol-function
                                      'apu--write-current-day)
                                     (lambda ()
                                       (push :write-day events)))
                                    ((symbol-function
                                      'apu--write-results-buffer)
                                     (lambda (contents)
                                       (push
                                        (list
                                         :results
                                         contents)
                                        events))))
                                 (list
                                  (auto-package-update-maybe)
                                  (nreverse refreshes)
                                  (nreverse events)
                                  (get-buffer
                                   auto-package-preview-buffer-name)))
                             (auto-package-update-test-kill-buffers
                              auto-package-preview-buffer-name)))"##;
    let expect = expect![[
        r#"OK (nil (nil) ((:prompt "Auto-update packages now?") :hide (:install (alpha beta)) :write-day (:results "[PACKAGES UPDATED]:\nalpha done\nbeta done")) nil)"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_now_reports_each_success_and_failure_in_real_results_buffer() {
    let elisp_form = r##"(let*
                             ((good
                               (auto-package-update-test-desc
                                'good
                                '(2 0)))
                              (bad
                               (auto-package-update-test-desc
                                'bad
                                '(2 0)))
                              (package-archive-contents
                               `((good ,good)
                                 (bad ,bad)))
                              (auto-package-update-buffer-name
                               " *apu-mixed-results*")
                              (auto-package-update-hide-results
                               t)
                              events)
                           (unwind-protect
                               (cl-letf
                                   (((symbol-function
                                      'package-refresh-contents)
                                     (lambda (&rest arguments)
                                       (push
                                        (list
                                         :refresh
                                         arguments)
                                        events)))
                                    ((symbol-function
                                      'apu--packages-to-install)
                                     (lambda ()
                                       '(good bad missing)))
                                    ((symbol-function
                                      'package-compute-transaction)
                                     (lambda
                                         (packages _requirements)
                                       packages))
                                    ((symbol-function
                                      'package-download-transaction)
                                     (lambda (transaction)
                                       (let ((name
                                              (package-desc-name
                                               (car transaction))))
                                         (push
                                          (list
                                           :download
                                           name)
                                          events)
                                         (when
                                             (eq name 'bad)
                                           (error
                                            "broken archive")))))
                                    ((symbol-function
                                      'apu--write-current-day)
                                     (lambda ()
                                       (push
                                        :write-day
                                        events))))
                                 (auto-package-update-now)
                                 (with-current-buffer
                                     auto-package-update-buffer-name
                                   (list
                                    (nreverse events)
                                    (buffer-string)
                                    buffer-read-only
                                    auto-package-update-minor-mode)))
                             (auto-package-update-test-kill-buffers
                              auto-package-update-buffer-name)))"##;
    let expect = expect![[
        r#"OK (((:refresh (nil)) (:download good) (:download bad) :write-day) "[PACKAGES UPDATED]:\nError installing missing\nError installing bad\ngood up to date." t t)"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_refresh_error_aborts_before_selection_day_results_and_after_hook() {
    let elisp_form = r##"(let (events)
                           (let
                               ((before
                                 (lambda ()
                                   (push :before events)))
                                (after
                                 (lambda ()
                                   (push :after events))))
                             (let
                                 ((auto-package-update-before-hook
                                   (list before))
                                  (auto-package-update-after-hook
                                   (list after)))
                               (cl-letf
                                   (((symbol-function
                                      'package-refresh-contents)
                                     (lambda (&rest _arguments)
                                       (push :refresh events)
                                       (error
                                        "offline fixture")))
                                    ((symbol-function
                                      'apu--packages-to-install)
                                     (lambda ()
                                       (push :select events)
                                       nil))
                                    ((symbol-function
                                      'apu--write-current-day)
                                     (lambda ()
                                       (push :write-day events)))
                                    ((symbol-function
                                      'apu--write-results-buffer)
                                     (lambda (_contents)
                                       (push :results events))))
                                 (list
                                  (auto-package-update-test-error
                                   #'auto-package-update-now)
                                  (nreverse events))))))"##;
    let expect = expect![[r#"OK ((:signal error ("offline fixture")) (:before :refresh))"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_async_creates_named_thread_whose_body_runs_async_update() {
    let elisp_form = r##"(let
                             ((apu--update-thread nil)
                              captured-function
                              captured-name
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'make-thread)
                                 (lambda (function name)
                                   (setq
                                    captured-function
                                    function
                                    captured-name
                                    name)
                                   'fixture-thread))
                                ((symbol-function
                                  'auto-package-update-now)
                                 (lambda (&optional async)
                                   (push async calls)
                                   :updated)))
                             (let ((result
                                    (auto-package-update-now-async)))
                               (list
                                result
                                apu--update-thread
                                captured-name
                                (functionp
                                 captured-function)
                                (funcall
                                 captured-function)
                                (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (fixture-thread fixture-thread "auto-package-update-now-async" t :updated (:async))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_async_rejects_second_live_thread_without_force() {
    let elisp_form = r##"(let
                             ((apu--update-thread
                               'existing-thread)
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'thread-live-p)
                                 (lambda (thread)
                                   (push
                                    (list :live thread)
                                    calls)
                                   t))
                                ((symbol-function
                                  'make-thread)
                                 (lambda (&rest arguments)
                                   (push
                                    (list :make arguments)
                                    calls)
                                   'unexpected-thread))
                                ((symbol-function
                                  'thread-signal)
                                 (lambda (&rest arguments)
                                   (push
                                    (list :signal arguments)
                                    calls))))
                             (list
                              (auto-package-update-test-error
                               #'auto-package-update-now-async)
                              apu--update-thread
                              (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:signal error ("auto-package-update thread is still running.")) existing-thread ((:live existing-thread)))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_async_force_signals_live_thread_clears_and_replaces_it() {
    let elisp_form = r##"(let
                             ((apu--update-thread
                               'existing-thread)
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'thread-live-p)
                                 (lambda (thread)
                                   (push
                                    (list :live thread)
                                    calls)
                                   (eq
                                    thread
                                    'existing-thread)))
                                ((symbol-function
                                  'thread-signal)
                                 (lambda
                                     (thread error-symbol data)
                                   (push
                                    (list
                                     :signal
                                     thread
                                     error-symbol
                                     data)
                                    calls)
                                   :signaled))
                                ((symbol-function
                                  'make-thread)
                                 (lambda (function name)
                                   (push
                                    (list
                                     :make
                                     (functionp function)
                                     name)
                                    calls)
                                   'replacement-thread)))
                             (list
                              (auto-package-update-now-async
                               t)
                              apu--update-thread
                              (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (replacement-thread replacement-thread ((:live existing-thread) (:signal existing-thread nil nil) (:make t "auto-package-update-now-async")))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}
