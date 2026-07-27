use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_indent_defun_reformats_real_lisp_and_preserves_live_point_marker() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(defun first (value)\n"
          "(let ((next (+ value 1)))\n"
          "(if (> next 2)\n"
          "(progn\n"
          "(message \"large\")\n"
          "next)\n"
          "value)))\n\n"
          "(defun second ()\n"
          "(message \"untouched\"))\n")
         (goto-char (point-min))
         (search-forward "(message \"large\")")
         (let ((before (point)))
           (aggressive-indent-indent-defun)
           (list
            before
            (point)
            (line-number-at-pos)
            (current-column)
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK (87 89 5 19 "(defun first (value)\n(let ((next (+ value 1)))\n(if (> next 2)\n(progn\n  (message \"large\")\nnext)\nvalue)))\n\n(defun second ()\n(message \"untouched\"))\n")"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_indent_defun_uses_supplied_limits_and_custom_region_function() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(defun alpha ()\n(list 1 2))\n\n"
          "(defun beta ()\n(list 3 4))\n")
         (goto-char (point-min))
         (search-forward "(defun beta")
         (let ((left (point))
               (right (point-max))
               calls)
           (goto-char 7)
           (let ((aggressive-indent-region-function
                  (lambda (begin end)
                    (push
                     (list
                      begin
                      end
                      (buffer-substring-no-properties
                       begin end))
                     calls)
                    (goto-char begin)
                    (insert ">>"))))
             (aggressive-indent-indent-defun left right)
             (list
              (point)
              (buffer-string)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (7 "(defun alpha ()\n(list 1 2))\n\n>>(defun beta ()\n(list 3 4))\n" ((30 57 "(defun beta ()\n(list 3 4))\n")))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_soft_defun_swallows_region_errors_and_messages_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert "(defun broken ()\n(message \"x\")")
         (goto-char 9)
         (let ((before (point))
               messages)
           (cl-letf (((symbol-function 'message)
                      (lambda (&rest arguments)
                        (push arguments messages))))
             (list
              (aggressive-indent--softly-indent-defun)
              before
              (point)
              (buffer-string)
              (nreverse messages)))))"##;
    let expect = expect![[r#"OK (nil 9 9 "(defun broken ()\n(message \"x\")" nil)"#]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_extend_end_consumes_complete_sexps_across_whitespace_and_comments() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(alpha 1)  (beta\n"
          " 2 3)\n"
          ";; divider\n"
          "(gamma (delta 4))\n")
         (mapcar
          (lambda (limits)
            (let ((begin (car limits))
                  (end (cdr limits)))
              (list
               limits
               (aggressive-indent--extend-end-to-whole-sexps
                begin end)
               (buffer-substring-no-properties
                begin
                (aggressive-indent--extend-end-to-whole-sexps
                 begin end)))))
          '((1 . 2)
            (1 . 13)
            (12 . 17)
            (24 . 34))))"##;
    let expect = expect![[
        r#"OK (((1 . 2) 10 "(alpha 1)") ((1 . 13) 23 "(alpha 1)  (beta\n 2 3)") ((12 . 17) 23 "(beta\n 2 3)") ((24 . 34) 52 ";; divider\n(gamma (delta 4))"))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_balanced_line_reindents_following_sexps_as_one_real_region() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(progn\n"
          "(message \"first\") (list\n"
          "1\n"
          "2)\n"
          "(message \"last\"))\n")
         (goto-char (point-min))
         (forward-line 1)
         (let ((before (buffer-string)))
           (list
            (aggressive-indent--indent-current-balanced-line 0)
            before
            (buffer-string)
            (line-number-at-pos)
            (current-column))))"##;
    let expect = expect![[
        r#"OK (0 "(progn\n(message \"first\") (list\n1\n2)\n(message \"last\"))\n" "(progn\n  (message \"first\") (list\n\11\11     1\n\11\11     2)\n(message \"last\"))\n" 5 0)"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_balanced_line_respects_base_column_and_noop_indentation() {
    let elisp_form = r##"(mapcar
         (lambda (column)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(progn\n"
              "  (message \"already\"))\n")
             (goto-char (point-min))
             (forward-line 1)
             (when (= column 2)
               (forward-char 2))
             (list
              column
              (aggressive-indent--indent-current-balanced-line
               column)
              (buffer-string)
              (point)
              (current-column))))
         '(2 8))"##;
    let expect = expect![[
        r#"OK ((2 nil "(progn\n  (message \"already\"))\n" 10 2) (8 nil "(progn\n  (message \"already\"))\n" 8 0))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_region_and_on_repairs_real_cascading_lisp_indentation() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(let ((enabled t))\n"
          "(when enabled\n"
          "(progn\n"
          "(message \"one\")\n"
          "(message \"two\")))\n"
          "(message \"done\"))\n")
         (goto-char (point-min))
         (search-forward "(message \"one\")")
         (let ((left (point-min))
               (right (point-max))
               (before-point (point)))
           (aggressive-indent-indent-region-and-on left right)
           (list
            before-point
            (point)
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK (56 68 "(let ((enabled t))\n  (when enabled\n    (progn\n      (message \"one\")\n      (message \"two\")))\n  (message \"done\"))\n")"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_region_and_on_trims_newline_boundaries_and_honors_stop_hook() {
    let elisp_form = r##"(with-temp-buffer
         (insert "first\nsecond\nthird\nfourth\n")
         (goto-char 6)
         (let (region-calls stop-lines)
           (let ((aggressive-indent-region-function
                  (lambda (begin end)
                    (push
                     (list
                      begin end
                      (buffer-substring-no-properties
                       begin end))
                     region-calls)))
                 (aggressive-indent-stop-here-hook
                  (list
                   (lambda ()
                     (push
                      (line-number-at-pos)
                      stop-lines)
                     (= (line-number-at-pos) 3)))))
             (cl-letf (((symbol-function
                         'aggressive-indent--indent-current-balanced-line)
                        (lambda (column)
                          (push
                           (list 'continue
                                 (line-number-at-pos)
                                 column)
                           region-calls)
                          t)))
               (aggressive-indent-indent-region-and-on 6 14)
               (list
                (point)
                (nreverse region-calls)
                (nreverse stop-lines)
                (buffer-string))))))"##;
    let expect = expect![[r#"OK (6 ((7 13 "second")) (3) "first\nsecond\nthird\nfourth\n")"#]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_soft_region_swallows_failures_but_preserves_variadic_hook_shape() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha\nbeta\n")
         (goto-char 4)
         (let (messages calls)
           (let ((aggressive-indent-region-function
                  (lambda (&rest arguments)
                    (push arguments calls)
                    (message "about to fail")
                    (error "indent failure"))))
             (cl-letf (((symbol-function 'message)
                        (lambda (&rest arguments)
                          (push arguments messages))))
               (list
                (aggressive-indent--softly-indent-region-and-on
                 1 6 'ignored 'arguments)
                (point)
                (buffer-string)
                (nreverse calls)
                (nreverse messages))))))"##;
    let expect = expect![[r#"OK (nil 4 "alpha\nbeta\n" ((1 6)) nil)"#]];
    assert_aggressive_indent_parity(elisp_form, expect);
}
