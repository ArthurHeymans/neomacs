use std::time::Duration;

use neomacs_melpa_tests::{CachedMelpaOracle, S_MELPA_PIN};

const S_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn s_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(S_MELPA_PIN, "s.el")
        .expect("prepare pinned s source below ./tmp")
        .with_timeout(S_TEST_TIMEOUT)
}

fn assert_s_parity(name: &str, form: &str) {
    s_oracle()
        .run_value(name, form)
        .unwrap_or_else(|error| panic!("s parity case `{name}` failed:\n{error}"));
}

fn assert_s_signal_parity(name: &str, form: &str) {
    s_oracle()
        .run_signal(name, form)
        .unwrap_or_else(|error| panic!("s signal parity case `{name}` failed:\n{error}"));
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_trimming_splitting_and_joining() {
    assert_s_parity(
        "s_trimming_splitting_and_joining",
        r##"(list
              (s-trim-left " \t\nleft  ")
              (s-trim-right "  right \r\n")
              (s-trim " \n both \t ")
              (s-collapse-whitespace "a \t\n b\r\nc")
              (s-unindent "  |one\n\t|two")
              (s-unindent "  >one\n\t>two" ">")
              (s-split "," "a,,b," nil)
              (s-split "," "a,,b," t)
              (s-split-up-to ":" "a:b:c:d" 2)
              (s-split-up-to ":" ":a::b" 3 t)
              (s-lines "a\r\nb\nc\rd")
              (s-join "::" '("a" "b" "c"))
              (s-concat "a" "" "b" "c"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_affixes_splicing_and_repetition() {
    assert_s_parity(
        "s_affixes_splicing_and_repetition",
        r##"(list
              (s-prepend "pre-" "value")
              (s-append "-post" "value")
              (s-splice "X" 0 "abcd")
              (s-splice "X" 2 "abcd")
              (s-splice "X" -1 "abcd")
              (s-splice "X" -3 "abcd")
              (s-repeat 0 "ab")
              (s-repeat 3 "ab")
              (s-chop-prefix "pre-" "pre-value")
              (s-chop-prefix "missing" "value")
              (s-chop-prefixes '("one-" "two-") "one-two-value")
              (s-chop-suffix ".el" "s.el")
              (s-chop-suffix ".rs" "s.el")
              (s-chop-suffixes '(".gz" ".tar") "archive.tar.gz"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_shared_edges_chomp_and_truncation() {
    assert_s_parity(
        "s_shared_edges_chomp_and_truncation",
        r##"(list
              (s-shared-start "prefix-one" "prefix-two")
              (s-shared-start "" "other")
              (s-shared-start "same" "same")
              (s-shared-end "one-suffix" "two-suffix")
              (s-shared-end "abc" "xyz")
              (s-shared-end "same" "same")
              (s-chomp "line\n")
              (s-chomp "line\r\n")
              (s-chomp "line\n\n")
              (s-chomp "line")
              (s-truncate 8 "abcdefghijk")
              (s-truncate 8 "abcdefghijk" "…")
              (s-truncate 20 "short")
              (s-truncate 0 ""))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_wrapping_padding_and_substrings() {
    assert_s_parity(
        "s_wrapping_padding_and_substrings",
        r##"(list
              (s-word-wrap 8 "one two three four")
              (s-center 8 "mid")
              (s-center 2 "long")
              (s-pad-left 6 "0" "42")
              (s-pad-right 6 "." "42")
              (s-pad-left 2 "0" "long")
              (s-left 3 "abcdef")
              (s-left 30 "abcdef")
              (s-right 3 "abcdef")
              (s-right 30 "abcdef")
              (s-chop-left 2 "abcdef")
              (s-chop-left 20 "abcdef")
              (s-chop-right 2 "abcdef")
              (s-chop-right 20 "abcdef")
              (s-wrap "value" "\"")
              (s-wrap "value" "<" ">"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_primary_predicates_and_indices() {
    assert_s_parity(
        "s_primary_predicates_and_indices",
        r##"(list
              (s-ends-with? ".el" "init.el")
              (s-ends-with? ".EL" "init.el" t)
              (s-starts-with? "init" "init.el")
              (s-starts-with? "INIT" "init.el" t)
              (s-contains? "it." "init.el")
              (s-contains? "INIT" "init.el" t)
              (s-equals? "same" "same")
              (s-equals? "same" "different")
              (s-less? "a" "b")
              (s-matches? "[0-9]+" "a12b")
              (s-matches? "a" "ba" 2)
              (s-blank? nil)
              (s-blank? "")
              (s-blank-str? " \t\n")
              (s-present? "value")
              (s-presence "")
              (s-presence "value")
              (s-lowercase? "lower 123")
              (s-uppercase? "UPPER 123")
              (s-mixedcase? "Mixed")
              (s-capitalized? "Capitalized")
              (s-numeric? "012345")
              (s-numeric? "12.5")
              (s-index-of "it" "initial")
              (s-index-of "INIT" "initial" t)
              (s-index-of "missing" "initial"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_predicate_compatibility_aliases() {
    assert_s_parity(
        "s_predicate_compatibility_aliases",
        r##"(list
              (s-blank-p nil)
              (s-blank-str-p " ")
              (s-capitalized-p "Capitalized")
              (s-contains-p "nit" "initial")
              (s-ends-with-p ".el" "init.el")
              (s-equals-p "same" "same")
              (s-less-p "a" "b")
              (s-lowercase-p "lower")
              (s-matches-p "[0-9]" "a1")
              (s-mixedcase-p "Mixed")
              (s-numeric-p "123")
              (s-prefix-p "init" "init.el")
              (s-prefix? "init" "init.el")
              (s-present-p "value")
              (s-starts-with-p "init" "init.el")
              (s-suffix-p ".el" "init.el")
              (s-suffix? ".el" "init.el")
              (s-uppercase-p "UPPER"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_replacement_and_case_conversion() {
    assert_s_parity(
        "s_replacement_and_case_conversion",
        r##"(list
              (s-replace "." "/" "one.two.three")
              (s-replace "$" "\\dollar" "$5 + $10")
              (s-replace-regexp "[0-9]+" "#" "a12b345")
              (s-replace-all
               '(("cat" . "dog") ("red" . "blue"))
               "red cat, cat")
              (s-downcase "MiXeD Ä")
              (s-upcase "MiXeD ä")
              (s-capitalize "hELLO WORLD")
              (s-titleize "hELLO-world AGAIN"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_threading_macro() {
    assert_s_parity(
        "s_threading_macro",
        r##"(list
              (s-with "  hello  "
                s-trim
                s-upcase
                (s-prepend "[")
                (s-append "]"))
              (s-with "abc"
                (s-replace "b" "B")
                s-reverse)
              (s-with "abc" s-reverse)
              (macroexpand
               '(s-with value
                  (s-prepend "pre-")
                  (s-append "-post"))))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_reverse_ascii_multibyte_and_graphemes() {
    assert_s_parity(
        "s_reverse_ascii_multibyte_and_graphemes",
        r##"(list
              (s-reverse "abcdef")
              (s-reverse "")
              (s-reverse "åß中")
              (s-reverse (string ?A #x0301 ?B))
              (s-reverse "😀ab")
              (multibyte-string-p (s-reverse "åß中")))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_regexp_matches_and_positions() {
    assert_s_parity(
        "s_regexp_matches_and_positions",
        r##"(list
              (s-match-strings-all
               "\\([a-z]+\\)=\\([0-9]+\\)"
               "x=1;y=22")
              (s-match-strings-all "[0-9]" "no digits")
              (s-matched-positions-all "ana" "bananas")
              (s-matched-positions-all
               "\\([a-z]+\\)=\\([0-9]+\\)"
               "x=1;y=22"
               2)
              (s-match "\\([a-z]+\\)-\\([0-9]+\\)" "id-ab-42!")
              (s-match "\\([a-z]+\\)" "one two" 4)
              (s-match "missing" "value")
              (progn
                (string-match "\\(outer\\)" "outer")
                (let ((before (match-data)))
                  (s-match "\\([0-9]+\\)" "a12b")
                  (equal before (match-data)))))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_slicing_and_word_splitting() {
    assert_s_parity(
        "s_slicing_and_word_splitting",
        r##"(list
              (s-slice-at "[A-Z]" "lowerCamelHTTP")
              (s-slice-at "[0-9]+" "abc12def34")
              (s-slice-at "," "")
              (s-split-words "lowerCamelHTTP2Value")
              (s-split-words "snake_case dashed-words")
              (s-split-words "XMLHttpRequest")
              (s-split-words "")
              (s-split-words "  spaced  words  "))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_word_style_transformations() {
    assert_s_parity(
        "s_word_style_transformations",
        r##"(list
              (s-lower-camel-case "HTTP response code")
              (s-upper-camel-case "http_response-code")
              (s-snake-case "HTTPResponseCode")
              (s-dashed-words "HTTPResponseCode")
              (s-spaced-words "HTTPResponseCode")
              (s-capitalized-words "HTTP_response CODE")
              (s-titleized-words "HTTP_response CODE")
              (s-word-initials "Hyper Text Markup Language")
              (s-word-initials "snake_case"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_format_replacers_and_extra_values() {
    assert_s_parity(
        "s_format_replacers_and_extra_values",
        r##"(progn
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
                  "!"))))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_lexical_format_macro_and_variable() {
    assert_s_parity(
        "s_lexical_format_macro_and_variable",
        r##"(list
              (let ((name "Ada")
                    (count 3))
                (s-lex-format "${name} has ${count} messages"))
              (let ((s-lex-value-as-lisp t)
                    (payload '(a "b")))
                (s-lex-format "payload=${payload}"))
              (s-lex-fmt|expand "${name}:${count}")
              (macroexpand
               '(s-lex-format "${name}:${count}")))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_match_counting_and_wrapping() {
    assert_s_parity(
        "s_match_counting_and_wrapping",
        r##"(list
              (s-count-matches "ana" "bananana")
              (s-count-matches-all "ana" "bananana")
              (s-count-matches "[0-9]" "a1b2c3" 3 6)
              (s-count-matches-all "[0-9]" "a1b2c3" 3 6)
              (s-count-matches "^" "abc")
              (s-count-matches-all "^" "abc")
              (s-wrap "value" "*")
              (s-wrap "value" "<" ">")
              (s-wrap "" "[" "]"))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_empty_and_boundary_values() {
    assert_s_parity(
        "s_empty_and_boundary_values",
        r##"(list
              (s-join "," nil)
              (s-concat)
              (s-repeat -1 "x")
              (s-chop-prefix "" "value")
              (s-chop-suffix "" "value")
              (s-shared-start "" "")
              (s-shared-end "" "")
              (s-center 0 "")
              (s-pad-left 0 "." "")
              (s-pad-right 0 "." "")
              (s-left 0 "abc")
              (s-right 0 "abc")
              (s-chop-left 0 "abc")
              (s-chop-right 0 "abc")
              (s-presence nil)
              (s-lowercase? "")
              (s-uppercase? "")
              (s-mixedcase? "")
              (s-capitalized? "")
              (s-numeric? ""))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_unresolved_format_variable_signal() {
    assert_s_signal_parity(
        "s_unresolved_format_variable_signal",
        r##"(s-format "Hello ${missing}" (lambda (_key) nil))"##,
    );
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned s below ./tmp"]
fn s_splice_out_of_range_signal() {
    assert_s_signal_parity(
        "s_splice_out_of_range_signal",
        r##"(s-splice "X" 99 "abc")"##,
    );
}
