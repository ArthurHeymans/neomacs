use expect_test::expect;

use super::assert_afterglow_autoload_parity;

#[test]
fn afterglow_autoload_file_registers_public_entry_points_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'afterglow)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (autoloadp
              (symbol-function symbol))
             (nth
              1
              (symbol-function symbol))
             (nth
              4
              (symbol-function symbol))))
          '(afterglow-add-trigger
            afterglow-add-triggers
            afterglow-remove-trigger
            afterglow-remove-triggers
            afterglow-mode))
         (boundp 'afterglow-mode)
         (get 'afterglow-mode 'custom-autoload)
         (get 'afterglow 'custom-loads))"##;
    let expect = expect![
        "OK (nil ((afterglow-add-trigger nil nil nil) (afterglow-add-triggers nil nil nil) (afterglow-remove-trigger nil nil nil) (afterglow-remove-triggers nil nil nil) (afterglow-mode nil nil nil)) nil nil nil)"
    ];
    assert_afterglow_autoload_parity(elisp_form, expect);
}

#[test]
fn afterglow_autoload_file_then_explicit_require_supports_a_real_advised_trigger() {
    let elisp_form = r##"(progn
         (require
          'afterglow)
         (fset
          'afterglow-test-autoload-target
          (lambda (value)
            (list
             'target
             value)))
         (afterglow-add-trigger
          'afterglow-test-autoload-target
          :thing 'word
          :duration 90)
         (let ((result
                (afterglow-test-autoload-target
                 7))
               (advice-symbol
                (afterglow--advice-fn-symbol
                 'afterglow-test-autoload-target)))
           (list
            result
            (featurep 'afterglow)
            (gethash
             'afterglow-test-autoload-target
             afterglow--triggers)
            (and
             (advice-member-p
              advice-symbol
              'afterglow-test-autoload-target)
             t)
            afterglow--advised-functions)))"##;
    let expect = expect![
        "OK ((target 7) t (:thing word :duration 90) t ((afterglow-test-autoload-target . afterglow--after-trigger-afterglow-test-autoload-target)))"
    ];
    assert_afterglow_autoload_parity(elisp_form, expect);
}
