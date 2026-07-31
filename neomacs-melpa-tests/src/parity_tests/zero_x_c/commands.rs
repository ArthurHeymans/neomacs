use expect_test::expect;

use super::assert_zero_x_c_batch;

#[test]
fn commands_public_surface_batch() {
    assert_zero_x_c_batch(&[
        (
            "zero_x_c_convert_reproduces_upstream_cross_base_examples",
            r##"(list
               (0xc-convert 10 "0xff" t)
               (0xc-convert 10 "'hff" t)
               (0xc-convert 10 "0b1010" t)
               (0xc-convert 10 "'b1010" t)
               (0xc-convert 8 "7:100" t)
               (0xc-convert 10 "5:41300" t)
               (0xc-convert 12 "0t10201020" t))"##,
            true,
            expect![[r#"OK ("255" "255" "10" "10" "61" "2700" "1696")"#]],
        ),
        (
            "zero_x_c_convert_messages_unless_silent",
            r##"(let (messages)
               (cl-letf (((symbol-function 'message)
                          (lambda (format-string &rest args)
                            (let ((text
                                   (apply
                                    #'format
                                    format-string
                                    args)))
                              (push text messages)
                              text))))
                 (list
                  (0xc-convert 16 "255")
                  (0xc-convert
                   2 "0xff" t)
                  (nreverse messages))))"##,
            true,
            expect![[r#"OK ("FF" "11111111" ("FF"))"#]],
        ),
        (
            "zero_x_c_convert_prompts_for_number_and_non_prefix_base",
            r##"(let (calls)
               (cl-letf (((symbol-function
                           'read-from-minibuffer)
                          (lambda (prompt &rest _)
                            (push
                             (list 'number prompt)
                             calls)
                            "0xff"))
                         ((symbol-function 'read-minibuffer)
                          (lambda (prompt &rest _)
                            (push
                             (list 'base prompt)
                             calls)
                            2)))
                 (list
                  (0xc-convert 1 nil t)
                  (nreverse calls))))"##,
            true,
            expect![[r#"OK ("11111111" ((number "Number: ") (base "Convert to base: ")))"#]],
        ),
        (
            "zero_x_c_bounds_at_point_include_apostrophe_hints",
            r##"(mapcar
               (lambda (case)
                 (with-temp-buffer
                   (insert (car case))
                   (goto-char (cdr case))
                   (0xc--bounds-of-number-at-point)))
               '(("before 0xBEEF after" . 11)
                 ("before 'hBEEF after" . 12)
                 ("12345" . 3)))"##,
            true,
            expect!["OK ((8 14) (8 14) (1 6))"],
        ),
        (
            "zero_x_c_convert_point_replaces_only_the_number_and_uses_default_base",
            r##"(list
               (with-temp-buffer
                 (insert "left 0xBEEF right")
                 (goto-char 10)
                 (0xc-convert-point 10)
                 (buffer-string))
               (let ((0xc-default-base 2))
                 (with-temp-buffer
                   (insert "value 'h0F end")
                   (goto-char 11)
                   (0xc-convert-point)
                   (buffer-string))))"##,
            true,
            expect![[r#"OK ("left 48879 right" "value 1111 end")"#]],
        ),
    ]);
}
