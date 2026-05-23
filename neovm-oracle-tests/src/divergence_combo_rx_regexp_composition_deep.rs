//! Deep combo: rx macro + regexp composition + string-match + replace.
//! Tests the rx DSL building complex regexps from composable parts.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_rx_basic_composition() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx bos (+ (or alpha digit)) \"@\"\n\
         (+ (or alpha digit)) \".\" (+ alpha) eos)))\n\
         (list re\n\
         (string-match re \"user@host.com\")\n\
         (string-match re \"not-an-email\"))))",
    );
}

#[test]
fn deficiency_rx_with_group_and_backref() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx (group (+ (not (in \" \\t\\n\")))\n\
         \" \"\n\
         (backref 1)))))\n\
         (list (string-match re \"hello hello\")\n\
         (string-match re \"hello world\")\n\
         (match-string 1 \"hello hello\"))))",
    );
}

#[test]
fn deficiency_rx_char_classes_and_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx (+ (any alpha digit)))))\n\
         (list (string-match re \"abc123\")\n\
         (string-match re \"!@#\")\n\
         (match-string 0 \"abc123\"))))",
    );
}

#[test]
fn deficiency_rx_complement_and_negation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx (+ (not (in \":\"))) \":\" (+ digit))))\n\
         (list (string-match re \"key:42\")\n\
         (string-match re \"no-colon\")\n\
         (match-string 0 \"key:42\"))))",
    );
}

#[test]
fn deficiency_rx_or_sequence_alternation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx (or \"hello\" \"world\" \"test\"))))\n\
         (list (string-match re \"say hello there\")\n\
         (string-match re \"the world is big\")\n\
         (string-match re \"no match\"))))",
    );
}

#[test]
fn deficiency_rx_repeat_operators() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx (= 3 digit) \"-\" (= 4 digit))))\n\
         (list (string-match re \"123-4567\")\n\
         (string-match re \"12-4567\")\n\
         (string-match re \"123-456\"))))",
    );
}

#[test]
fn deficiency_rx_line_anchors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx bol (+ alpha) eol)))\n\
         (list (string-match re \"hello\")\n\
         (string-match re \"hello world\")\n\
         (string-match re \"123\"))))",
    );
}

#[test]
fn deficiency_rx_word_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((re (rx bow \"test\" eow)))\n\
         (list (string-match re \"a test here\")\n\
         (string-match re \"testing\")\n\
         (string-match re \"atest\"))))",
    );
}

#[test]
fn deficiency_rx_with_eval_and_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((prefix \"my-\"))\n\
         (let ((re (rx-to-string '(eval prefix) (+ alpha)))))\n\
         (list re\n\
         (string-match re \"my-function\")\n\
         (string-match re \"other-function\"))))",
    );
}

#[test]
fn deficiency_rx_composed_search_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((buf (generate-new-buffer \"rxs\")))\n\
         (with-current-buffer buf\n\
         (insert \"TODO: fix bug FIXME: update docs TODO: review\")\n\
         (put-text-property 1 5 'type 'todo)\n\
         (put-text-property 15 20 'type 'fixme)\n\
         (put-text-property 33 37 'type 'todo)\n\
         (goto-char 1)\n\
         (let ((count 0))\n\
         (while (re-search-forward (rx (or \"TODO\" \"FIXME\")) nil t)\n\
         (replace-match (format \"[%s:%d]\" (match-string 0) (cl-incf count))))\n\
         (list count (buffer-string)\n\
         (get-text-property 1 'type))))\n\
         (kill-buffer buf)))",
    );
}
