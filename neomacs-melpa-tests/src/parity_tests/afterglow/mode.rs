use expect_test::expect;

use super::assert_afterglow_parity;

#[test]
fn afterglow_mode_is_buffer_local_runs_hooks_and_controls_global_target_advice() {
    let elisp_form = r##"(let ((first-buffer
              (generate-new-buffer
               " *afterglow-mode-first*"))
             (second-buffer
              (generate-new-buffer
               " *afterglow-mode-second*"))
             (afterglow--triggers
              (make-hash-table
               :test 'equal))
             (afterglow--advised-functions nil)
             hook-calls
             overlay-calls)
         (unwind-protect
             (progn
               (fset
                'afterglow-test-mode-target
                (lambda (value)
                  (list
                   'target
                   value)))
               (puthash
                'afterglow-test-mode-target
                '(:thing word :duration 5)
                afterglow--triggers)
               (with-current-buffer first-buffer
                 (add-hook
                  'afterglow-mode-hook
                  (lambda ()
                    (push
                     (list
                      (buffer-name)
                      afterglow-mode)
                     hook-calls))
                  nil
                  t))
               (cl-letf
                   (((symbol-function
                      'afterglow--apply-overlay)
                     (lambda (properties)
                       (push
                        properties
                        overlay-calls))))
                 (with-current-buffer first-buffer
                   (afterglow-mode
                    1))
                 (let ((enabled-state
                        (list
                         (with-current-buffer first-buffer
                           (list
                            afterglow-mode
                            (local-variable-p
                             'afterglow-mode)
                            (assq
                             'afterglow-mode
                             minor-mode-alist)))
                         (with-current-buffer second-buffer
                           (list
                            afterglow-mode
                            (local-variable-p
                             'afterglow-mode)))
                         (advice-member-p
                          'afterglow--after-trigger-afterglow-test-mode-target
                          'afterglow-test-mode-target)
                         afterglow--advised-functions
                         (afterglow-test-mode-target
                          1)
                         (nreverse
                          overlay-calls))))
                   (setq overlay-calls
                         nil)
                   (with-current-buffer first-buffer
                     (afterglow-mode
                      -1))
                   (list
                    enabled-state
                    (afterglow-test-mode-target
                     2)
                    overlay-calls
                    (with-current-buffer first-buffer
                      (list
                       afterglow-mode
                       (local-variable-p
                        'afterglow-mode)))
                    (nreverse hook-calls)
                    (advice-member-p
                     'afterglow--after-trigger-afterglow-test-mode-target
                     'afterglow-test-mode-target)
                    afterglow--advised-functions))))
           (kill-buffer
            first-buffer)
           (kill-buffer
            second-buffer)))"##;
    let expect = expect![[
        r#"OK (((t t (afterglow-mode " afterglow")) (nil nil) #[128 "����\2\"����\3\"����" [afterglow--after-trigger-afterglow-test-mode-target #[(value) ((list 'target value)) (t)] :after nil apply] 5 advice] ((afterglow-test-mode-target . afterglow--after-trigger-afterglow-test-mode-target)) (target 1) ((:thing word :duration 5))) (target 2) nil (nil t) ((" *afterglow-mode-first*" t) (" *afterglow-mode-first*" nil)) nil nil)"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_mode_disable_removes_tagged_overlay_but_leaves_package_overlay_untagged() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta")
         (goto-char
          2)
         (let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               (afterglow--temp-overlay nil))
           (fset
            'afterglow-test-mode-overlay-target
            (lambda ()
              'target-result))
           (puthash
            'afterglow-test-mode-overlay-target
            '(:thing word :duration 90)
            afterglow--triggers)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (&rest _arguments)
                   'fixture-timer)))
             (afterglow-mode
              1)
             (afterglow-test-mode-overlay-target)
             (let ((package-overlay
                    afterglow--temp-overlay)
                   (tagged
                    (make-overlay
                     7
                     9)))
               (overlay-put
                tagged
                'afterglow
                t)
               (let ((result
                      (afterglow-mode
                       -1)))
                 (list
                  result
                  afterglow-mode
                  (overlay-buffer
                   package-overlay)
                  (overlay-start
                   package-overlay)
                  (overlay-end
                   package-overlay)
                  (overlay-get
                   package-overlay
                   'afterglow)
                  (overlay-buffer
                   tagged)
                  (overlays-in
                   (point-min)
                   (point-max))
                  (advice-member-p
                   'afterglow--after-trigger-afterglow-test-mode-overlay-target
                   'afterglow-test-mode-overlay-target)
                  afterglow--advised-functions))))))"##;
    let expect =
        expect!["OK (nil nil (:buffer nil) 1 6 nil nil (#<overlay in no buffer>) nil nil)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_enable_skips_missing_targets_and_is_idempotent_for_existing_targets() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (dolist (entry
                  '((afterglow-test-enable-a :thing word)
                    (afterglow-test-enable-b :thing line)
                    (afterglow-test-enable-missing :thing window)))
           (puthash
            (car entry)
            (cdr entry)
            afterglow--triggers))
         (dolist (symbol
                  '(afterglow-test-enable-a
                    afterglow-test-enable-b))
           (fset
            symbol
            `(lambda ()
               ',symbol)))
         (let ((first
                (afterglow--enable))
               (second
                (afterglow--enable)))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls))))
             (list
              first
              second
              (afterglow-test-enable-a)
              (afterglow-test-enable-b)
              (nreverse calls)
              (sort
               (mapcar
                (lambda (pair)
                  (list
                   (car pair)
                   (cdr pair)))
                afterglow--advised-functions)
               (lambda (left right)
                 (string<
                  (symbol-name (car left))
                  (symbol-name (car right)))))
              (mapcar
               #'fboundp
               '(afterglow--after-trigger-afterglow-test-enable-a
                 afterglow--after-trigger-afterglow-test-enable-b
                 afterglow--after-trigger-afterglow-test-enable-missing))
              (mapcar
               (lambda (pair)
                 (and
                  (advice-member-p
                   (cdr pair)
                   (car pair))
                  t))
               '((afterglow-test-enable-a
                  .
                  afterglow--after-trigger-afterglow-test-enable-a)
                 (afterglow-test-enable-b
                  .
                  afterglow--after-trigger-afterglow-test-enable-b)))))))"##;
    let expect = expect![
        "OK (nil nil afterglow-test-enable-a afterglow-test-enable-b ((:thing word) (:thing line)) ((afterglow-test-enable-a afterglow--after-trigger-afterglow-test-enable-a) (afterglow-test-enable-b afterglow--after-trigger-afterglow-test-enable-b)) (t t nil) (t t))"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_disable_is_idempotent_and_preserves_original_target_functions() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "abcdef")
         (let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil))
           (dolist (symbol
                    '(afterglow-test-disable-a
                      afterglow-test-disable-b))
             (fset
              symbol
              `(lambda ()
                 ',symbol))
             (puthash
              symbol
              (list
               :thing
               symbol)
              afterglow--triggers))
           (afterglow--enable)
           (let ((tagged
                  (make-overlay
                   1
                   3))
                 (untagged
                  (make-overlay
                   4
                   6)))
             (overlay-put
              tagged
              'afterglow
              t)
             (let ((first
                    (afterglow--disable))
                   (second
                    (afterglow--disable)))
               (list
                first
                second
                (overlay-buffer
                 tagged)
                (overlay-buffer
                 untagged)
                (overlay-start
                 untagged)
                (overlay-end
                 untagged)
                afterglow--advised-functions
                (mapcar
                 #'fboundp
                 '(afterglow-test-disable-a
                   afterglow-test-disable-b))
                (mapcar
                 #'fboundp
                 '(afterglow--after-trigger-afterglow-test-disable-a
                   afterglow--after-trigger-afterglow-test-disable-b))
                (list
                 (afterglow-test-disable-a)
                 (afterglow-test-disable-b)))))))"##;
    let expect = expect![
        "OK (nil nil nil (:buffer nil) 4 6 nil (t t) (nil nil) (afterglow-test-disable-a afterglow-test-disable-b))"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_reset_rebuilds_live_advice_keeps_triggers_and_removes_tagged_overlays() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "abcdef")
         (let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
           (fset
            'afterglow-test-reset-target
            (lambda ()
              'target-result))
           (puthash
            'afterglow-test-reset-target
            '(:thing sentence :duration 7)
            afterglow--triggers)
           (afterglow--enable)
           (let ((first-advice
                  (symbol-function
                   'afterglow--after-trigger-afterglow-test-reset-target))
                 (tagged
                  (make-overlay
                   2
                   5)))
             (overlay-put
              tagged
              'afterglow
              t)
             (let ((result
                    (afterglow--reset))
                   (second-advice
                    (symbol-function
                     'afterglow--after-trigger-afterglow-test-reset-target)))
               (cl-letf
                   (((symbol-function
                      'afterglow--apply-overlay)
                     (lambda (properties)
                       (push
                        properties
                        calls))))
                 (list
                  result
                  (eq
                   first-advice
                   second-advice)
                  (overlay-buffer
                   tagged)
                  (gethash
                   'afterglow-test-reset-target
                   afterglow--triggers)
                  (hash-table-count
                   afterglow--triggers)
                  (advice-member-p
                   'afterglow--after-trigger-afterglow-test-reset-target
                   'afterglow-test-reset-target)
                  afterglow--advised-functions
                  (afterglow-test-reset-target)
                  (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil nil nil #1=(:thing sentence :duration 7) 1 #[128 "����\2\"����\3\"����" [afterglow--after-trigger-afterglow-test-reset-target #[nil ('target-result) (t)] :after nil apply] 5 advice] ((afterglow-test-reset-target . afterglow--after-trigger-afterglow-test-reset-target)) target-result (#1#))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_mode_explicit_repeated_and_toggle_calls_run_hook_with_each_state() {
    let elisp_form = r##"(with-temp-buffer
         (let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               states)
           (add-hook
            'afterglow-mode-hook
            (lambda ()
              (push
               afterglow-mode
               states))
            nil
            t)
           (let ((first
                  (afterglow-mode
                   1))
                 (second
                  (afterglow-mode
                   1))
                 (third
                  (afterglow-mode
                   -1))
                 (fourth
                  (afterglow-mode)))
             (list
              first
              second
              third
              fourth
              afterglow-mode
              (nreverse states)
              (local-variable-p
               'afterglow-mode)
              afterglow--advised-functions))))"##;
    let expect = expect!["OK (t t nil t t (t t nil t) t nil)"];
    assert_afterglow_parity(elisp_form, expect);
}
