use std::time::Duration;

use crate::{CachedMelpaOracle, S_MELPA_PIN};
use expect_test::{Expect, expect};

const S_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn s_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(S_MELPA_PIN, "s.el")
        .expect("prepare pinned s source below ./tmp")
        .with_timeout(S_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed s parity test").into()
}

fn assert_s_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = s_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("s parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_s_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = s_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("s signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn s_trim_variants_preserve_non_boundary_content() {
    let elisp_form = r##"(list
              (s-trim-left " \t\nleft  ")
              (s-trim-left "already")
              (s-trim-left "   ")
              (s-trim-right "  right \r\n")
              (s-trim-right "already")
              (s-trim-right "\t\n")
              (s-trim " \n both \t ")
              (s-trim "")
              (s-trim " åß中 "))"##;
    let expect = expect![[r#"OK ("left  " "already" "" "  right" "already" "" "both" "" "åß中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_collapse_whitespace_normalizes_runs() {
    let elisp_form = r##"(list
              (s-collapse-whitespace "a \t\n b\r\nc")
              (s-collapse-whitespace "  leading")
              (s-collapse-whitespace "trailing  ")
              (s-collapse-whitespace "")
              (s-collapse-whitespace "å  中"))"##;
    let expect = expect![[r#"OK ("a b c" " leading" "trailing " "" "å 中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_unindent_handles_default_and_custom_markers() {
    let elisp_form = r##"(list
              (s-unindent "  |one\n\t|two")
              (s-unindent "  >one\n\t>two" ">")
              (s-unindent "plain\ntext")
              (s-unindent "")
              (s-unindent "  |å\n  |中"))"##;
    let expect = expect![[r#"OK ("one\ntwo" "one\ntwo" "plain\ntext" "" "å\n中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_split_controls_null_fields() {
    let elisp_form = r##"(list
              (s-split "," "a,,b," nil)
              (s-split "," "a,,b," t)
              (s-split "," "" nil)
              (s-split "," "" t)
              (s-split "[[:space:]]+" "a b\tc" t)
              (s-split "中" "å中ß中" nil))"##;
    let expect = expect![[r#"OK (("a" "" "b" "") ("a" "b") ("") nil ("a" "b" "c") ("å" "ß" ""))"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_split_up_to_honors_limit_and_null_policy() {
    let elisp_form = r##"(list
              (s-split-up-to ":" "a:b:c:d" 2)
              (s-split-up-to ":" ":a::b" 3 t)
              (s-split-up-to ":" "a:b" 0)
              (s-split-up-to ":" "a:b" 20)
              (s-split-up-to ":" "" 2 nil)
              (s-split-up-to ":" "" 2 t))"##;
    let expect = expect![[r#"OK (("a" "b" "c:d") ("a" "b") ("a:b") ("a" "b") ("") nil)"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_lines_recognizes_common_line_endings() {
    let elisp_form = r##"(list
              (s-lines "a\r\nb\nc\rd")
              (s-lines "one\n")
              (s-lines "\n")
              (s-lines "")
              (s-lines "å\n中"))"##;
    let expect = expect![[r#"OK (("a" "b" "c" "d") ("one" "") ("" "") ("") ("å" "中"))"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_join_and_concat_preserve_order() {
    let elisp_form = r##"(list
              (s-join "::" '("a" "b" "c"))
              (s-join "," nil)
              (s-join "" '("å" "中"))
              (s-concat "a" "" "b" "c")
              (s-concat)
              (s-concat "å" "中"))"##;
    let expect = expect![[r#"OK ("a::b::c" "" "å中" "abc" "" "å中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_prepend_and_append_handle_empty_and_unicode_strings() {
    let elisp_form = r##"(list
              (s-prepend "pre-" "value")
              (s-prepend "" "value")
              (s-prepend "å" "中")
              (s-append "-post" "value")
              (s-append "" "value")
              (s-append "中" "å"))"##;
    let expect = expect![[r#"OK ("pre-value" "value" "å中" "value-post" "value" "å中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_splice_supports_positive_negative_and_boundary_indices() {
    let elisp_form = r##"(list
              (s-splice "X" 0 "abcd")
              (s-splice "X" 2 "abcd")
              (s-splice "X" -1 "abcd")
              (s-splice "X" -3 "abcd")
              (s-splice "X" 4 "abcd")
              (s-splice "中" 1 "åß"))"##;
    let expect = expect![[r#"OK ("Xabcd" "abXcd" "abcdX" "abXcd" "abcdX" "å中ß")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_repeat_handles_zero_negative_and_unicode_counts() {
    let elisp_form = r##"(list
              (s-repeat 0 "ab")
              (s-repeat -1 "ab")
              (s-repeat 1 "")
              (s-repeat 3 "ab")
              (s-repeat 2 "å中"))"##;
    let expect = expect![[r#"OK ("" "" "" "ababab" "å中å中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_chop_prefix_variants_remove_only_matching_prefixes() {
    let elisp_form = r##"(list
              (s-chop-prefix "pre-" "pre-value")
              (s-chop-prefix "missing" "value")
              (s-chop-prefix "" "value")
              (s-chop-prefix "å" "å中")
              (s-chop-prefixes '("one-" "two-") "one-two-value")
              (s-chop-prefixes '("x" "y") "value")
              (s-chop-prefixes nil "value"))"##;
    let expect = expect![[r#"OK ("value" "value" "value" "中" "value" "value" "value")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_chop_suffix_variants_remove_only_matching_suffixes() {
    let elisp_form = r##"(list
              (s-chop-suffix ".el" "s.el")
              (s-chop-suffix ".rs" "s.el")
              (s-chop-suffix "" "value")
              (s-chop-suffix "中" "å中")
              (s-chop-suffixes '(".gz" ".tar") "archive.tar.gz")
              (s-chop-suffixes '("x" "y") "value")
              (s-chop-suffixes nil "value"))"##;
    let expect = expect![[r#"OK ("s" "s.el" "value" "å" "archive" "value" "value")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_shared_edges_cover_equal_disjoint_empty_and_unicode_strings() {
    let elisp_form = r##"(list
              (s-shared-start "prefix-one" "prefix-two")
              (s-shared-start "" "other")
              (s-shared-start "same" "same")
              (s-shared-start "å中x" "å中y")
              (s-shared-end "one-suffix" "two-suffix")
              (s-shared-end "abc" "xyz")
              (s-shared-end "same" "same")
              (s-shared-end "xå中" "yå中")
              (s-shared-end "" ""))"##;
    let expect = expect![[r#"OK ("prefix-" "" "same" "å中" "-suffix" "" "same" "å中" "")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_chomp_removes_one_line_ending() {
    let elisp_form = r##"(list
              (s-chomp "line\n")
              (s-chomp "line\r\n")
              (s-chomp "line\n\n")
              (s-chomp "line")
              (s-chomp "")
              (s-chomp "å中\n"))"##;
    let expect = expect![[r#"OK ("line" "line" "line\n" "line" "" "å中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_truncate_respects_width_and_ellipsis() {
    let elisp_form = r##"(list
              (s-truncate 8 "abcdefghijk")
              (s-truncate 8 "abcdefghijk" "…")
              (s-truncate 20 "short")
              (s-truncate 0 "")
              (s-truncate 4 "åß中x" ".")
              (s-truncate 3 "abc" "..."))"##;
    let expect = expect![[r#"OK ("abcde..." "abcdefg…" "short" "" "åß中x" "abc")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_word_wrap_handles_words_boundaries_and_existing_newlines() {
    let elisp_form = r##"(list
              (s-word-wrap 8 "one two three four")
              (s-word-wrap 3 "long")
              (s-word-wrap 10 "")
              (s-word-wrap 5 "åß 中 x")
              (s-word-wrap 8 "one\ntwo three"))"##;
    let expect = expect![[
        r#"OK ("one two\nthree\nfour" "long" "" #("åß 中\nx" 4 5 (fill-space " ")) "one two\nthree")"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_center_handles_padding_short_width_and_odd_remainders() {
    let elisp_form = r##"(list
              (s-center 8 "mid")
              (s-center 2 "long")
              (s-center 0 "")
              (s-center 5 "x")
              (s-center 4 "å中"))"##;
    let expect = expect![[r#"OK ("   mid  " "long" "" "  x  " " å中 ")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_padding_handles_growth_noop_and_multichar_padding() {
    let elisp_form = r##"(list
              (s-pad-left 6 "0" "42")
              (s-pad-right 6 "." "42")
              (s-pad-left 2 "0" "long")
              (s-pad-right 2 "." "long")
              (s-pad-left 7 "ab" "x")
              (s-pad-right 7 "ab" "x")
              (s-pad-left 0 "." "")
              (s-pad-right 0 "." ""))"##;
    let expect = expect![[r#"OK ("000042" "42...." "long" "long" "aaaaaax" "xaaaaaa" "" "")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_left_and_right_clamp_lengths_and_count_characters() {
    let elisp_form = r##"(list
              (s-left 3 "abcdef")
              (s-left 30 "abcdef")
              (s-left 0 "abc")
              (s-left 2 "åß中")
              (s-left -1 "abc")
              (s-right 3 "abcdef")
              (s-right 30 "abcdef")
              (s-right 0 "abc")
              (s-right 2 "åß中"))"##;
    let expect = expect![[r#"OK ("abc" "abcdef" "" "åß" "ab" "def" "abcdef" "" "ß中")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_chop_left_and_right_clamp_lengths_and_count_characters() {
    let elisp_form = r##"(list
              (s-chop-left 2 "abcdef")
              (s-chop-left 20 "abcdef")
              (s-chop-left 0 "abc")
              (s-chop-left 2 "åß中")
              (s-chop-right 2 "abcdef")
              (s-chop-right 20 "abcdef")
              (s-chop-right 0 "abc")
              (s-chop-right 2 "åß中"))"##;
    let expect = expect![[r#"OK ("cdef" "" "abc" "中" "abcd" "" "abc" "å")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_affix_predicates_cover_case_empty_and_aliases() {
    let elisp_form = r##"(list
              (s-starts-with? "init" "init.el")
              (s-starts-with? "INIT" "init.el" t)
              (s-starts-with? "" "init.el")
              (s-prefix? "init" "init.el")
              (s-prefix-p "init" "init.el")
              (s-starts-with-p "init" "init.el")
              (s-ends-with? ".el" "init.el")
              (s-ends-with? ".EL" "init.el" t)
              (s-ends-with? "" "init.el")
              (s-suffix? ".el" "init.el")
              (s-suffix-p ".el" "init.el")
              (s-ends-with-p ".el" "init.el"))"##;
    let expect = expect!["OK (t t t t t t t t t t t t)"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_contains_equals_and_less_predicates_cover_true_false_and_aliases() {
    let elisp_form = r##"(list
              (s-contains? "it." "init.el")
              (s-contains? "INIT" "init.el" t)
              (s-contains? "missing" "init.el")
              (s-contains-p "nit" "initial")
              (s-equals? "same" "same")
              (s-equals? "same" "different")
              (s-equals-p "same" "same")
              (s-less? "a" "b")
              (s-less? "b" "a")
              (s-less-p "a" "b"))"##;
    let expect = expect!["OK (t t nil t t nil t t nil t)"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_matches_and_index_of_preserve_search_boundaries() {
    let elisp_form = r##"(list
              (s-matches? "[0-9]+" "a12b")
              (s-matches? "a" "ba" 2)
              (s-matches-p "[0-9]" "a1")
              (s-index-of "it" "initial")
              (s-index-of "INIT" "initial" t)
              (s-index-of "missing" "initial")
              (s-index-of "" "initial")
              (s-index-of "中" "å中x"))"##;
    let expect = expect!["OK (t nil t 2 0 nil 0 1)"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_blank_present_and_presence_distinguish_nil_empty_and_whitespace() {
    let elisp_form = r##"(list
              (s-blank? nil)
              (s-blank? "")
              (s-blank? " \t\n")
              (s-blank? "x")
              (s-blank-p nil)
              (s-blank-str? " \t\n")
              (s-blank-str? "x")
              (s-blank-str-p " ")
              (s-present? nil)
              (s-present? "")
              (s-present? "value")
              (s-present-p "value")
              (s-presence nil)
              (s-presence "")
              (s-presence "value"))"##;
    let expect = expect![[r#"OK (t t nil nil t t nil t nil nil t t nil nil "value")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_case_predicates_cover_cased_uncased_empty_and_alias_values() {
    let elisp_form = r##"(list
              (s-lowercase? "lower 123")
              (s-lowercase? "Lower")
              (s-lowercase? "")
              (s-lowercase-p "lower")
              (s-uppercase? "UPPER 123")
              (s-uppercase? "Upper")
              (s-uppercase? "")
              (s-uppercase-p "UPPER")
              (s-mixedcase? "Mixed")
              (s-mixedcase? "lower")
              (s-mixedcase? "")
              (s-mixedcase-p "Mixed")
              (s-capitalized? "Capitalized")
              (s-capitalized? "capitalized")
              (s-capitalized? "")
              (s-capitalized-p "Capitalized"))"##;
    let expect = expect!["OK (t nil t t t nil t t t nil nil t t nil nil t)"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_numeric_predicate_requires_ascii_digits_only() {
    let elisp_form = r##"(list
              (s-numeric? "012345")
              (s-numeric? "12.5")
              (s-numeric? "-12")
              (s-numeric? "")
              (s-numeric? "１２")
              (s-numeric-p "123"))"##;
    let expect = expect!["OK (t nil nil nil nil t)"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_replace_literal_regexp_and_multiple_rules_are_exact() {
    let elisp_form = r##"(list
              (s-replace "." "/" "one.two.three")
              (s-replace "$" "\\dollar" "$5 + $10")
              (s-replace "" "-" "ab")
              (s-replace-regexp "[0-9]+" "#" "a12b345")
              (s-replace-regexp "x" "y" "abc")
              (s-replace-all
               '(("cat" . "dog") ("red" . "blue"))
               "red cat, cat")
              (s-replace-all nil "unchanged"))"##;
    let expect = expect![[
        r#"OK ("one/two/three" "\\dollar5 + \\dollar10" "-a-b" "a#b#" "abc" "blue dog, dog" "unchanged")"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_replace_all_is_single_pass_case_sensitive_and_matches_longest_keys() {
    let elisp_form = r##"(list
              (s-replace-all
               '(("lib" . "test") ("test" . "lib"))
               "lib/test.js")
              (s-replace-all
               '(("FOO" . "bar") ("FLOO" . "bah"))
               "FOO BLOO foo")
              (s-replace-all
               '(("cat" . "short") ("cater" . "long"))
               "cater cat")
              (s-replace-all
               '(("." . "dot") ("$" . "dollar"))
               ".$."))"##;
    let expect = expect![[r#"OK ("test/lib.js" "bar BLOO foo" "long short" "dotdollardot")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_case_conversion_handles_ascii_unicode_and_empty_strings() {
    let elisp_form = r##"(list
              (s-downcase "MiXeD Ä")
              (s-upcase "MiXeD ä")
              (s-capitalize "hELLO WORLD")
              (s-titleize "hELLO-world AGAIN")
              (s-downcase "")
              (s-upcase "")
              (s-capitalize "åNGSTRÖM")
              (s-titleize "åNGSTRÖM-value"))"##;
    let expect = expect![[
        r#"OK ("mixed ä" "MIXED Ä" "Hello world" "Hello-World Again" "" "" "Ångström" "Ångström-Value")"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_with_threads_values_and_evaluates_the_source_once() {
    let elisp_form = r##"(list
              (s-with "  hello  "
                s-trim
                s-upcase
                (s-prepend "[")
                (s-append "]"))
              (s-with "abc"
                (s-replace "b" "B")
                s-reverse)
              (s-with "abc" s-reverse)
              (let ((evaluations 0))
                (list
                 (s-with (progn (setq evaluations (1+ evaluations)) " x ")
                   s-trim
                   s-upcase)
                 evaluations))
              (macroexpand
               '(s-with value
                  (s-prepend "pre-")
                  (s-append "-post"))))"##;
    let expect = expect![[
        r#"OK ("[HELLO]" "cBa" "cba" ("X" 1) (s-append "-post" (s-with value (s-prepend "pre-"))))"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_reverse_counts_characters_and_preserves_multibyte_strings() {
    let elisp_form = r##"(list
              (s-reverse "abcdef")
              (s-reverse "")
              (s-reverse "åß中")
              (s-reverse (string ?A #x0301 ?B))
              (s-reverse "😀ab")
              (multibyte-string-p (s-reverse "åß中")))"##;
    let expect = expect![[r#"OK ("fedcba" "" "中ßå" "BÁ" "ba😀" t)"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_reverse_keeps_multiple_combining_marks_attached_to_their_base_character() {
    let elisp_form = r##"(list
              (s-reverse "résumé")
              (s-reverse "Ęyǫgwędę́hte⁷"))"##;
    let expect = expect![[r#"OK ("émusér" "⁷ethę́dęwgǫyĘ")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_match_strings_all_returns_full_and_capture_groups() {
    let elisp_form = r##"(list
              (s-match-strings-all
               "\\([a-z]+\\)=\\([0-9]+\\)"
               "x=1;y=22")
              (s-match-strings-all "[0-9]" "no digits")
              (s-match-strings-all "\\(å\\)\\(中\\)" "xå中y")
              (s-match-strings-all "^" "abc"))"##;
    let expect =
        expect![[r#"OK ((("x=1" "x" "1") ("y=22" "y" "22")) nil (("å中" "å" "中")) (("")))"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_matched_positions_all_tracks_depth_and_non_overlapping_matches() {
    let elisp_form = r##"(list
              (s-matched-positions-all "ana" "bananas")
              (s-matched-positions-all
               "\\([a-z]+\\)=\\([0-9]+\\)"
               "x=1;y=22"
               2)
              (s-matched-positions-all "missing" "value")
              (s-matched-positions-all "中" "å中x中"))"##;
    let expect = expect!["OK (((1 . 4)) ((2 . 3) (6 . 8)) nil ((1 . 2) (3 . 4)))"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_match_honors_start_and_restores_existing_match_data() {
    let elisp_form = r##"(list
              (s-match "\\([a-z]+\\)-\\([0-9]+\\)" "id-ab-42!")
              (s-match "\\([a-z]+\\)" "one two" 4)
              (s-match "missing" "value")
              (s-match "" "")
              (progn
                (string-match "\\(outer\\)" "outer")
                (let ((before (match-data)))
                  (s-match "\\([0-9]+\\)" "a12b")
                  (equal before (match-data)))))"##;
    let expect = expect![[r#"OK (("ab-42" "ab" "42") ("two" "two") nil ("") t)"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_match_distinguishes_trailing_and_intermediate_unmatched_capture_groups() {
    let elisp_form = r##"(list
              (s-match
               "^\\(abc\\)\\(def\\)?"
               "abc")
              (s-match
               "^\\(abc\\)\\(def\\)?\\(ghi\\)"
               "abcghi")
              (s-match "abc" "abcdefabc" 2)
              (s-match-strings-all "\\<" "foo bar baz"))"##;
    let expect =
        expect![[r#"OK (("abc" "abc") ("abcghi" "abc" nil "ghi") ("abc") (("") ("") ("")))"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_slice_at_exposes_each_regexp_boundary() {
    let elisp_form = r##"(list
              (s-slice-at "[A-Z]" "lowerCamelHTTP")
              (s-slice-at "[0-9]+" "abc12def34")
              (s-slice-at "," "")
              (s-slice-at "中" "å中ß中")
              (s-slice-at "missing" "value"))"##;
    let expect = expect![[
        r#"OK (("l" "o" "w" "e" "r" "C" "a" "m" "e" "l" "H" "T" "T" "P") ("abc" "1" "2def" "3" "4") ("") ("å" "中ß" "中") ("value"))"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_split_words_handles_case_separators_digits_unicode_and_empty_input() {
    let elisp_form = r##"(list
              (s-split-words "lowerCamelHTTP2Value")
              (s-split-words "snake_case dashed-words")
              (s-split-words "XMLHttpRequest")
              (s-split-words "")
              (s-split-words "  spaced  words  ")
              (s-split-words "ÅngströmValue"))"##;
    let expect = expect![[
        r#"OK (("lower" "Camel" "HTT" "P2Value") ("snake" "case" "dashed" "words") ("XML" "Http" "Request") nil ("spaced" "words") ("Ångström" "Value"))"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_camel_and_snake_case_transform_word_boundaries() {
    let elisp_form = r##"(list
              (s-lower-camel-case "HTTP response code")
              (s-lower-camel-case "")
              (s-upper-camel-case "http_response-code")
              (s-upper-camel-case "")
              (s-snake-case "HTTPResponseCode")
              (s-snake-case "already_snake")
              (s-snake-case ""))"##;
    let expect = expect![[
        r#"OK ("httpResponseCode" "" "HttpResponseCode" "" "http_response_code" "already_snake" "")"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_word_separator_styles_preserve_word_order() {
    let elisp_form = r##"(list
              (s-dashed-words "HTTPResponseCode")
              (s-dashed-words "")
              (s-spaced-words "HTTPResponseCode")
              (s-spaced-words "")
              (s-capitalized-words "HTTP_response CODE")
              (s-titleized-words "HTTP_response CODE")
              (s-titleized-words ""))"##;
    let expect = expect![[
        r#"OK ("http-response-code" "" "HTTP Response Code" "" "Http response code" "Http Response Code" "")"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_capitalized_words_rejects_empty_input() {
    let elisp_form = r##"(s-capitalized-words "")"##;
    let expect = expect!["ERR (wrong-type-argument char-or-string-p nil)"];

    assert_s_signal_parity(elisp_form, expect);
}

#[test]
fn s_word_initials_handles_spaces_separators_and_empty_input() {
    let elisp_form = r##"(list
              (s-word-initials "Hyper Text Markup Language")
              (s-word-initials "snake_case")
              (s-word-initials "lowerCamel")
              (s-word-initials "")
              (s-word-initials "å中 Value"))"##;
    let expect = expect![[r#"OK ("HTML" "sc" "lC" "" "åV")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_format_supports_hash_alist_vector_object_lambda_and_extra_values() {
    let elisp_form = r##"(progn
              (require 'eieio)
              (eval
               '(defclass s-parity-person ()
                  ((name :initarg :name))))
              (let ((table (make-hash-table :test 'equal))
                    (person
                     (make-instance 's-parity-person :name "Ada")))
                (puthash "name" "Ada" table)
                (list
                 (s-format "Hello ${name}" 'gethash table)
                 (s-format
                  "${name}:${language}"
                  'aget
                  '(("name" . "Ada") ("language" . "Lisp")))
                 (s-format "$2/$0/$1" 'elt ["zero" "one" "two"])
                 (s-format "Hello ${name}" 'oref person)
                 (s-format
                  "${key}"
                  (lambda (key) (upcase key)))
                 (s-format
                  "${key}"
                  (lambda (key suffix) (concat key suffix))
                  "!")
                 (s-format "literal" (lambda (_key) "unused")))))"##;
    let expect = expect![[
        r#"OK ("Hello Ada" "Ada:Lisp" "two/zero/one" "Hello Ada" "KEY" "key!" "literal")"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_format_callbacks_observe_and_restore_the_callers_match_data() {
    let elisp_form = r##"(progn
              (string-match "\\(outer\\)" "outer")
              (let ((before (match-data))
                    calls)
                (list
                 (s-format
                  "${key}-$0"
                  (lambda (key)
                    (push
                     (list key (match-string 1 "outer"))
                     calls)
                    (if (stringp key) "value" "zero")))
                 (nreverse calls)
                 (equal before (match-data)))))"##;
    let expect = expect![[r#"OK ("value-zero" (("key" "outer") (0 "outer")) t)"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_split_restores_the_callers_match_data() {
    let elisp_form = r##"(progn
              (string-match "\\(outer\\)" "outer")
              (let ((before (match-data)))
                (s-split "," "a,b")
                (equal before (match-data))))"##;
    let expect = expect!["OK t"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_reverse_restores_the_callers_match_data() {
    let elisp_form = r##"(progn
              (string-match "\\(outer\\)" "outer")
              (let ((before (match-data)))
                (s-reverse "å中")
                (equal before (match-data))))"##;
    let expect = expect!["OK t"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_match_strings_all_restores_the_callers_match_data() {
    let elisp_form = r##"(progn
              (string-match "\\(outer\\)" "outer")
              (let ((before (match-data)))
                (s-match-strings-all "[0-9]" "a1b2")
                (equal before (match-data))))"##;
    let expect = expect!["OK t"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_lex_format_uses_lexical_values_and_lisp_printing_policy() {
    let elisp_form = r##"(list
              (let ((name "Ada")
                    (count 3))
                (s-lex-format "${name} has ${count} messages"))
              (let ((s-lex-value-as-lisp t)
                    (payload '(a "b")))
                (s-lex-format "payload=${payload}"))
              (let ((s-lex-value-as-lisp nil)
                    (payload '(a "b")))
                (s-lex-format "payload=${payload}"))
              (s-lex-fmt|expand "${name}:${count}")
              (macroexpand
               '(s-lex-format "${name}:${count}")))"##;
    let expect = expect![[
        r#"OK ("Ada has 3 messages" "payload=(a \"b\")" "payload=(a b)" (s-format "${name}:${count}" #2='aget (list (cons "name" (format #1=(if s-lex-value-as-lisp "%S" "%s") name)) (cons "count" (format #1# count)))) (s-format "${name}:${count}" #2# (list (cons "name" (format #1# name)) (cons "count" (format #1# count)))))"#
    ]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_count_matches_distinguishes_overlapping_and_non_overlapping_ranges() {
    let elisp_form = r##"(list
              (s-count-matches "ana" "bananana")
              (s-count-matches-all "ana" "bananana")
              (s-count-matches "[0-9]" "a1b2c3" 3 6)
              (s-count-matches-all "[0-9]" "a1b2c3" 3 6)
              (s-count-matches "^" "abc")
              (s-count-matches-all "^" "abc")
              (s-count-matches "中" "中x中")
              (s-count-matches-all "aa" "aaaa"))"##;
    let expect = expect!["OK (2 3 1 1 1 0 2 3)"];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_wrap_uses_symmetric_or_distinct_delimiters() {
    let elisp_form = r##"(list
              (s-wrap "value" "*")
              (s-wrap "value" "<" ">")
              (s-wrap "" "[" "]")
              (s-wrap "å中" "«" "»")
              (s-wrap "value" "" ""))"##;
    let expect = expect![[r#"OK ("*value*" "<value>" "[]" "«å中»" "value")"#]];

    assert_s_parity(elisp_form, expect);
}

#[test]
fn s_trim_rejects_nil() {
    let elisp_form = r##"(s-trim nil)"##;
    let expect = expect!["ERR (wrong-type-argument stringp nil)"];

    assert_s_signal_parity(elisp_form, expect);
}

#[test]
fn s_splice_rejects_an_out_of_range_index() {
    let elisp_form = r##"(s-splice "X" 99 "abc")"##;
    let expect = expect![[r#"ERR (args-out-of-range "abc" 99 3)"#]];

    assert_s_signal_parity(elisp_form, expect);
}

#[test]
fn s_left_rejects_a_non_numeric_length() {
    let elisp_form = r##"(s-left 'two "abc")"##;
    let expect = expect!["ERR (wrong-type-argument number-or-marker-p two)"];

    assert_s_signal_parity(elisp_form, expect);
}

#[test]
fn s_format_reports_the_unresolved_placeholder() {
    let elisp_form = r##"(s-format "Hello ${missing}" (lambda (_key) nil))"##;
    let expect = expect![[r#"ERR (s-format-resolve . "${missing}")"#]];

    assert_s_signal_parity(elisp_form, expect);
}

#[test]
fn s_repeat_rejects_a_non_numeric_count() {
    let elisp_form = r##"(s-repeat 'three "x")"##;
    let expect = expect!["ERR (wrong-type-argument number-or-marker-p three)"];

    assert_s_signal_parity(elisp_form, expect);
}
