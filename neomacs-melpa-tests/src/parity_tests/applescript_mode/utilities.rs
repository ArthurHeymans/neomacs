use expect_test::expect;

use super::assert_applescript_mode_parity;

#[test]
fn applescript_mode_safe_macro_expansion_and_runtime_behavior_are_fully_exposed() {
    let elisp_form = r##"(let ((counter 0))
         (list
          (macroexpand
           '(as-safe
             (+ 20 22)))
          (macroexpand
           '(as-safe
             (setq counter
                   (1+ counter))
             (error "boom")))
          (as-safe
           (+ 20 22))
          (as-safe
           (setq counter
                 (1+ counter))
           (error
            "boom"))
          (as-safe)
          counter))"##;
    let expect = expect![[
        r#"OK ((condition-case nil (progn ((+ 20 22))) . #1=((error nil))) (condition-case nil (progn ((setq counter (1+ counter)) (error "boom"))) . #1#) nil nil nil 0)"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_point_queries_cover_all_documented_positions_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "on demo()\n"
          "    set value to 42\n"
          "end demo\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (search-forward
          "value")
         (let ((origin
                (point)))
           (list
            origin
            (mapcar
             (lambda (position)
               (list
                position
                (as-point position)
                (point)
                (= origin
                   (point))))
             '(bol
               eol
               bod
               eod
               bob
               eob
               boi
               bos))
            (point))))"##;
    let expect = expect![
        "OK (24 ((bol 11 24 t) (eol 30 24 t) (bod 24 24 t) (eod 24 24 t) (bob 1 24 t) (eob 40 24 t) (boi 15 24 t) (bos 24 24 t)) 24)"
    ];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_point_rejects_unknown_positions_and_still_restores_the_cursor() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "first\nsecond\n")
         (goto-char 8)
         (let ((origin
                (point)))
           (list
            (condition-case error
                (as-point
                 'middle)
              (error
               (list
                (car error)
                (cadr error))))
            origin
            (point)
            (line-number-at-pos)
            (current-column))))"##;
    let expect = expect![[r#"OK ((error "Unknown buffer position requested: middle") 8 8 2 1)"#]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_region_activity_helper_is_inert_on_gnu_and_sets_the_xemacs_contract() {
    let elisp_form = r##"(progn
         (defvar zmacs-region-stays nil)
         (setq zmacs-region-stays
               nil)
         (let ((normal
                (as-keep-region-active)))
           (cl-letf
               (((symbol-function
                  'featurep)
                 (lambda (feature)
                   (eq feature
                       'xemacs))))
             (let ((simulated
                    (as-keep-region-active)))
               (list
                normal
                simulated
                zmacs-region-stays
                (boundp
                 'zmacs-region-stays))))))"##;
    let expect = expect!["OK (nil t t t)"];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_placeholder_handler_navigation_functions_preserve_real_buffer_state() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "on first()\n"
          "    return 1\n"
          "end first\n"
          "\n"
          "to second()\n"
          "    return 2\n"
          "end second\n")
         (goto-char 18)
         (let ((origin
                (point)))
           (list
            (as-beginning-of-handler
             'on)
            (point)
            (as-end-of-handler
             'to)
            (point)
            (as-goto-initial-line)
            (point)
            (= origin
               (point)))))"##;
    let expect = expect![[r#"OK ("Todo." 18 "Todo." 18 "Todo." 18 t)"#]];

    assert_applescript_mode_parity(elisp_form, expect);
}
