use expect_test::expect;

use super::assert_ansi_parity;

#[test]
fn every_cursor_wrapper_preserves_default_zero_positive_and_negative_repetition_values() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (funcall symbol)
    (funcall symbol 0)
    (funcall symbol 1)
    (funcall symbol 3)
    (funcall symbol -2)))
 '(ansi-up
   ansi-down
   ansi-forward
   ansi-backward
   ansi-next-line
   ansi-previous-line
   ansi-column
   ansi-kill))"##;
    let expect = expect![[
        r#"OK ((ansi-up "\33[1A" "\33[0A" "\33[1A" "\33[3A" "\33[-2A") (ansi-down "\33[1B" "\33[0B" "\33[1B" "\33[3B" "\33[-2B") (ansi-forward "\33[1C" "\33[0C" "\33[1C" "\33[3C" "\33[-2C") (ansi-backward "\33[1D" "\33[0D" "\33[1D" "\33[3D" "\33[-2D") (ansi-next-line "\33[1E" "\33[0E" "\33[1E" "\33[3E" "\33[-2E") (ansi-previous-line "\33[1F" "\33[0F" "\33[1F" "\33[3F" "\33[-2F") (ansi-column "\33[1G" "\33[0G" "\33[1G" "\33[3G" "\33[-2G") (ansi-kill "\33[1K" "\33[0K" "\33[1K" "\33[3K" "\33[-2K"))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn csi_apply_accepts_registered_symbols_and_literal_final_characters_for_real_terminal_operations()
{
    let elisp_form = r##"(mapcar
 (lambda (case)
   (let ((effect-or-character (car case))
         (repetitions (cadr case)))
     (list
      effect-or-character
      repetitions
      (ansi-csi-apply effect-or-character repetitions)
      (string-to-list
       (ansi-csi-apply effect-or-character repetitions)))))
 '((up nil)
   (forward 12)
   (column 80)
   (kill 2)
   ("J" 2)
   ("H" 1)
   ("m" 38)))"##;
    let expect = expect![[
        r#"OK ((up nil "\33[1A" (27 91 49 65)) (forward 12 "\33[12C" (27 91 49 50 67)) (column 80 "\33[80G" (27 91 56 48 71)) (kill 2 "\33[2K" (27 91 50 75)) ("J" 2 "\33[2J" (27 91 50 74)) ("H" 1 "\33[1H" (27 91 49 72)) ("m" 38 "\33[38m" (27 91 51 56 109)))"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn dsl_composes_cursor_motion_and_colored_text_into_a_practical_terminal_progress_update() {
    let elisp_form = r##"(let ((value
       (with-ansi
        (column 1)
        (kill 2)
        (bold (cyan "Compiling %-12s" "neovm-core"))
        (forward 2)
        (green "%3d%%" 87)
        (previous-line 1))))
  (list value (string-to-list value) (length value)))"##;
    let expect = expect![[
        r#"OK ("\33[1G\33[2K\33[1m\33[36mCompiling neovm-core  \33[0m\33[0m\33[2C\33[32m 87%\33[0m\33[1F" (27 91 49 71 27 91 50 75 27 91 49 109 27 91 51 54 109 67 111 109 112 105 108 105 110 103 32 110 101 111 118 109 45 99 111 114 101 32 32 27 91 48 109 27 91 48 109 27 91 50 67 27 91 51 50 109 32 56 55 37 27 91 48 109 27 91 49 70) 68)"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn cursor_helpers_accept_large_counts_without_truncation_or_terminal_state() {
    let elisp_form = r##"(list
 (ansi-up 2147483647)
 (ansi-down 999999999999999999)
 (ansi-forward most-positive-fixnum)
 (ansi-backward most-negative-fixnum)
 (ansi-column 1000000)
 (ansi-kill 0)
 (ansi-kill 1)
 (ansi-kill 2))"##;
    let expect = expect![[
        r#"OK ("\33[2147483647A" "\33[999999999999999999B" "\33[2305843009213693951C" "\33[-2305843009213693952D" "\33[1000000G" "\33[0K" "\33[1K" "\33[2K")"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}

#[test]
fn repeated_csi_calls_are_pure_and_do_not_mutate_the_public_registry() {
    let elisp_form = r##"(let ((before (copy-tree ansi-csis)))
  (dotimes (index 5)
    (ansi-up (1+ index))
    (ansi-column (* 10 (1+ index)))
    (ansi-kill (% index 3)))
  (list
   before
   ansi-csis
   (equal before ansi-csis)
   (ansi-up)
   (ansi-column)
   (ansi-kill)))"##;
    let expect = expect![[
        r#"OK (((up . "A") (down . "B") (forward . "C") (backward . "D") (next-line . "E") (previous-line . "F") (column . "G") (kill . "K")) ((up . "A") (down . "B") (forward . "C") (backward . "D") (next-line . "E") (previous-line . "F") (column . "G") (kill . "K")) t "\33[1A" "\33[1G" "\33[1K")"#
    ]];
    assert_ansi_parity(elisp_form, expect);
}
