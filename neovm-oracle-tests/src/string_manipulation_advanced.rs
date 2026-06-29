//! Oracle parity tests for advanced string manipulation:
//! substring with negative indices, concat with chars, split-string params,
//! string-join, string-trim with custom chars, string-prefix-p / string-suffix-p,
//! upcase-initials, and complex path manipulation.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// substring with negative indices (from end)
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_substring_negative_indices() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Negative FROM counts from end
    crate::common::assert_oracle_parity_expect(
        r#"(substring "hello world" -5)"#,
        expect_test::expect![[r#""OK \"world\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(substring "hello world" -5 -1)"#,
        expect_test::expect![[r#""OK \"worl\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(substring "abcdef" -3)"#,
        expect_test::expect![[r#""OK \"def\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(substring "abcdef" -4 -1)"#,
        expect_test::expect![[r#""OK \"cde\"""#]],
    );

    // Negative TO only
    crate::common::assert_oracle_parity_expect(
        r#"(substring "hello world" 0 -1)"#,
        expect_test::expect![[r#""OK \"hello worl\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(substring "hello world" 0 -6)"#,
        expect_test::expect![[r#""OK \"hello\"""#]],
    );

    // Both negative
    crate::common::assert_oracle_parity_expect(
        r#"(substring "abcdefgh" -6 -2)"#,
        expect_test::expect![[r#""OK \"cdef\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(substring "abcdefgh" -1)"#,
        expect_test::expect![[r#""OK \"h\"""#]],
    );

    // Edge: last char
    crate::common::assert_oracle_parity_expect(
        r#"(substring "x" -1)"#,
        expect_test::expect![[r#""OK \"x\"""#]],
    );

    // Combining positive start with negative end
    crate::common::assert_oracle_parity_expect(
        r#"(substring "hello world" 3 -3)"#,
        expect_test::expect![[r#""OK \"lo wo\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(substring "abcdef" 1 -1)"#,
        expect_test::expect![[r#""OK \"bcde\"""#]],
    );

    // Full string via negative
    crate::common::assert_oracle_parity_expect(
        r#"(substring "test" -4)"#,
        expect_test::expect![[r#""OK \"test\"""#]],
    );
}

// ---------------------------------------------------------------------------
// concat with many args including chars
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_concat_many_args_with_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Basic multi-arg concat
    crate::common::assert_oracle_parity_expect(
        r#"(concat "a" "b" "c" "d" "e")"#,
        expect_test::expect![[r#""OK \"abcde\"""#]],
    );

    // Mix strings and empty strings
    crate::common::assert_oracle_parity_expect(
        r#"(concat "" "hello" "" " " "" "world" "")"#,
        expect_test::expect![[r#""OK \"hello world\"""#]],
    );

    // Concat with nil (nil is ignored in concat)
    crate::common::assert_oracle_parity_expect(
        r#"(concat "a" nil "b" nil "c")"#,
        expect_test::expect![[r#""OK \"abc\"""#]],
    );

    // Concat with lists of chars
    crate::common::assert_oracle_parity_expect(
        r#"(concat '(72 101 108 108 111))"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(concat "He" '(108 108) "o")"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );

    // Concat with vectors of chars
    crate::common::assert_oracle_parity_expect(
        r#"(concat [72 101 108 108 111])"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(concat "He" [108 108] "o")"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );

    // Many small strings
    let form = r####"(let ((parts nil))
                    (dotimes (i 10)
                      (setq parts (cons (number-to-string i) parts)))
                    (apply #'concat (nreverse parts)))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK \"0123456789\"""#]],
    );

    // Zero args
    crate::common::assert_oracle_parity_expect(
        r#"(concat)"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
}

// ---------------------------------------------------------------------------
// split-string with all params (STRING, SEPARATORS, OMIT-NULLS, TRIM)
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_split_string_full_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Default separator (whitespace)
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "  hello   world  ")"#,
        expect_test::expect![[r#""OK (\"hello\" \"world\")""#]],
    );

    // Custom separator
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "a,b,,c,d" ",")"#,
        expect_test::expect![[r#""OK (\"a\" \"b\" \"\" \"c\" \"d\")""#]],
    );

    // OMIT-NULLS = nil (keep empty strings)
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "a,,b,,c" "," nil)"#,
        expect_test::expect![[r#""OK (\"a\" \"\" \"b\" \"\" \"c\")""#]],
    );

    // OMIT-NULLS = t (remove empty strings)
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "a,,b,,c" "," t)"#,
        expect_test::expect![[r#""OK (\"a\" \"b\" \"c\")""#]],
    );

    // TRIM parameter
    crate::common::assert_oracle_parity_expect(
        r#"(split-string " a , b , c " "," t " ")"#,
        expect_test::expect![[r#""OK (\"a\" \"b\" \"c\")""#]],
    );

    // Multi-character separator regex
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "one--two--three" "--")"#,
        expect_test::expect![[r#""OK (\"one\" \"two\" \"three\")""#]],
    );

    // Splitting on newlines
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "line1\nline2\nline3" "\n")"#,
        expect_test::expect![[r#""OK (\"line1\" \"line2\" \"line3\")""#]],
    );

    // No matches for separator
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "hello" ",")"#,
        expect_test::expect![[r#""OK (\"hello\")""#]],
    );

    // Empty string
    crate::common::assert_oracle_parity_expect(
        r#"(split-string "" ",")"#,
        expect_test::expect![[r#""OK (\"\")""#]],
    );

    // Separator at edges
    crate::common::assert_oracle_parity_expect(
        r#"(split-string ",a,b,c," "," nil)"#,
        expect_test::expect![[r#""OK (\"\" \"a\" \"b\" \"c\" \"\")""#]],
    );
}

// ---------------------------------------------------------------------------
// string-join with separator
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_string_join_advanced() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Various separators
    crate::common::assert_oracle_parity_expect(
        r#"(string-join '("a" "b" "c") ", ")"#,
        expect_test::expect![[r#""OK \"a, b, c\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-join '("x" "y" "z") " -> ")"#,
        expect_test::expect![[r#""OK \"x -> y -> z\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-join '("1" "2" "3") "")"#,
        expect_test::expect![[r#""OK \"123\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-join '("one") "--")"#,
        expect_test::expect![[r#""OK \"one\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-join nil ",")"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );

    // Join with newline separator
    crate::common::assert_oracle_parity_expect(
        r#"(string-join '("line1" "line2" "line3") "\n")"#,
        expect_test::expect![[r#""OK \"line1\nline2\nline3\"""#]],
    );

    // Roundtrip: split then join
    let form = r####"(string-join (split-string "a-b-c" "-") "+")"####;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK \"a+b+c\"""#]]);

    // Join many elements
    let form2 = r#"(let ((items nil))
                     (dotimes (i 8)
                       (setq items (cons (format "item%d" i) items)))
                     (string-join (nreverse items) "|"))"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[r#""OK \"item0|item1|item2|item3|item4|item5|item6|item7\"""#]],
    );
}

// ---------------------------------------------------------------------------
// string-trim / string-trim-left / string-trim-right with custom chars
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_string_trim_custom_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Default trim (whitespace)
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim " \t\n hello \t\n ")"#,
        expect_test::expect![[r#""OK \"hello\"""#]],
    );

    // Custom trim chars
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "---hello---" "-")"#,
        expect_test::expect![[r#""OK \"--hello---\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "***hello***" "*")"#,
        expect_test::expect![[r#""OK \"**hello***\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "//path//" "/")"#,
        expect_test::expect![[r#""OK \"/path//\"""#]],
    );

    // Trim-left only
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim-left ">>>hello<<<" ">")"#,
        expect_test::expect![[r#""OK \">>hello<<<\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim-left "  hello  ")"#,
        expect_test::expect![[r#""OK \"hello  \"""#]],
    );

    // Trim-right only
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim-right ">>>hello<<<" "<")"#,
        expect_test::expect![[r#""OK \">>>hello<<\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim-right "hello..."  ".")"#,
        expect_test::expect![[r#""OK \"hello..\"""#]],
    );

    // Multiple custom chars in trim set
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "+-=hello=-+" "+-=")"#,
        expect_test::expect![[r#""OK \"hello=-+\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim-left "+-=hello=-+" "+-=")"#,
        expect_test::expect![[r#""OK \"hello=-+\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim-right "+-=hello=-+" "+-=")"#,
        expect_test::expect![[r#""OK \"+-=hello=-+\"""#]],
    );

    // Nothing to trim
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "hello" "x")"#,
        expect_test::expect![[r#""OK \"hello\"""#]],
    );

    // Entire string is trim chars
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "---" "-")"#,
        expect_test::expect![[r#""OK \"--\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-trim "" "-")"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
}

// ---------------------------------------------------------------------------
// string-prefix-p / string-suffix-p
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_string_prefix_suffix_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Prefix checks
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "hel" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "hello" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "world" "hello")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "hello!" "hello")"#,
        expect_test::expect![[r#""OK nil""#]],
    );

    // Suffix checks
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "llo" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "hello" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "world" "hello")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "hello!" "hello")"#,
        expect_test::expect![[r#""OK nil""#]],
    );

    // Case-insensitive prefix
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "HEL" "hello" t)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "HEL" "hello" nil)"#,
        expect_test::expect![[r#""OK nil""#]],
    );

    // Case-insensitive suffix
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "LLO" "hello" t)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "LLO" "hello" nil)"#,
        expect_test::expect![[r#""OK nil""#]],
    );

    // With empty string
    crate::common::assert_oracle_parity_expect(
        r#"(string-prefix-p "" "")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(string-suffix-p "" "")"#,
        expect_test::expect![[r#""OK t""#]],
    );
}

// ---------------------------------------------------------------------------
// upcase-initials for title case
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_upcase_initials() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "hello world")"#,
        expect_test::expect![[r#""OK \"Hello World\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "hello")"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "HELLO WORLD")"#,
        expect_test::expect![[r#""OK \"HELLO WORLD\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "hELLO wORLD")"#,
        expect_test::expect![[r#""OK \"HELLO WORLD\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "")"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "a")"#,
        expect_test::expect![[r#""OK \"A\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "hello-world")"#,
        expect_test::expect![[r#""OK \"Hello-World\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "one two three four")"#,
        expect_test::expect![[r#""OK \"One Two Three Four\"""#]],
    );

    // With non-alpha separators
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "foo_bar_baz")"#,
        expect_test::expect![[r#""OK \"Foo_Bar_Baz\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(upcase-initials "foo.bar.baz")"#,
        expect_test::expect![[r#""OK \"Foo.Bar.Baz\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Complex: string-based path manipulation (join, split, normalize)
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_string_path_manipulation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Build a path manipulation toolkit using string primitives
    let form = r####"(let ((path-split
                         (lambda (path)
                           (split-string path "/" t)))
                        (path-join
                         (lambda (parts)
                           (concat "/" (string-join parts "/"))))
                        (path-dirname
                         (lambda (path)
                           (let ((parts (split-string path "/" t)))
                             (if (> (length parts) 1)
                                 (concat "/"
                                         (string-join (butlast parts) "/"))
                               "/"))))
                        (path-basename
                         (lambda (path)
                           (let ((parts (split-string path "/" t)))
                             (if parts (car (last parts)) ""))))
                        (path-extension
                         (lambda (path)
                           (let ((base (car (last (split-string path "/" t)))))
                             (if (and base (string-match "\\." base))
                                 (let ((parts (split-string base "\\." t)))
                                   (car (last parts)))
                               nil))))
                        (path-normalize
                         (lambda (path)
                           (let ((parts (split-string path "/" t))
                                 (stack nil))
                             (dolist (part parts)
                               (cond
                                ((string= part "."))
                                ((string= part "..")
                                 (when stack (setq stack (cdr stack))))
                                (t (setq stack (cons part stack)))))
                             (concat "/" (string-join (nreverse stack) "/"))))))
                    (list
                     ;; Split
                     (funcall path-split "/usr/local/bin/emacs")
                     ;; Join
                     (funcall path-join '("usr" "local" "bin"))
                     ;; Dirname
                     (funcall path-dirname "/usr/local/bin/emacs")
                     (funcall path-dirname "/single")
                     ;; Basename
                     (funcall path-basename "/usr/local/bin/emacs")
                     (funcall path-basename "/")
                     ;; Extension
                     (funcall path-extension "/home/user/file.txt")
                     (funcall path-extension "/home/user/file")
                     ;; Normalize with . and ..
                     (funcall path-normalize "/usr/local/../share/./emacs")
                     (funcall path-normalize "/a/b/c/../../d")
                     (funcall path-normalize "/a/./b/./c")))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK ((\"usr\" \"local\" \"bin\" \"emacs\") \"/usr/local/bin\" \"/usr/local/bin\" \"/\" \"emacs\" \"\" \"txt\" nil \"/usr/share/emacs\" \"/a/d\" \"/a/b/c\")""#
        ]],
    );
}
