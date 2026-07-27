use expect_test::expect;

use super::assert_afterglow_parity;

#[test]
fn afterglow_advice_function_symbol_derives_interned_names_for_varied_symbols() {
    let elisp_form = r##"(let* ((uninterned
                  (make-symbol
                   "private-target"))
                 (plain
                  (afterglow--advice-fn-symbol
                   'move))
                 (qualified
                  (afterglow--advice-fn-symbol
                   'fixture/move-next))
                 (private
                  (afterglow--advice-fn-symbol
                   uninterned)))
         (list
          plain
          qualified
          private
          (intern-soft
           "afterglow--after-trigger-private-target")
          (eq
           private
           (intern
            "afterglow--after-trigger-private-target"))
          (eq
           uninterned
           private)))"##;
    let expect = expect![
        "OK (afterglow--after-trigger-move afterglow--after-trigger-fixture/move-next afterglow--after-trigger-private-target afterglow--after-trigger-private-target t nil)"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advice_add_installs_once_tracks_once_and_forwards_original_results() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (fset
          'afterglow-test-advice-target
          (lambda (left right)
            (list
             'original
             left
             right)))
         (puthash
          'afterglow-test-advice-target
          '(:thing word :duration 4)
          afterglow--triggers)
         (let* ((advice-symbol
                 (afterglow--advice-fn-symbol
                  'afterglow-test-advice-target))
                (first
                 (afterglow--advice-add
                  'afterglow-test-advice-target
                  advice-symbol))
                (second
                 (afterglow--advice-add
                  'afterglow-test-advice-target
                  advice-symbol)))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls)
                   'overlay-result)))
             (list
              (and
               (consp first)
               (eq
                (caar first)
                'afterglow-test-advice-target)
               (eq
                (cdar first)
                advice-symbol))
              second
              (afterglow-test-advice-target
               'left
               'right)
              (nreverse calls)
              (advice-member-p
               advice-symbol
               'afterglow-test-advice-target)
              (length
               afterglow--advised-functions)
              (equal
               afterglow--advised-functions
               (list
                (cons
                 'afterglow-test-advice-target
                 advice-symbol)))))))"##;
    let expect = expect![[
        r#"OK (t nil (original left right) ((:thing word :duration 4)) #[128 "����\2\"����\3\"����" [afterglow--after-trigger-afterglow-test-advice-target #[(left right) ((list 'original left right)) (t)] :after nil apply] 5 advice] 1 t)"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advice_add_respects_an_already_bound_generated_function() {
    let elisp_form = r##"(let ((afterglow--advised-functions nil)
               calls)
         (fset
          'afterglow-test-prebound-target
          (lambda ()
            'target-result))
         (fset
          'afterglow--after-trigger-afterglow-test-prebound-target
          (lambda (&rest arguments)
            (push
             arguments
             calls)))
         (let ((result
                (afterglow--advice-add
                 'afterglow-test-prebound-target
                 'afterglow--after-trigger-afterglow-test-prebound-target)))
           (list
            result
            (afterglow-test-prebound-target)
            calls
            (advice-member-p
             'afterglow--after-trigger-afterglow-test-prebound-target
             'afterglow-test-prebound-target)
            afterglow--advised-functions
            (fboundp
             'afterglow--after-trigger-afterglow-test-prebound-target))))"##;
    let expect = expect!["OK (nil target-result nil nil nil t)"];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advice_remove_detaches_generated_advice_and_preserves_other_tracking() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (dolist (symbol
                  '(afterglow-test-detach-a
                    afterglow-test-detach-b))
           (fset
            symbol
            `(lambda ()
               ',symbol))
           (puthash
            symbol
            (list
             :thing
             symbol)
            afterglow--triggers)
           (afterglow--advice-add
            symbol
            (afterglow--advice-fn-symbol
             symbol)))
         (let ((result
                (afterglow--advice-remove
                 'afterglow-test-detach-a
                 'afterglow--after-trigger-afterglow-test-detach-a)))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls))))
             (list
              result
              (afterglow-test-detach-a)
              (afterglow-test-detach-b)
              (nreverse calls)
              (fboundp
               'afterglow--after-trigger-afterglow-test-detach-a)
              (fboundp
               'afterglow--after-trigger-afterglow-test-detach-b)
              (advice-member-p
               'afterglow--after-trigger-afterglow-test-detach-a
               'afterglow-test-detach-a)
              afterglow--advised-functions))))"##;
    let expect = expect![
        "OK (#1=((afterglow-test-detach-b . afterglow--after-trigger-afterglow-test-detach-b)) afterglow-test-detach-a afterglow-test-detach-b ((:thing afterglow-test-detach-b)) nil t nil #1#)"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advice_remove_cleans_tracking_even_when_generated_function_is_absent() {
    let elisp_form = r##"(let ((afterglow--advised-functions
                '((afterglow-test-absent
                   .
                   afterglow--after-trigger-afterglow-test-absent)
                  (afterglow-test-retained
                   .
                   afterglow--after-trigger-afterglow-test-retained))))
         (fmakunbound
          'afterglow--after-trigger-afterglow-test-absent)
         (let ((result
                (afterglow--advice-remove
                 'afterglow-test-absent
                 'afterglow--after-trigger-afterglow-test-absent)))
           (list
            result
            afterglow--advised-functions
            (fboundp
             'afterglow--after-trigger-afterglow-test-absent))))"##;
    let expect = expect![
        "OK (#1=((afterglow-test-retained . afterglow--after-trigger-afterglow-test-retained)) #1# nil)"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advice_remove_all_detaches_every_target_and_clears_tracking() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (dolist (symbol
                  '(afterglow-test-remove-all-a
                    afterglow-test-remove-all-b))
           (fset
            symbol
            `(lambda ()
               ',symbol))
           (puthash
            symbol
            (list
             :thing
             symbol)
            afterglow--triggers)
           (afterglow--advice-add
            symbol
            (afterglow--advice-fn-symbol
             symbol)))
         (let ((result
                (afterglow--advice-remove-all)))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls))))
             (list
              result
              (afterglow-test-remove-all-a)
              (afterglow-test-remove-all-b)
              calls
              afterglow--advised-functions
              (mapcar
               #'fboundp
               '(afterglow--after-trigger-afterglow-test-remove-all-a
                 afterglow--after-trigger-afterglow-test-remove-all-b))
              (mapcar
               (lambda (pair)
                 (advice-member-p
                  (cdr pair)
                  (car pair)))
               '((afterglow-test-remove-all-a
                  .
                  afterglow--after-trigger-afterglow-test-remove-all-a)
                 (afterglow-test-remove-all-b
                  .
                  afterglow--after-trigger-afterglow-test-remove-all-b)))))))"##;
    let expect = expect![
        "OK (nil afterglow-test-remove-all-a afterglow-test-remove-all-b nil nil (nil nil) (nil nil))"
    ];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advices_remove_unused_keeps_live_entries_and_prunes_stale_ones() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil)
               calls)
         (dolist (symbol
                  '(afterglow-test-live-advice
                    afterglow-test-stale-advice))
           (fset
            symbol
            `(lambda ()
               ',symbol))
           (puthash
            symbol
            (list
             :thing
             symbol)
            afterglow--triggers)
           (afterglow--advice-add
            symbol
            (afterglow--advice-fn-symbol
             symbol)))
         (remhash
          'afterglow-test-stale-advice
          afterglow--triggers)
         (let ((result
                (afterglow--advices-remove-unused)))
           (cl-letf
               (((symbol-function
                  'afterglow--apply-overlay)
                 (lambda (properties)
                   (push
                    properties
                    calls))))
             (list
              result
              (afterglow-test-live-advice)
              (afterglow-test-stale-advice)
              (nreverse calls)
              afterglow--advised-functions
              (mapcar
               #'fboundp
               '(afterglow--after-trigger-afterglow-test-live-advice
                 afterglow--after-trigger-afterglow-test-stale-advice))
              (mapcar
               (lambda (pair)
                 (advice-member-p
                  (cdr pair)
                  (car pair)))
               '((afterglow-test-live-advice
                  .
                  afterglow--after-trigger-afterglow-test-live-advice)
                 (afterglow-test-stale-advice
                  .
                  afterglow--after-trigger-afterglow-test-stale-advice)))))))"##;
    let expect = expect![[
        r#"OK (#1=((afterglow-test-live-advice . afterglow--after-trigger-afterglow-test-live-advice)) afterglow-test-live-advice afterglow-test-stale-advice ((:thing afterglow-test-live-advice)) #1# (t nil) (#[128 "����\2\"����\3\"����" [afterglow--after-trigger-afterglow-test-live-advice (lambda nil 'afterglow-test-live-advice) :after nil apply] 5 advice] nil))"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}

#[test]
fn afterglow_advices_remove_all_reports_both_options_but_unbinds_generated_functions() {
    let elisp_form = r##"(let ((afterglow--triggers
                (make-hash-table
                 :test 'equal))
               (afterglow--advised-functions nil))
         (fset
          'afterglow-test-cleanup
          (lambda ()
            'target-result))
         (puthash
          'afterglow-test-cleanup
          '(:thing word)
          afterglow--triggers)
         (afterglow--advice-add
          'afterglow-test-cleanup
          'afterglow--after-trigger-afterglow-test-cleanup)
         (let ((without-unbind
                (afterglow--advices-remove-all
                 nil))
               (without-bound
                (fboundp
                 'afterglow--after-trigger-afterglow-test-cleanup)))
           (afterglow--advice-add
            'afterglow-test-cleanup
            'afterglow--after-trigger-afterglow-test-cleanup)
           (let ((with-unbind
                  (afterglow--advices-remove-all
                   t)))
             (list
              without-unbind
              without-bound
              with-unbind
              (fboundp
               'afterglow--after-trigger-afterglow-test-cleanup)
              afterglow--advised-functions
              (afterglow-test-cleanup)
              (current-message)))))"##;
    let expect = expect![[
        r#"OK ("afterglow--advices-remove-all done. Functions not unbound." nil "afterglow--advices-remove-all done. Functions unbound." nil nil target-result nil)"#
    ]];
    assert_afterglow_parity(elisp_form, expect);
}
