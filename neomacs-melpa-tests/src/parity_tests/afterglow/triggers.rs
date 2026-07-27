use expect_test::expect;

use super::assert_afterglow_parity;

#[test]
fn afterglow_internal_trigger_storage_replaces_properties_and_lists_exact_keys() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil))
         (list
          (afterglow--add-trigger
           'afterglow-test-beta
           '(:thing word :duration 2))
          (afterglow--add-trigger
           'afterglow-test-alpha
           '(:thing line :width 4))
          (afterglow--add-trigger
           'afterglow-test-beta
           '(:thing sentence :face highlight))
          (sort
           (afterglow--trigger-functions)
           (lambda (left right)
             (string<
              (symbol-name left)
              (symbol-name right))))
          (gethash
           'afterglow-test-alpha
           afterglow--triggers)
          (gethash
           'afterglow-test-beta
           afterglow--triggers)
          (hash-table-count
           afterglow--triggers)))"##;
    let expect = expect![
        "OK ((:thing word :duration 2) #1=(:thing line :width 4) #2=(:thing sentence :face highlight) (afterglow-test-alpha afterglow-test-beta) #1# #2# 2)"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_internal_add_trigger_prunes_stale_advice_before_storing_new_entry() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil))
         (fset
          'afterglow-test-stale
          (lambda ()
            'stale-result))
         (afterglow--advice-add
          'afterglow-test-stale
          (afterglow--advice-fn-symbol
           'afterglow-test-stale))
         (remhash
          'afterglow-test-stale
          afterglow--triggers)
         (let ((result
                (afterglow--add-trigger
                 'afterglow-test-fresh
                 '(:thing word))))
           (list
            result
            (hash-table-count
             afterglow--triggers)
            (gethash
             'afterglow-test-fresh
             afterglow--triggers)
            (fboundp
             'afterglow--after-trigger-afterglow-test-stale)
            (advice-member-p
             'afterglow--after-trigger-afterglow-test-stale
             'afterglow-test-stale)
            afterglow--advised-functions
            (afterglow-test-stale))))"##;
    let expect = expect!["OK (#1=(:thing word) 1 #1# nil nil nil stale-result)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_add_trigger_advises_a_real_command_and_highlights_its_result_word() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta gamma")
         (goto-char
          (point-min))
         (fset
          'afterglow-test-move-to-beta
          (lambda ()
            (search-forward
             "beta")
            (backward-word)
            'moved))
         (let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               (afterglow--temp-overlay nil)
               timer-call)
           (cl-letf
               (((symbol-function
                  'run-with-timer)
                 (lambda (duration repeat callback &rest arguments)
                   (setq timer-call
                         (list
                          duration
                          repeat
                          (functionp callback)
                          arguments))
                   'fixture-timer)))
             (afterglow-add-trigger
              'afterglow-test-move-to-beta
              :thing 'word
              :duration 12
              :face 'highlight)
             (let ((result
                    (afterglow-test-move-to-beta))
                   (advice-symbol
                    (afterglow--advice-fn-symbol
                     'afterglow-test-move-to-beta)))
               (list
                result
                (point)
                (gethash
                 'afterglow-test-move-to-beta
                 afterglow--triggers)
                (advice-member-p
                 advice-symbol
                 'afterglow-test-move-to-beta)
                (overlay-start
                 afterglow--temp-overlay)
                (overlay-end
                 afterglow--temp-overlay)
                (buffer-substring
                 (overlay-start afterglow--temp-overlay)
                 (overlay-end afterglow--temp-overlay))
                (overlay-get
                 afterglow--temp-overlay
                 'face)
                (overlay-get
                 afterglow--temp-overlay
                 'priority)
                timer-call)))))"##;
    let expect = expect![[
        r#"OK (moved 7 (:thing word :duration 12 :face highlight) #[128 "����\2\"����\3\"����" [afterglow--after-trigger-afterglow-test-move-to-beta #[nil ((search-forward "beta") (backward-word) 'moved) (t)] :after nil apply] 5 advice] 7 11 "beta" highlight 100 (12 nil t nil))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_enable_later_advises_a_trigger_whose_function_was_initially_missing() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (afterglow-add-trigger
          'afterglow-test-late-target
          :thing 'sentence
          :duration 3)
         (let ((before
                (list
                 (gethash
                  'afterglow-test-late-target
                  afterglow--triggers)
                 (fboundp
                  'afterglow--after-trigger-afterglow-test-late-target)
                 afterglow--advised-functions)))
           (fset
            'afterglow-test-late-target
            (lambda (value)
              (list
               'late-result
               value)))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls))))
             (afterglow--enable)
             (list
              before
              (afterglow-test-late-target
               9)
              (nreverse calls)
              (advice-member-p
               'afterglow--after-trigger-afterglow-test-late-target
               'afterglow-test-late-target)
              afterglow--advised-functions))))"##;
    let expect = expect![[
        r#"OK ((#1=(:thing sentence :duration 3) nil nil) (late-result 9) (#1#) #[128 "����\2\"����\3\"����" [afterglow--after-trigger-afterglow-test-late-target #[(value) ((list 'late-result value)) (t)] :after nil apply] 5 advice] ((afterglow-test-late-target . afterglow--after-trigger-afterglow-test-late-target)))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_add_triggers_batches_existing_missing_and_replaced_entries() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (fset
          'afterglow-test-batch-a
          (lambda ()
            'result-a))
         (fset
          'afterglow-test-batch-b
          (lambda ()
            'result-b))
         (cl-letf
             (((symbol-function
                'afterglow--apply-overlay)
               (lambda (properties)
                 (push
                  properties
                  calls))))
           (let ((result
                  (afterglow-add-triggers
                   '((afterglow-test-batch-a
                      :thing word
                      :duration 1)
                     (afterglow-test-missing
                      :thing window)
                     (afterglow-test-batch-b
                      :thing line
                      :width 8)
                     (afterglow-test-batch-a
                      :thing sentence
                      :face highlight)))))
             (list
              result
              (afterglow-test-batch-a)
              (afterglow-test-batch-b)
              (nreverse calls)
              (mapcar
               (lambda (symbol)
                 (cons
                  symbol
                  (gethash
                   symbol
                   afterglow--triggers)))
               (sort
                (afterglow--trigger-functions)
                (lambda (left right)
                  (string<
                   (symbol-name left)
                   (symbol-name right)))))
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
                  (symbol-name (car right)))))))))"##;
    let expect = expect![
        "OK (nil result-a result-b (#1=(:thing sentence :face highlight) #2=(:thing line :width 8)) ((afterglow-test-batch-a . #1#) (afterglow-test-batch-b . #2#) (afterglow-test-missing :thing window)) ((afterglow-test-batch-a afterglow--after-trigger-afterglow-test-batch-a) (afterglow-test-batch-b afterglow--after-trigger-afterglow-test-batch-b)))"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_remove_trigger_drops_properties_advice_and_generated_function_only() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (fset
          'afterglow-test-remove-one
          (lambda ()
            'target-result))
         (cl-letf
             (((symbol-function
                'afterglow--apply-overlay)
               (lambda (properties)
                 (push
                  properties
                  calls))))
           (afterglow-add-trigger
            'afterglow-test-remove-one
            :thing 'line)
           (let ((before
                  (afterglow-test-remove-one))
                 (result
                  (afterglow-remove-trigger
                   'afterglow-test-remove-one)))
             (list
              before
              result
              (afterglow-test-remove-one)
              (nreverse calls)
              (gethash
               'afterglow-test-remove-one
               afterglow--triggers
               'absent)
              (fboundp
               'afterglow--after-trigger-afterglow-test-remove-one)
              (advice-member-p
               'afterglow--after-trigger-afterglow-test-remove-one
               'afterglow-test-remove-one)
              afterglow--advised-functions
              (fboundp
               'afterglow-test-remove-one)))))"##;
    let expect =
        expect!["OK (target-result nil target-result ((:thing line)) absent nil nil nil t)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_remove_triggers_handles_present_and_absent_names_while_leaving_others_live() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (dolist (symbol
                  '(afterglow-test-remove-a
                    afterglow-test-remove-b
                    afterglow-test-remove-c))
           (fset
            symbol
            `(lambda ()
               ',symbol)))
         (afterglow-add-triggers
          '((afterglow-test-remove-a :thing word)
            (afterglow-test-remove-b :thing line)
            (afterglow-test-remove-c :thing sentence)))
         (let ((result
                (afterglow-remove-triggers
                 '(afterglow-test-remove-a
                   afterglow-test-never-added
                   afterglow-test-remove-b))))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls))))
             (list
              result
              (afterglow-test-remove-a)
              (afterglow-test-remove-b)
              (afterglow-test-remove-c)
              (nreverse calls)
              (afterglow--trigger-functions)
              (gethash
               'afterglow-test-remove-c
               afterglow--triggers)
              afterglow--advised-functions
              (mapcar
               #'fboundp
               '(afterglow-test-remove-a
                 afterglow-test-remove-b
                 afterglow-test-remove-c))))))"##;
    let expect = expect![
        "OK (nil afterglow-test-remove-a afterglow-test-remove-b afterglow-test-remove-c (#1=(:thing sentence)) (afterglow-test-remove-c) #1# ((afterglow-test-remove-c . afterglow--after-trigger-afterglow-test-remove-c)) (t t t))"
    ];
    assert_afterglow_parity(elisp_form, expect);
}
