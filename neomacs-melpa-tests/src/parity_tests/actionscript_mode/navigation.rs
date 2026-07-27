use expect_test::expect;

use super::assert_actionscript_mode_parity;

#[test]
fn actionscript_mode_defun_position_helpers_cover_inside_between_before_and_after_functions() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "function plain() {\n  trace(1);\n}\n\n")
         (insert
          "public static function second(value:int):void {\n  if (value) {\n    trace(value);\n  }\n}\n")
         (actionscript-mode)
         (let ((positions
                (mapcar
                 (lambda (needle)
                   (goto-char
                    (point-min))
                   (search-forward
                    needle)
                   (cons needle
                         (point)))
                 '("function plain"
                   "trace(1)"
                   "public static"
                   "trace(value)"
                   "\n}\n"))))
           (mapcar
            (lambda (entry)
              (goto-char
               (cdr entry))
              (list
               (car entry)
               (point)
               (as-get-beginning-of-defun)
               (as-get-end-of-defun)
               (as-get-end-of-defun2)
               (as-inside-defun?)))
            positions)))"##;
    let expect = expect![[
        r#"OK (("function plain" 15 nil nil 33 nil) ("trace(1)" 30 nil 33 121 nil) ("public static" 48 nil 33 121 nil) ("trace(value)" 114 35 121 nil t) ("\n}\n" 34 nil 33 121 nil))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_navigation_commands_move_mark_regions_and_report_missing_functions() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "public function alpha():void {\n  trace(1);\n}\n\n")
         (insert
          "private function beta():String {\n  return \"b\";\n}\n")
         (actionscript-mode)
         (let (results)
           (goto-char
            (point-max))
           (as-beginning-of-defun)
           (push
            (list
             'beginning
             (point)
             (buffer-substring-no-properties
              (line-beginning-position)
              (line-end-position)))
            results)
           (search-forward
            "return")
           (as-end-of-defun)
           (push
            (list
             'end
             (point)
             (char-before))
            results)
           (goto-char
            (point-max))
           (as-mark-defun)
           (push
            (list
             'mark
             (point)
             (mark)
             (buffer-substring-no-properties
              (point)
              (mark)))
            results)
           (erase-buffer)
           (insert
            "no functions here")
           (goto-char
            (point-max))
           (as-beginning-of-defun)
           (push
            (list
             'missing-beginning
             (point)
             (current-message))
            results)
           (as-end-of-defun)
           (push
            (list
             'missing-end
             (point)
             (current-message))
            results)
           (as-mark-defun)
           (push
            (list
             'missing-mark
             (point)
             (current-message))
            results)
           (nreverse results)))"##;
    let expect = expect![[
        r#"OK ((beginning 47 "private function beta():String {") (end 95 125) (mark 47 95 "private function beta():String {\n  return \"b\";\n}") (missing-beginning 18 nil) (missing-end 18 nil) (missing-mark 18 nil))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_defun_helpers_handle_malformed_bodies_and_unmatched_braces_exactly() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (insert source)
             (actionscript-mode)
             (goto-char
              (point-max))
             (list
              source
              (condition-case error
                  (as-get-end-of-defun)
                (error
                 (list
                  (car error)
                  (cdr error))))
              (condition-case error
                  (as-get-end-of-defun2)
                (error
                 (list
                  (car error)
                  (cdr error))))
              (condition-case error
                  (as-inside-defun?)
                (error
                 (list
                  (car error)
                  (cdr error)))))))
         '("public function open():void {"
           "function noBody()"
           "public function nested():void { if (x) { y(); } }"
           ""))"##;
    let expect = expect![[
        r#"OK (("public function open():void {" (scan-error ("Unbalanced parentheses" 29 30)) (scan-error ("Unbalanced parentheses" 29 30)) (scan-error ("Unbalanced parentheses" 29 30))) ("function noBody()" nil nil nil) ("public function nested():void { if (x) { y(); } }" 50 50 nil) ("" nil nil nil))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_missing_navigation_commands_emit_exact_user_messages() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "no functions here")
         (actionscript-mode)
         (goto-char
          (point-max))
         (mapcar
          (lambda (command)
            (let (messages)
              (cl-letf
                  (((symbol-function
                     'message)
                    (lambda
                      (format-string
                       &rest arguments)
                      (push
                       (list
                        format-string
                        arguments
                        (apply
                         #'format
                         format-string
                         arguments))
                       messages))))
                (funcall command))
              (list
               command
               (point)
               (nreverse messages))))
          '(as-beginning-of-defun
            as-end-of-defun
            as-mark-defun)))"##;
    let expect = expect![[
        r#"OK ((as-beginning-of-defun 18 (("Can't find any functions." nil "Can't find any functions."))) (as-end-of-defun 18 (("Can't find any functions." nil "Can't find any functions."))) (as-mark-defun 18 (("Can't find any functions." nil "Can't find any functions."))))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}
