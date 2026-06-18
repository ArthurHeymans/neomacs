//! Regex match-data parity: explicit numbered groups (\(?N:...\)), shy
//! groups, match-data (integers/markers/reuse), set/save-match-data,
//! repetition bounds \{n,m\}, anchors \`/\'/\_</\_>; while-let; plus the
//! replace-region-contents function-argument divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn md_explicit_numbered_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (string-match "\\(?2:[a-z]+\\)\\(?1:[0-9]+\\)" "abc123")
  (list (match-string 1 "abc123") (match-string 2 "abc123")
        (match-beginning 1) (match-end 2)))"##,
    );
}

#[test]
fn md_match_data_integers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (string-match "\\(a\\)\\(b\\)" "xaby")
  (let ((md (match-data)))
    (list md (length md) (match-data t))))"##,
    );
}

#[test]
fn md_match_data_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello world")
  (goto-char (point-min))
  (re-search-forward "\\(wor\\)ld" nil t)
  (let ((md (match-data t)))
    (list (integerp (nth 0 md)) (match-string 1)
          (let ((mm (match-data nil (list nil nil)))) (markerp (nth 0 (progn (goto-char 1) (re-search-forward "hello") (match-data t t))))))))"##,
    );
}

#[test]
fn md_regex_alternation_anchors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-match "\\`foo" "foobar")
        (string-match "bar\\'" "foobar")
        (progn (string-match "\\(?:cat\\|dog\\)s?" "cats") (match-end 0))
        (string-match "\\_<word\\_>" "a word b"))"##,
    );
}

#[test]
fn md_regex_repetition_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (progn (string-match "a\\{2,3\\}" "aaaa") (match-end 0))
        (progn (string-match "a\\{2,\\}" "aaaa") (match-end 0))
        (progn (string-match "a\\{,2\\}" "aaaa") (match-end 0))
        (string-match "a\\{0\\}b" "b"))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: replace-region-contents rejects a function REPLACE-FN (signals wrong-type-argument expecting string/buffer/vector); GNU calls the function to obtain the replacement buffer/string."]
fn divergence_replace_region_contents_fn() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((src (generate-new-buffer " neo-rrc-xxx")))
  (with-current-buffer src (insert "REPLACED"))
  (prog1 (with-temp-buffer (insert "original text")
           (replace-region-contents (point-min) (point-max) (lambda () src))
           (buffer-string))
    (kill-buffer src)))"##,
    );
}

#[test]
fn md_save_match_data_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (string-match "foo" "foobar")
  (save-match-data (string-match "bar" "bar"))
  (list (match-beginning 0) (match-string 0 "foobar")))"##,
    );
}

#[test]
fn md_set_match_data_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (string-match "abc" "xabcy")
  (let ((saved (match-data)))
    (string-match "z" "z")
    (set-match-data saved)
    (list (match-beginning 0) (match-end 0))))"##,
    );
}

#[test]
fn md_shy_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (string-match "\\(?:ab\\)+\\(c\\)" "ababc")
  (list (match-string 0 "ababc") (match-string 1 "ababc") (match-beginning 1)))"##,
    );
}

#[test]
fn md_while_let_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((data '(1 2 3 nil 5)) (acc nil))
  (while-let ((x (pop data)) ((numberp x)))
    (push (* x x) acc))
  (nreverse acc))"##,
    );
}
