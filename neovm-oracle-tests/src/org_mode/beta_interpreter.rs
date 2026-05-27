//! Beta-strict combo tests for org-mode interpreter round-trips.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Beta: Block interpreter round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta_interpret_center_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_CENTER\nText\n#+END_CENTER")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert ":DRAWER:\nContents\n:END:")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_dynamic_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN: myblock :param val\nContent\n#+END:")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_footnote_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[fn:1] Definition.")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_headline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Headline")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_inlinetask() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'org-inlinetask)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "*************** Inline task\nBody\n*************** END")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_plain_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- Item 1\n- Item 2")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_quote_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_QUOTE\nQuoted\n#+END_QUOTE")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_special_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_SPECIAL\nContent\n#+END_SPECIAL")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_babel_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+CALL: test()")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "CLOCK: [2024-01-15 Mon 10:00]--[2024-01-15 Mon 11:00] =>  1:00")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "# Comment")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_comment_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_COMMENT\nTest\n#+END_COMMENT")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_diary_sexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "%%(org-anniversary 1956 5 14) Arthur Dent is %d years old")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_example_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_EXAMPLE\nTest\n#+END_EXAMPLE")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_export_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_EXPORT HTML\n<p>Text</p>\n#+END_EXPORT")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_fixed_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert ": Test")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_horizontal_rule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "-------")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+KEYWORD: value")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_latex_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\begin{equation}\n1+1=2\n\\end{equation}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_planning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Headline\nDEADLINE: <2012-03-29> SCHEDULED: <2012-03-29> CLOSED: [2012-03-29]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_property_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:prop: value\n:END:")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_src_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_SRC emacs-lisp :results silent\n(+ 1 1)\n#+END_SRC")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| a | b |\n| c | d |")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_table_with_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| 2 |\n| 4 |\n| 3 |\n#+TBLFM: @3=vmean(@1..@2)")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_verse_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_VERSE\nTest\n#+END_VERSE")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta: Object interpreter round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta_interpret_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "*text*")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_citation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[cite:@key]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_code() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "~text~")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_entity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\alpha text")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_export_snippet() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "@@backend:contents@@")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_footnote_reference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_footnote_reference_named() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:label]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_footnote_reference_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:label:def]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_footnote_reference_anonymous() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn::def]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_inline_babel_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "call_test()")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_inline_babel_call_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "call_test(x=2)")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_inline_src_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "src_emacs-lisp{(+ 1 1)}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "/text/")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_latex_fragment_command() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\command{}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_latex_fragment_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "$x$")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_latex_fragment_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "$$x+y$$")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_latex_fragment_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\(x+y\\)")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_latex_fragment_bracket() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\\[x+y\\]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_line_break() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "First line \\\\\nSecond line")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_no_desc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[https://orgmode.org]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_with_desc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[https://orgmode.org][Org mode]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[file:todo.org::*task]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[id:aaaa]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_custom_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[#id]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_coderef() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[(ref)]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_plain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "https://orgmode.org")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_angular() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<https://orgmode.org>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_link_pathological() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[[file://path][%s]]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_macro_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "{{{test}}}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_macro_with_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "{{{test(arg1,arg2)}}}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_radio_target() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<<<some text>>>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_statistics_cookie_fraction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[0/1]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_statistics_cookie_percent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[66%]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_strike_through() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "+target+")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_subscript() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "a_b")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_subscript_braces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "a_{b}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_superscript() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "a^b")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_superscript_braces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "a^{b}")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_target() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<<target>>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "_text_")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_verbatim() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "=text=")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta: Timestamp interpreter round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta_interpret_timestamp_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu 16:40>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "[2012-03-29 Thu 16:40]")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_active_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu 16:40>--<2012-03-29 Thu 16:41>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_active_timerange() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu 16:40-16:41>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_diary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<%%(diary-float t 4 2)>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_diary_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<%%(diary-float t 4 2) 12:00>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu +1y>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu -1y>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_repeater_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu +1y -1y>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}

#[test]
fn beta_interpret_timestamp_range_repeater() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "<2012-03-29 Thu +1y>--<2012-03-30 Fri +1y>")
      (org-element-interpret-data (org-element-parse-buffer)))))"##,
    );
}
