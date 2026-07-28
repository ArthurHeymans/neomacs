use expect_test::expect;

use super::assert_attrap_parity;

#[test]
fn attrap_flymake_collects_configured_diagnostics_at_their_beginnings_in_source_order() {
    let elisp_form = r##"(with-temp-buffer
          (insert "alpha warning\nbeta warning\ngamma ignored\n")
          (let* ((diagnostics
                  '((:backend backend-a
                     :beg 3
                     :end 8
                     :text "first")
                    (:backend ignored-backend
                     :beg 29
                     :end 34
                     :text "ignored")
                    (:backend backend-b
                     :beg 17
                     :end 21
                     :text "second")))
                 (attrap-flymake-backends-alist
                  '((backend-a . attrap-test-fixer-a)
                    (backend-b . attrap-test-fixer-b)))
                 events
                 selected)
            (cl-letf
                (((symbol-function
                   'flymake-diagnostics)
                  (lambda (&optional beg end)
                    (push
                     (list
                      :diagnostics beg end)
                     events)
                    diagnostics))
                 ((symbol-function
                   'flymake-diagnostic-backend)
                  (lambda (diagnostic)
                    (plist-get
                     diagnostic
                     :backend)))
                 ((symbol-function
                   'flymake-diagnostic-beg)
                  (lambda (diagnostic)
                    (plist-get
                     diagnostic
                     :beg)))
                 ((symbol-function
                   'flymake-diagnostic-end)
                  (lambda (diagnostic)
                    (plist-get
                     diagnostic
                     :end)))
                 ((symbol-function
                   'flymake-diagnostic-text)
                  (lambda (diagnostic)
                    (plist-get
                     diagnostic
                     :text)))
                 ((symbol-function
                   'attrap-test-fixer-a)
                  (lambda (message beg end)
                    (push
                     (list
                      :fix-a
                      message
                      beg
                      end
                      (point))
                     events)
                    (list
                     (cons
                      '(repair first)
                      (lambda ()
                        :first)))))
                 ((symbol-function
                   'attrap-test-fixer-b)
                  (lambda (message beg end)
                    (push
                     (list
                      :fix-b
                      message
                      beg
                      end
                      (point))
                     events)
                    (list
                     nil
                     (cons
                      '(repair second)
                      (lambda ()
                        :second)))))
                 ((symbol-function
                   'attrap-select-and-apply-option)
                  (lambda (options)
                    (setq selected
                          (attrap-test-option-shape
                           options))
                    :selected)))
              (list
               (attrap-flymake 11)
               selected
               (nreverse events)
               (point)))))"##;
    let expect = expect![[
        r#"OK (:selected (((repair first) t) ((repair second) t)) ((:diagnostics 11 nil) (:fix-a "first" 3 8 3) (:fix-b "second" 17 21 17)) 17)"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_flymake_reports_missing_diagnostics_empty_repairs_and_fixer_errors_exactly() {
    let elisp_form = r##"(progn
          (defvar flyspell-mode nil)
          (defvar flymake-mode nil)
          (defvar flycheck-mode nil)
          (mapcar
          (lambda (scenario)
            (let ((attrap-flymake-backends-alist
                   '((fixture-backend
                      . attrap-test-flymake-fixer))))
              (cl-letf
                  (((symbol-function
                     'flymake-diagnostics)
                    (lambda (&optional _beg _end)
                      (unless
                          (eq scenario 'no-diagnostics)
                        '((:backend fixture-backend
                           :beg 1
                           :end 2
                           :text "fixture")))))
                   ((symbol-function
                     'flymake-diagnostic-backend)
                    (lambda (diagnostic)
                      (plist-get
                       diagnostic
                       :backend)))
                   ((symbol-function
                     'flymake-diagnostic-beg)
                    (lambda (diagnostic)
                      (plist-get
                       diagnostic
                       :beg)))
                   ((symbol-function
                     'flymake-diagnostic-end)
                    (lambda (diagnostic)
                      (plist-get
                       diagnostic
                       :end)))
                   ((symbol-function
                     'flymake-diagnostic-text)
                    (lambda (diagnostic)
                      (plist-get
                       diagnostic
                       :text)))
                   ((symbol-function
                     'attrap-test-flymake-fixer)
                    (lambda (&rest _arguments)
                      (pcase scenario
                        ('fixer-error
                         (error
                          "fixture fixer failed"))
                        ('empty-options nil)
                        (_
                         (attrap-one-option
                             'repair
                           :done))))))
                (list
                 scenario
                 (attrap-test-error-data
                  (lambda ()
                    (attrap-flymake 1)))))))
          '(no-diagnostics
            empty-options
            fixer-error
            success)))"##;
    let expect = expect![[
        r#"OK ((no-diagnostics (:error error ("No flymake diagnostic at point"))) (empty-options (:error error ("No fixer applies to the issue at point"))) (fixer-error (:error error ("fixture fixer failed"))) (success (:ok :done)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_flycheck_uses_real_overlay_ranges_and_all_matching_fixers_for_each_message() {
    let elisp_form = r##"(with-temp-buffer
          (insert
           "first diagnostic\nsecond diagnostic\n")
          (let* ((first
                  (make-overlay 2 7))
                 (ignored
                  (make-overlay 8 10))
                 (second
                  (make-overlay 19 25))
                 (attrap-flycheck-checkers-alist
                  '((fixture-checker
                     . attrap-test-flycheck-fixer-a)
                    (other-checker
                     . attrap-test-unexpected-fixer)
                    (fixture-checker
                     . attrap-test-flycheck-fixer-b)))
                 events
                 selected)
            (overlay-put
             first
             'flycheck-error
             'first-error)
            (overlay-put
             ignored
             'flycheck-error
             'ignored-error)
            (overlay-put
             second
             'flycheck-error
             'second-error)
            (cl-letf
                (((symbol-function
                   'flycheck-overlays-at)
                  (lambda (position)
                    (push
                     (list
                      :overlays-at
                      position)
                     events)
                    (list
                     first
                     ignored
                     second)))
                 ((symbol-function
                   'flycheck-error-message)
                  (lambda (error)
                    (pcase error
                      ('first-error
                       "first message")
                      ('second-error
                       "second message")
                      (_ nil))))
                 ((symbol-function
                   'flycheck-get-checker-for-buffer)
                  (lambda ()
                    'fixture-checker))
                 ((symbol-function
                   'attrap-test-flycheck-fixer-a)
                  (lambda (message beg end)
                    (push
                     (list
                      :fix-a
                      message
                      beg
                      end)
                     events)
                    (attrap-one-option
                        (list 'a message)
                      :a)))
                 ((symbol-function
                   'attrap-test-flycheck-fixer-b)
                  (lambda (message beg end)
                    (push
                     (list
                      :fix-b
                      message
                      beg
                      end)
                     events)
                    (attrap-one-option
                        (list 'b message)
                      :b)))
                 ((symbol-function
                   'attrap-select-and-apply-option)
                  (lambda (options)
                    (setq selected
                          (attrap-test-option-shape
                           options))
                    :selected)))
              (list
               (attrap-flycheck 23)
               selected
               (nreverse events)
               (mapcar
                (lambda (overlay)
                  (list
                   (overlay-start overlay)
                   (overlay-end overlay)))
                (list first ignored second))))))"##;
    let expect = expect![[
        r#"OK (:selected (((a "first message") t) ((b "first message") t) ((a "second message") t) ((b "second message") t)) ((:overlays-at 23) (:fix-a "first message" 2 7) (:fix-b "first message" 2 7) (:fix-a "second message" 19 25) (:fix-b "second message" 19 25)) ((2 7) (8 10) (19 25)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_flycheck_validates_messages_checker_and_fixer_configuration_in_order() {
    let elisp_form = r##"(mapcar
          (lambda (scenario)
            (let ((attrap-flycheck-checkers-alist
                   (unless
                       (eq scenario 'no-fixers)
                     '((fixture-checker
                        . attrap-test-fixer)))))
              (cl-letf
                  (((symbol-function
                     'flycheck-overlays-at)
                    (lambda (_position)
                      (unless
                          (eq scenario 'no-message)
                        (list
                         (list
                          :start 4
                          :end 9
                          :error 'fixture-error)))))
                   ((symbol-function
                     'overlay-get)
                    (lambda (overlay _property)
                      (plist-get
                       overlay
                       :error)))
                   ((symbol-function
                     'overlay-start)
                    (lambda (overlay)
                      (plist-get
                       overlay
                       :start)))
                   ((symbol-function
                     'overlay-end)
                    (lambda (overlay)
                      (plist-get
                       overlay
                       :end)))
                   ((symbol-function
                     'flycheck-error-message)
                    (lambda (_error)
                      "fixture message"))
                   ((symbol-function
                     'flycheck-get-checker-for-buffer)
                    (lambda ()
                      (unless
                          (eq scenario 'no-checker)
                        'fixture-checker)))
                   ((symbol-function
                     'attrap-test-fixer)
                    (lambda (&rest _arguments)
                      (if
                          (eq scenario 'fixer-error)
                          (error
                           "flycheck fixer failed")
                        (attrap-one-option
                            'repair
                          :done)))))
                (list
                 scenario
                 (attrap-test-error-data
                  (lambda ()
                    (attrap-flycheck 7)))))))
          '(no-message
            no-checker
            no-fixers
            fixer-error
            success))"##;
    let expect = expect![[
        r#"OK ((no-message (:error error ("No flycheck message at point"))) (no-checker (:error error ("No flycheck-checker for current buffer"))) (no-fixers (:error error ("No fixers for flycheck-checker fixture-checker"))) (fixer-error (:error error ("flycheck fixer failed"))) (success (:ok :done)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_entrypoint_prioritizes_flyspell_then_flymake_then_flycheck() {
    let elisp_form = r##"(progn
          (defvar flyspell-mode nil)
          (defvar flymake-mode nil)
          (defvar flycheck-mode nil)
          (with-temp-buffer
            (insert "misspelled")
            (let ((flyspell-mode t)
                  (flymake-mode t)
                  (flycheck-mode t)
                  events)
              (cl-letf
                  (((symbol-function
                     'overlays-at)
                    (lambda (position)
                      (push
                       (list
                        :overlays-at
                        position)
                       events)
                      '(:spelling-overlay)))
                   ((symbol-function
                     'flyspell-overlay-p)
                    (lambda (overlay)
                      (eq overlay
                          :spelling-overlay)))
                   ((symbol-function
                     'flyspell-correct-at-point)
                    (lambda ()
                      (push
                       :flyspell
                       events)
                      :spell-fixed))
                   ((symbol-function
                     'attrap-flymake)
                    (lambda (position)
                      (push
                       (list
                        :flymake
                        position)
                       events)
                      :flymake-fixed))
                   ((symbol-function
                     'attrap-flycheck)
                    (lambda (position)
                      (push
                       (list
                        :flycheck
                        position)
                       events)
                      :flycheck-fixed)))
                (list
                 (attrap-attrap 8)
                 (nreverse events))))))"##;
    let expect = expect!["OK (:spell-fixed ((:overlays-at 11) :flyspell))"];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_entrypoint_ignores_non_spelling_overlays_and_flyspell_without_overlay_api() {
    let elisp_form = r##"(progn
          (defvar flyspell-mode nil)
          (defvar flymake-mode nil)
          (defvar flycheck-mode nil)
          (mapcar
          (lambda (scenario)
            (with-temp-buffer
              (let ((flyspell-mode t)
                    (flymake-mode t)
                    flycheck-mode
                    events)
                (cl-letf
                    (((symbol-function
                       'overlays-at)
                      (lambda (_position)
                        '(:ordinary)))
                     ((symbol-function
                       'flyspell-overlay-p)
                      (if
                          (eq scenario 'no-api)
                          nil
                        (lambda (_overlay)
                          nil)))
                     ((symbol-function
                       'flyspell-correct-at-point)
                      (lambda ()
                        (push
                         :unexpected-spell
                         events)))
                     ((symbol-function
                       'attrap-flymake)
                      (lambda (position)
                        (push
                         (list
                          :flymake
                          position)
                         events)
                        :fixed)))
                  (list
                   scenario
                   (attrap-attrap 12)
                   (nreverse events))))))
           '(non-spelling no-api)))"##;
    let expect =
        expect!["OK ((non-spelling :fixed ((:flymake 12))) (no-api :fixed ((:flymake 12))))"];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_entrypoint_routes_to_flymake_before_flycheck_and_reports_when_neither_mode_is_active() {
    let elisp_form = r##"(progn
          (defvar flyspell-mode nil)
          (defvar flymake-mode nil)
          (defvar flycheck-mode nil)
          (list
           (let (flyspell-mode
                 (flymake-mode t)
                 (flycheck-mode t)
                 events)
             (cl-letf
                 (((symbol-function
                    'attrap-flymake)
                   (lambda (position)
                     (push
                      (list
                       :flymake
                       position)
                      events)
                     :flymake-fixed))
                  ((symbol-function
                    'attrap-flycheck)
                   (lambda (position)
                     (push
                      (list
                       :flycheck
                       position)
                      events)
                     :flycheck-fixed)))
               (list
                (attrap-attrap 21)
                (nreverse events))))
           (let (flyspell-mode
                 flymake-mode
                 (flycheck-mode t)
                 events)
             (cl-letf
                 (((symbol-function
                    'attrap-flymake)
                   (lambda (position)
                     (push
                      (list
                       :unexpected-flymake
                       position)
                      events)))
                  ((symbol-function
                    'attrap-flycheck)
                   (lambda (position)
                     (push
                      (list
                       :flycheck
                       position)
                      events)
                     :flycheck-fixed)))
               (list
                (attrap-attrap 34)
                (nreverse events))))
           (let (flyspell-mode
                 flymake-mode
                 flycheck-mode)
             (attrap-test-error-data
              (lambda ()
                (attrap-attrap 55))))))"##;
    let expect = expect![[
        r#"OK ((:flymake-fixed ((:flymake 21))) (:flycheck-fixed ((:flycheck 34))) (:error error ("Expecting flymake or flycheck to be active")))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}
