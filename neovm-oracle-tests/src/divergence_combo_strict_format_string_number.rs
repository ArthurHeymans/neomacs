//! Strict combo oracle probes: numeric/string formatting, regex replace,
//! coding, hashing, cl-lib accumulators, and narrowed-buffer text-property
//! combos.  These target edge cases that single-feature files tend to miss:
//! %g cutoffs, %f rounding, bignum stringification, split-string trimming,
//! replace-regexp-in-string fixed-case/subexp/function, format-spec flags,
//! CJK char/string widths, secure-hash family, regexp-opt grouping, cl-loop
//! accumulators, and cl-destructuring-bind/&rest.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- %g / %e / %f precision and rounding -----------------------------------

#[test]
fn div_fsn_g_cutoffs_and_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%g" 1e-3)
      (format "%g" 1e-4)
      (format "%g" 1e-5)
      (format "%g" 1.0)
      (format "%g" 100000.0)
      (format "%g" 1000000.0)
      (format "%g" 0.00001)
      (format "%g" 123456789.0)
      (format "%.10g" 0.1))
"##,
    );
}

#[test]
fn div_fsn_f_e_rounding_and_flags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%.0f" 0.5)
      (format "%.0f" 1.5)
      (format "%.0f" 2.5)
      (format "%.0f" 3.5)
      (format "%.2f" 1.005)
      (format "%.2f" 2.675)
      (format "%e" 0.0)
      (format "%.3e" 123456.789)
      (format "%010.3f" 3.14159)
      (format "%+.2e" -3.14159))
"##,
    );
}

#[test]
fn div_fsn_integer_format_flags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%+d" 42)
      (format "% d" 42)
      (format "%05d" 42)
      (format "%-5d|" 42)
      (format "%#o" 64)
      (format "%#x" 255)
      (format "%x" 3735928559)
      (format "%#X" 255)
      (format "%o" 64)
      (format "%b" 42))
"##,
    );
}

// --- bignum stringification and string-to-number --------------------------

#[test]
fn div_fsn_bignum_and_base_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (number-to-string 100000000000000000000)
      (format "%d" 1000000000000000000000)
      (format "%x" 1000000000000000000)
      (format "%o" 1000000000000000000)
      (string-to-number "100000000000000000000")
      (string-to-number "ff" 16)
      (string-to-number "1010" 2)
      (string-to-number "z" 36)
      (string-to-number "1e3")
      (string-to-number "  -17  "))
"##,
    );
}

#[test]
fn div_fsn_char_format_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%c" 65)
      (format "%c" 128578)
      (char-to-string 128578)
      (string (list 65 66 67))
      (format "%c" 945))
"##,
    );
}

// --- split-string trimming / omit-nulls ------------------------------------

#[test]
fn div_fsn_split_string_trim_and_omit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (split-string "  a  b  c  " " +" t)
      (split-string "  a  b  c  " " +" nil)
      (split-string ",,a,,b,," "," nil t)
      (split-string ",,a,,b,," "," nil nil)
      (split-string "aa||bb||cc" "|")
      (split-string "aa||bb||cc" "|" t)
      (split-string "Remove trailing
" "\n+" t))
"##,
    );
}

// --- replace-regexp-in-string: fixed-case, function, subexp ---------------

#[test]
fn div_fsn_replace_regexp_in_string_modes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (replace-regexp-in-string "[0-9]+" "#" "a1b22c333")
      (replace-regexp-in-string "[a-z]" "*" "AbCdEf")
      (replace-regexp-in-string "\\b\\w" 'upcase "hello world")
      (replace-regexp-in-string "foo" "bar" "Foo foo FOO" t)
      (replace-regexp-in-string "foo" "bar" "Foo foo FOO")
      (replace-regexp-in-string "\\([0-9]\\)" "[\\1]" "a1b2")
      (replace-regexp-in-string "o" "0" "foooo"))
"##,
    );
}

#[test]
fn div_fsn_replace_match_subexp_and_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (progn (string-match "ell" "hello") (replace-match "ELL" nil nil "hello"))
      (progn (string-match "\\(o\\)" "foo") (replace-match "0" t t "foo" 1))
      (progn (string-match "x" "axb") (match-data t))
      (progn (string-match "\\(a\\)\\(b\\)" "xabz")
             (list (match-beginning 0) (match-end 0)
                   (match-beginning 1) (match-end 1)
                   (match-beginning 2) (match-end 2)
                   (match-substring 1)))
      (replace-regexp-in-string "\\([a-z]\\)\\1" "<\\1\\1>" "aabbc"))
"##,
    );
}

// --- format-spec width / flags / escaping ---------------------------------

#[test]
fn div_fsn_format_spec_flags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format-spec "%a-%b-%c" '((?a . "x") (?b . "y") (?c . "z")))
      (format-spec "%2a|%2b" '((?a . "x") (?b . "yz")))
      (format-spec "%-2a|%2b" '((?a . "x") (?b . "y")))
      (format-spec "100%% done: %p" '((?p . "yes")))
      (format-spec "[%4a]" '((?a . "x")))
      (format-spec "[%-4a]" '((?a . "x"))))
"##,
    );
}

// --- CJK char/string width -------------------------------------------------

#[test]
fn div_fsn_cjk_char_string_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (char-width ?a)
      (string-width "abc")
      (char-width ?一)
      (string-width "a一b")
      (char-width ?\N{TIBETAN VOWEL SIGN AA})
      (string-width "日本語テスト"))
"##,
    );
}

// --- secure-hash family ----------------------------------------------------

#[test]
fn div_fsn_secure_hash_family() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (secure-hash 'md5 "abc")
      (secure-hash 'sha1 "abc")
      (secure-hash 'sha224 "abc")
      (secure-hash 'sha256 "abc")
      (secure-hash 'sha384 "abc")
      (secure-hash 'sha512 "abc")
      (sha1 "abc")
      (md5 "abc")
      (secure-hash 'sha1 "")
      (secure-hash 'sha256 "The quick brown fox jumps over the lazy dog"))
"##,
    );
}

// --- regexp-opt grouping / regexp-quote ------------------------------------

#[test]
fn div_fsn_regexp_opt_and_quote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (regexp-opt '("foo" "bar" "baz"))
      (regexp-opt '("foo" "bar" "baz") t)
      (regexp-quote "a.b*c+d?")
      (regexp-opt '("cat" "category" "catalog") nil)
      (regexp-opt '("x" "xx" "xxx") t))
"##,
    );
}

// --- base64 roundtrips -----------------------------------------------------

#[test]
fn div_fsn_base64_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (base64-encode-string "hello world")
      (base64-encode-string "hello world" t)
      (base64-decode-string (base64-encode-string "hello"))
      (base64url-encode-string "??")
      (base64url-encode-string "a/b+c="))
"##,
    );
}

// --- cl-loop accumulators --------------------------------------------------

#[test]
fn div_fsn_cl_loop_accumulators() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-loop for x in '(1 2 3 4) sum x)
      (cl-loop for x in '(1 2 3 4) count (= (% x 2) 0))
      (cl-loop for x in '(1 2 3) maximize x into m finally (return m))
      (cl-loop for x in '(1 2 3) minimize x into m finally (return m))
      (cl-loop for x on '(1 2 3) collect (length x))
      (cl-loop for i from 1 to 10 by 2 collect i)
      (cl-loop for i from 10 downto 1 by 3 collect i)
      (cl-loop for x across [1 2 3] collect (* x x))
      (cl-loop for x in '(1 2 3) append (list x x))
      (cl-loop for x in '(1 2 3) sum (* x x)))
"##,
    );
}

#[test]
fn div_fsn_cl_destructure_labels_coerce() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-destructuring-bind (a (b c) &rest d) '(1 (2 3) 4 5) (list a b c d))
      (cl-destructuring-bind (&key a b) '(:a 1 :b 2) (list a b))
      (cl-labels ((fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))) (fact 6))
      (cl-coerce "abc" 'list)
      (cl-coerce 65 'character)
      (cl-remove-duplicates '(1 2 1 3 2 4) :test #'=)
      (cl-sort (list 3 1 2) #'<)
      (cl-subseq "abcdef" 1 4)
      (cl-position 2 '(1 2 3 2 1))
      (cl-substitute 9 2 '(1 2 3 2 1)))
"##,
    );
}

// --- string comparison / version ------------------------------------------

#[test]
fn div_fsn_compare_and_version() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (compare-strings "abcdef" 0 6 "abcxyz" 0 3)
      (compare-strings "abcdef" 0 6 "abcdef" 0 6)
      (compare-strings "abcdef" 0 6 "ABCDEF" 0 6)
      (compare-strings "abcdef" 0 6 "ABCDEF" 0 6 t)
      (string-version-lessp "foo2" "foo10")
      (string-version-lessp "1.0" "1.10")
      (string-lessp "abc" "abd")
      (string-lessp "abc" "abc")
      (version-list-< '(1 2 3) '(1 2 10)))
"##,
    );
}

// --- concat / store-substring / make-string / coding ----------------------

#[test]
fn div_fsn_string_ops_and_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (store-substring "abcdef" 2 ?X)
      (make-string 5 ?*)
      (make-string 3 128578)
      (concat "a" (make-string 2 ?b) "c")
      (substring "abcdef" 1 -2)
      (substring-no-properties (propertize "abc" 'face 'bold) 0 2)
      (string-make-unibyte (string 200))
      (length (string-make-unibyte (string 200)))
      (encode-coding-string "café" 'utf-8)
      (length (encode-coding-string "café" 'utf-8))
      (decode-coding-string (encode-coding-string "café" 'utf-8) 'utf-8)
      (encode-coding-string (string 200) 'iso-8859-1)
      (length (encode-coding-string (string 200) 'iso-8859-1)))
"##,
    );
}

// --- narrowed buffer + text-property combo --------------------------------

#[test]
fn div_fsn_narrow_textprop_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghij")
  (add-text-properties 2 6 '(face bold weight heavy))
  (narrow-to-region 3 8)
  (list (buffer-string)
        (point-min) (point-max)
        (text-properties-at 4)
        (get-text-property 4 'face)
        (next-single-property-change 3 'face)
        (previous-single-property-change 7 'face)))
"##,
    );
}

// --- mapcar/mapconcat over mixed types -------------------------------------

#[test]
fn div_fsn_mapcar_mapconcat_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (mapcar #'1+ '(1 2 3))
      (mapcar #'car '((1 . 2) (3 . 4)))
      (mapconcat #'identity '("a" "b" "c") "-")
      (mapconcat #'number-to-string '(1 2 3) ",")
      (mapcar #'identity "abc")
      (mapcar #'char-to-string "ABC")
      (apply #'concat (mapcar #'char-to-string "xyz")))
"##,
    );
}
