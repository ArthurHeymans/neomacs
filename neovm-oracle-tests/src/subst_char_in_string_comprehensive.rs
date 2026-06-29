//! Comprehensive oracle parity tests for `subst-char-in-string`:
//! all parameter combinations (OLD-CHAR NEW-CHAR STRING &optional INPLACE),
//! multibyte characters, identity substitution, multiple chained substitutions,
//! inplace vs copy semantics, empty strings, strings with only target chars,
//! Unicode codepoint edge cases, and combination with other string operations.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// Basic substitution: ASCII chars, single occurrence, multiple occurrences
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_basic_single_and_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Single occurrence
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?a ?z "abcdef")"#,
        expect_test::expect![[r#""OK \"zbcdef\"""#]],
    );
    // Multiple occurrences
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?a ?z "banana")"#,
        expect_test::expect![[r#""OK \"bznznz\"""#]],
    );
    // All characters are the target
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?x ?y "xxxxx")"#,
        expect_test::expect![[r#""OK \"yyyyy\"""#]],
    );
    // No occurrences of target
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?z ?a "hello world")"#,
        expect_test::expect![[r#""OK \"hello world\"""#]],
    );
    // Single character string
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?a ?b "a")"#,
        expect_test::expect![[r#""OK \"b\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?a ?b "z")"#,
        expect_test::expect![[r#""OK \"z\"""#]],
    );
    // Empty string
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?a ?b "")"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    // First and last characters
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?h ?H "hello")"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?o ?O "hello")"#,
        expect_test::expect![[r#""OK \"hellO\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Identity substitution: same old and new char
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_identity_substitution() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?a ?a "abcabc")"#,
        expect_test::expect![[r#""OK \"abcabc\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?x ?x "")"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?z ?z "zzz")"#,
        expect_test::expect![[r#""OK \"zzz\"""#]],
    );

    // Verify the result is equal to original
    let form = r#"(let ((s "hello world"))
                    (equal s (subst-char-in-string ?x ?x s)))"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK t""#]]);
}

// ---------------------------------------------------------------------------
// Inplace parameter: copy vs in-place behavior
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_inplace_vs_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Without inplace (default): returns a new string, original unchanged
    let form = r#"(let* ((original "hello")
                         (result (subst-char-in-string ?l ?r original)))
                    (list result original (eq result original)))"#;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK (\"herro\" \"hello\" nil)""#]],
    );

    // With inplace = nil (explicit): same as default
    let form2 = r#"(let* ((original "hello")
                          (result (subst-char-in-string ?l ?r original nil)))
                     (list result original (eq result original)))"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[r#""OK (\"herro\" \"hello\" nil)""#]],
    );

    // With inplace = t: modifies in place, returns the same object
    let form3 = r#"(let* ((original (copy-sequence "hello"))
                          (result (subst-char-in-string ?l ?r original t)))
                     (list result original (eq result original)))"#;
    crate::common::assert_oracle_parity_expect(
        form3,
        expect_test::expect![[r#""OK (\"herro\" \"herro\" t)""#]],
    );

    // Inplace with no matches: still returns same object
    let form4 = r#"(let* ((original (copy-sequence "hello"))
                          (result (subst-char-in-string ?z ?a original t)))
                     (list result original (eq result original)))"#;
    crate::common::assert_oracle_parity_expect(
        form4,
        expect_test::expect![[r#""OK (\"hello\" \"hello\" t)""#]],
    );
}

// ---------------------------------------------------------------------------
// Special ASCII characters: space, newline, tab, null
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_special_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Replace space with underscore
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?\s ?_ "hello world foo")"#,
        expect_test::expect![[r#""OK \"hello_world_foo\"""#]],
    );
    // Replace underscore with space
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?_ ?\s "hello_world_foo")"#,
        expect_test::expect![[r#""OK \"hello world foo\"""#]],
    );
    // Replace newline with space
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?\n ?\s "line1\nline2\nline3")"#,
        expect_test::expect![[r#""OK \"line1 line2 line3\"""#]],
    );
    // Replace tab with space
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?\t ?\s "col1\tcol2\tcol3")"#,
        expect_test::expect![[r#""OK \"col1 col2 col3\"""#]],
    );
    // Replace space with newline
    crate::common::assert_oracle_parity_expect(
        r#"(subst-char-in-string ?\s ?\n "a b c")"#,
        expect_test::expect![[r#""OK \"a\nb\nc\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Multibyte / Unicode characters
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_multibyte_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Replace ASCII in multibyte string
    let form = r#"(subst-char-in-string ?a ?z "café")"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK \"czfé\"""#]]);

    // Replace multibyte char with ASCII
    let form2 = r#"(subst-char-in-string ?é ?e "café")"#;
    crate::common::assert_oracle_parity_expect(form2, expect_test::expect![[r#""OK \"cafe\"""#]]);

    // Replace multibyte with multibyte
    let form3 = r#"(subst-char-in-string ?ü ?u "grüße")"#;
    crate::common::assert_oracle_parity_expect(form3, expect_test::expect![[r#""OK \"gruße\"""#]]);

    // CJK characters
    let form4 = r#"(subst-char-in-string ?世 ?地 "世界世界")"#;
    crate::common::assert_oracle_parity_expect(
        form4,
        expect_test::expect![[r#""OK \"地界地界\"""#]],
    );

    // Emoji replacement
    let form5 = r#"(subst-char-in-string ?a ?b "abc")"#;
    crate::common::assert_oracle_parity_expect(form5, expect_test::expect![[r#""OK \"bbc\"""#]]);

    // String entirely of multibyte chars
    let form6 = r#"(subst-char-in-string ?α ?β "αγαδα")"#;
    crate::common::assert_oracle_parity_expect(form6, expect_test::expect![[r#""OK \"βγβδβ\"""#]]);

    // Mixed ASCII and Unicode, no match
    let form7 = r#"(subst-char-in-string ?z ?a "héllo wörld")"#;
    crate::common::assert_oracle_parity_expect(
        form7,
        expect_test::expect![[r#""OK \"héllo wörld\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Chained substitutions
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_chained_substitutions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Chain multiple substitutions: ROT13-like
    let form = r#"(let ((s "hello"))
                    (setq s (subst-char-in-string ?h ?H s))
                    (setq s (subst-char-in-string ?e ?E s))
                    (setq s (subst-char-in-string ?l ?L s))
                    (setq s (subst-char-in-string ?o ?O s))
                    s)"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK \"HELLO\"""#]]);

    // Swap two characters using temporary
    let form2 = r#"(let ((s "aXbXc"))
                     (setq s (subst-char-in-string ?X ?~ s))
                     (setq s (subst-char-in-string ?a ?X s))
                     (setq s (subst-char-in-string ?~ ?a s))
                     s)"#;
    crate::common::assert_oracle_parity_expect(form2, expect_test::expect![[r#""OK \"Xabac\"""#]]);

    // Replace all vowels with dots, one at a time
    let form3 = r#"(let ((s "beautiful"))
                     (setq s (subst-char-in-string ?a ?. s))
                     (setq s (subst-char-in-string ?e ?. s))
                     (setq s (subst-char-in-string ?i ?. s))
                     (setq s (subst-char-in-string ?o ?. s))
                     (setq s (subst-char-in-string ?u ?. s))
                     s)"#;
    crate::common::assert_oracle_parity_expect(
        form3,
        expect_test::expect![[r#""OK \"b...t.f.l\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Combination with other string operations
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_combined_with_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // subst then concat
    let form = r#"(concat (subst-char-in-string ?- ?_ "foo-bar")
                          "."
                          (subst-char-in-string ?- ?_ "baz-qux"))"#;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK \"foo_bar.baz_qux\"""#]],
    );

    // subst on substring result
    let form2 = r#"(subst-char-in-string ?l ?r (substring "hello world" 0 5))"#;
    crate::common::assert_oracle_parity_expect(form2, expect_test::expect![[r#""OK \"herro\"""#]]);

    // upcase then subst
    let form3 = r#"(subst-char-in-string ?L ?* (upcase "hello"))"#;
    crate::common::assert_oracle_parity_expect(form3, expect_test::expect![[r#""OK \"HE**O\"""#]]);

    // subst in mapconcat result
    let form4 = r#"(subst-char-in-string ?, ?\s
                     (mapconcat #'number-to-string '(1 2 3 4 5) ","))"#;
    crate::common::assert_oracle_parity_expect(
        form4,
        expect_test::expect![[r#""OK \"1 2 3 4 5\"""#]],
    );

    // string-to-list after subst
    let form5 = r#"(let ((s (subst-char-in-string ?a ?z "abcabc")))
                     (list s (length s) (string-to-char s)))"#;
    crate::common::assert_oracle_parity_expect(
        form5,
        expect_test::expect![[r#""OK (\"zbczbc\" 6 122)""#]],
    );

    // Nested subst-char calls
    let form6 = r#"(subst-char-in-string ?b ?c
                     (subst-char-in-string ?a ?b "aaa"))"#;
    crate::common::assert_oracle_parity_expect(form6, expect_test::expect![[r#""OK \"ccc\"""#]]);
}

// ---------------------------------------------------------------------------
// Boundary: very long strings, repeated chars, format-like usage
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_boundary_and_performance() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Long repeated string
    let form = r#"(let ((s (make-string 200 ?x)))
                    (let ((result (subst-char-in-string ?x ?y s)))
                      (list (length result)
                            (= (length result) 200)
                            (string= result (make-string 200 ?y)))))"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK (200 t t)""#]]);

    // Alternating characters
    let form2 = r#"(let ((s (mapconcat (lambda (i)
                                          (if (= (% i 2) 0) "a" "b"))
                                        (number-sequence 0 19) "")))
                     (list s
                           (subst-char-in-string ?a ?x s)
                           (subst-char-in-string ?b ?y s)))"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[
            r#""OK (\"abababababababababab\" \"xbxbxbxbxbxbxbxbxbxb\" \"ayayayayayayayayayay\")""#
        ]],
    );

    // Substitute in a formatted string
    let form3 = r#"(subst-char-in-string ?/ ?\\
                     (format "%s/%s/%s" "path" "to" "file"))"#;
    crate::common::assert_oracle_parity_expect(
        form3,
        expect_test::expect![[r#""OK \"path\\\\to\\\\file\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Return value and type checks
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_return_type_checks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Always returns a string
    let form = r#"(list
                    (stringp (subst-char-in-string ?a ?b "abc"))
                    (stringp (subst-char-in-string ?x ?y ""))
                    (stringp (subst-char-in-string ?a ?b "zzz")))"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK (t t t)""#]]);

    // Length preserved
    let form2 = r#"(let ((s "hello world"))
                     (= (length s)
                        (length (subst-char-in-string ?o ?0 s))))"#;
    crate::common::assert_oracle_parity_expect(form2, expect_test::expect![[r#""OK t""#]]);

    // Compare original and result
    let form3 = r#"(let* ((s "test")
                          (r (subst-char-in-string ?z ?a s)))
                     (list (string= s r)
                           (equal s r)))"#;
    crate::common::assert_oracle_parity_expect(form3, expect_test::expect![[r#""OK (t t)""#]]);
}

// ---------------------------------------------------------------------------
// Integration: path manipulation, CSV transform, encoding-like ops
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_integration_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Path separator conversion (Unix to Windows style)
    let form = r#"(subst-char-in-string ?/ ?\\ "/usr/local/bin")"#;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK \"\\\\usr\\\\local\\\\bin\"""#]],
    );

    // Simple Caesar-shift-like substitution chain
    let form2 = r#"(let ((s "abc"))
                     (setq s (subst-char-in-string ?a ?b s))
                     (setq s (subst-char-in-string ?b ?c s))
                     (setq s (subst-char-in-string ?c ?d s))
                     s)"#;
    crate::common::assert_oracle_parity_expect(form2, expect_test::expect![[r#""OK \"ddd\"""#]]);

    // Replace delimiters for CSV-like transform
    let form3 = r#"(subst-char-in-string ?\t ?,
                     "name\tage\tcity")"#;
    crate::common::assert_oracle_parity_expect(
        form3,
        expect_test::expect![[r#""OK \"name,age,city\"""#]],
    );

    // Sanitize filename: replace spaces with hyphens
    let form4 = r#"(subst-char-in-string ?\s ?-
                     "my document name.txt")"#;
    crate::common::assert_oracle_parity_expect(
        form4,
        expect_test::expect![[r#""OK \"my-document-name.txt\"""#]],
    );

    // Multiple passes to normalize whitespace types to space
    let form5 = r#"(let ((s "hello\tworld\nfoo"))
                     (setq s (subst-char-in-string ?\t ?\s s))
                     (setq s (subst-char-in-string ?\n ?\s s))
                     s)"#;
    crate::common::assert_oracle_parity_expect(
        form5,
        expect_test::expect![[r#""OK \"hello world foo\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Char code edge cases: 0, 127, high codepoints
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_codepoint_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // DEL char (127)
    let form = r#"(subst-char-in-string 127 ?X (string 65 127 66 127 67))"#;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK \"AXBXC\"""#]]);

    // High ASCII
    let form2 = r#"(subst-char-in-string ?~ ?! "a~b~c")"#;
    crate::common::assert_oracle_parity_expect(form2, expect_test::expect![[r#""OK \"a!b!c\"""#]]);

    // Char 1 (control char)
    let form3 = r#"(subst-char-in-string 1 ?A (string 1 65 1 66))"#;
    crate::common::assert_oracle_parity_expect(form3, expect_test::expect![[r#""OK \"AAAB\"""#]]);

    // Replace char with same codepoint using numeric notation
    let form4 = r#"(subst-char-in-string 65 66 "ABCABC")"#;
    crate::common::assert_oracle_parity_expect(form4, expect_test::expect![[r#""OK \"BBCBBC\"""#]]);

    // Large Unicode codepoints
    let form5 = r#"(subst-char-in-string #x2603 #x2764 (string #x2603 65 #x2603))"#;
    crate::common::assert_oracle_parity_expect(form5, expect_test::expect![[r#""OK \"❤A❤\"""#]]);
}

// ---------------------------------------------------------------------------
// Comprehensive multi-aspect test
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_subst_char_comprehensive_multi_aspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"(list
      ;; Basic operations
      (subst-char-in-string ?a ?z "abracadabra")
      ;; Empty string
      (subst-char-in-string ?x ?y "")
      ;; No match
      (subst-char-in-string ?q ?z "hello")
      ;; All match
      (subst-char-in-string ?a ?b "aaaa")
      ;; Single char string, match
      (subst-char-in-string ?x ?y "x")
      ;; Single char string, no match
      (subst-char-in-string ?x ?y "z")
      ;; Unicode
      (subst-char-in-string ?ö ?o "schön")
      ;; Space to underscore
      (subst-char-in-string ?\s ?_ "a b c d")
      ;; Chained: shift a->b->c
      (let ((s "axbxc"))
        (subst-char-in-string ?x ?- s))
      ;; Copy semantics check
      (let* ((s "test")
             (r (subst-char-in-string ?t ?T s)))
        (list s r (string= s "test")))
      ;; Length preservation
      (let ((s "hello world"))
        (= (length (subst-char-in-string ?l ?L s)) (length s))))"#;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK (\"zbrzczdzbrz\" \"\" \"hello\" \"bbbb\" \"y\" \"z\" \"schon\" \"a_b_c_d\" \"a-b-c\" (\"test\" \"TesT\" t) t)""#
        ]],
    );
}
