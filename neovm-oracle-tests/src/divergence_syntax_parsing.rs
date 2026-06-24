//! Syntax parsing divergence probes (calibration).
//!
//! Probes parse-partial-sexp state vectors (paren depth, in-string,
//! in-comment, quoted, comment-style) across various buffer contents,
//! scan-lists, scan-sexps, and forward-sexp/list/up-list/down-list navigation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_sp_parse_partial_basic_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b) c)")
  (parse-partial-sexp 1 4))
"##,
    );
}

#[test]
fn div_sp_parse_partial_into_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(concat \"abc")
  (parse-partial-sexp 1 12))
"##,
    );
}

#[test]
fn div_sp_parse_partial_into_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "; abc comment")
  (parse-partial-sexp 1 8))
"##,
    );
}

#[test]
fn div_sp_parse_partial_nested_parens() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(((a)))")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_parse_partial_escaped_quote_in_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\"a\\\"b")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_parse_partial_semicolon_in_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\"a;b\"")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_scan_lists_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b) c) x")
  (scan-lists 1 1))
"##,
    );
}

#[test]
fn div_sp_scan_lists_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "x (a (b) c)")
  (scan-lists 12 -1))
"##,
    );
}

#[test]
fn div_sp_scan_sexps_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "a b c d")
  (scan-sexps 1 2))
"##,
    );
}

#[test]
fn div_sp_forward_sexp_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a b) (c d)")
  (goto-char 1)
  (list (progn (forward-sexp) (point))
        (progn (forward-sexp) (point))))
"##,
    );
}

#[test]
fn div_sp_forward_list_up_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b c) d)")
  (goto-char 1)
  (list (progn (forward-list) (point))
        (progn (backward-list) (point))
        (progn (down-list) (point))))
"##,
    );
}

#[test]
fn div_sp_up_list_from_inner() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b c) d)")
  (goto-char 5)
  (condition-case err (progn (up-list) (point)) (error (car err))))
"##,
    );
}

#[test]
fn div_sp_parse_partial_quoted_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // "\(a\)": every paren is escaped (Sescape quotes the next char), so neither
    // "(" nor ")" is a delimiter.  The trailing escape skips to EOB and GNU's
    // scan_sexps_forward reaches `endquoted', which bypasses `symdone' so the
    // last-complete-sexp slot (element 2) stays nil; element 5 (quoted) is t and
    // element 10 holds the escape syntax code (Sescape = 9).
    // GNU: (0 nil nil nil nil t 0 nil nil nil 9)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\\(a\\)")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_parse_partial_escaped_paren_mid_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // "(a \( b)": the escaped "(" in the middle is NOT an open paren, so the
    // outer list stays balanced and closes; element 2 (last complete sexp) is
    // the outer list start 1, element 5 nil, element 10 nil.
    // GNU: (0 nil 1 nil nil nil 0 nil nil nil nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a \\( b)")
  (parse-partial-sexp 1 9))
"##,
    );
}

#[test]
fn div_sp_parse_partial_char_literal_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // "?(": parse-partial-sexp uses the SYNTAX TABLE, not reader semantics, so
    // "?" is an expression-prefix and "(" still opens a list -> depth 1, with
    // the open paren at 2 recorded in element 1 and the open-paren stack (2).
    // GNU: (1 2 nil nil nil nil 0 nil nil (2) nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "?(")
  (parse-partial-sexp 1 3))
"##,
    );
}

#[test]
fn div_sp_parse_partial_escaped_paren_in_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Escape inside a string: "\"a\\(b\"" is the 5 chars  " a \ ( b  with no
    // closing quote scanned, so element 3 reports the string terminator (34 =
    // ?\") and element 8 the string start (1); the escaped "(" is inert.
    // GNU: (0 nil nil 34 nil nil 0 nil 1 nil nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\"a\\(b\"")
  (parse-partial-sexp 1 6))
"##,
    );
}

#[test]
fn div_sp_parse_partial_lone_trailing_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // "a\": a symbol char then a trailing escape at EOB.  Like \(a\), the
    // trailing escape forces `endquoted', so the symbol is NOT registered as a
    // complete sexp (element 2 nil); element 5 (quoted) is t and element 10 = 9.
    // GNU: (0 nil nil nil nil t 0 nil nil nil 9)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "a\\")
  (parse-partial-sexp 1 3))
"##,
    );
}

#[test]
fn div_sp_parse_partial_escaped_symbol_completes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // "a\(b\)c" scanned fully: the escapes are part of one symbol run; a normal
    // symbol char ends the run via `symdone', so element 2 (last complete sexp)
    // is the symbol start 1, with no trailing quote (element 5 nil, 10 nil).
    // GNU: (0 nil 1 nil nil nil 0 nil nil nil nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "a\\(b\\)c")
  (parse-partial-sexp 1 8))
"##,
    );
}

#[test]
fn div_sp_parse_partial_box_quotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(comment \"text\")")
  (parse-partial-sexp 1 10))
"##,
    );
}

#[test]
fn div_sp_parse_partial_oldstate_continue() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a \"str\" (b))")
  (let* ((s1 (parse-partial-sexp 1 5))
         (s2 (parse-partial-sexp 5 9 nil nil s1)))
    (list s1 s2)))
"##,
    );
}

#[test]
fn div_sp_unbalanced_paren_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b")
  (condition-case err (scan-lists 1 1) (scan-error (list 'scan-error)) (error (car err))))
"##,
    );
}

// ---------------------------------------------------------------------------
// parse-sexp-ignore-comments sexp scanning (PR #134 / #118)
//
// forward-sexp/backward-sexp/scan-sexps/scan-lists skip whole comment bodies
// when `parse-sexp-ignore-comments' is non-nil, matching GNU's
// scan_sexps_forward / scan_lists comment-skipping (src/syntax.c).  These lock
// in that a stray/unbalanced paren inside a comment body is ignored, and that
// the non-ignore path (parse-sexp-ignore-comments nil) is unaffected.
// ---------------------------------------------------------------------------

#[test]
fn div_sp_ignore_comments_forward_sexp_line_comment_stray_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // emacs-lisp-mode has parse-sexp-ignore-comments t by default; the stray
    // "(" inside the ";; oops (" comment must not be treated as an open paren,
    // so forward-sexp skips the comment and the "(real sexp)" list -> 22.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert ";; oops (\n(real sexp)\n")
  (goto-char (point-min))
  (forward-sexp)
  (point))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_forward_sexp_c_block_comment_parens() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // The C block comment "/* ) ( */" holds an unbalanced close+open; with
    // parse-sexp-ignore-comments t forward-sexp skips the comment body and the
    // whole list "(a /* ) ( */ b)" -> 16.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (c-mode)
  (setq parse-sexp-ignore-comments t)
  (insert "(a /* ) ( */ b)")
  (goto-char (point-min))
  (forward-sexp)
  (point))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_backward_sexp_line_comment_stray_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // backward-sexp 2 from end skips "(bar)" then the "; c (" comment with its
    // stray "(" and lands at the start of "(foo)" -> 1.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(foo) ; c (\n(bar)")
  (goto-char (point-max))
  (backward-sexp 2)
  (point))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_backward_sexp_nested_c_block_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // c-mode treats /* */ as nestable here; backward-sexp from end skips the
    // "(b)" then must skip the whole nested "/* outer /* inner */ */" comment,
    // landing at the start of "(b)" line -> 29.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (c-mode)
  (setq parse-sexp-ignore-comments t)
  (insert "(a) /* outer /* inner */ */\n(b)")
  (goto-char (point-max))
  (backward-sexp)
  (point))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_scan_sexps_unbalanced_paren_in_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // The #118 case: the unmatched "(" lives inside the ";  unmatched (" line
    // comment, so scan-sexps over the "(a ... b)" list finds the real closing
    // paren and returns 21 instead of signaling.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a ; unmatched (\n b)")
  (scan-sexps 1 1))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_forward_sexp_two_block_comments_in_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Two separate block comments inside a single list; forward-sexp must skip
    // both and the whole list "(a /* c */ b /* d */ e)" -> 24.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (c-mode)
  (setq parse-sexp-ignore-comments t)
  (insert "(a /* c */ b /* d */ e)")
  (goto-char (point-min))
  (forward-sexp)
  (point))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_scan_lists_backward_over_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // scan-lists backward over a comment containing a stray close paren must
    // skip the comment body and land before "(b)" -> 13.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (c-mode)
  (setq parse-sexp-ignore-comments t)
  (insert "(a) /* ) */ (b)")
  (scan-lists (point-max) -1 0))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_eof_unterminated_c_block_signals() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // An unterminated C block comment runs to EOF; GNU signals scan-error
    // because the enclosing list never closes.  Lock the signal class.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (c-mode)
  (setq parse-sexp-ignore-comments t)
  (insert "(a /* unterminated")
  (goto-char (point-min))
  (condition-case err
      (progn (forward-sexp) (point))
    (scan-error (list 'scan-error))
    (error (car err))))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_eof_unterminated_line_signals() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // An unterminated line comment swallows the rest of the buffer; the
    // enclosing list "(a ; ..." never closes -> scan-error.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a ; unterminated")
  (goto-char (point-min))
  (condition-case err
      (progn (forward-sexp) (point))
    (scan-error (list 'scan-error))
    (error (car err))))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_disabled_line_comment_stops_in_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // With parse-sexp-ignore-comments nil the comment is NOT skipped: ";" is a
    // comment-starter but not a sexp boundary, so forward-sexp over ";; oops "
    // stops just before the stray "(" at 8.  Locks the non-ignore path.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (setq parse-sexp-ignore-comments nil)
  (insert ";; oops (\n(real sexp)\n")
  (goto-char (point-min))
  (condition-case err
      (progn (forward-sexp) (point))
    (scan-error (list 'scan-error))
    (error (car err))))
"##,
    );
}

#[test]
fn div_sp_ignore_comments_disabled_scan_sexps_sees_comment_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // With parse-sexp-ignore-comments nil the stray "(" inside the comment is
    // counted as a real open paren, leaving the list unbalanced -> scan-error.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (setq parse-sexp-ignore-comments nil)
  (insert "(a ; unmatched (\n b)")
  (condition-case err
      (scan-sexps 1 1)
    (scan-error (list 'scan-error))
    (error (car err))))
"##,
    );
}
