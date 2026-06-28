use super::*;

#[test]
fn test_simple_literal() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let result = search_pattern("hello", "say hello world", 0, false, &syn, 0);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.is_some());
    let (pos, regs) = r.unwrap();
    assert_eq!(pos, 4); // "hello" starts at position 4
    assert_eq!(regs.end[0], 9); // ends at 9
}

#[test]
fn test_dot_matches_any() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let result = search_pattern("h.llo", "say hello world", 0, false, &syn, 0);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.is_some());
}

#[test]
fn test_anchors() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // ^ at beginning
    let r = match_pattern("^hello", "hello world", 0, false, &syn, 0).unwrap();
    assert!(r.is_some());
    // ^ not at beginning
    let r = match_pattern("^hello", "say hello", 4, false, &syn, 0).unwrap();
    assert!(r.is_none());
}

#[test]
fn test_groups() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let result = search_pattern("\\(hel\\)lo", "hello", 0, false, &syn, 0);
    assert!(result.is_ok());
    let (pos, regs) = result.unwrap().unwrap();
    assert_eq!(pos, 0);
    assert_eq!(regs.start[1], 0); // group 1 start
    assert_eq!(regs.end[1], 3); // group 1 end ("hel")
}

#[test]
fn test_word_boundary() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("\\bhello\\b", "say hello world", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_some());
}

#[test]
fn test_star_repetition() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("hel*o", "heo", 0, false, &syn, 0);
    assert!(r.unwrap().is_some()); // zero l's
    let r = search_pattern("hel*o", "hello", 0, false, &syn, 0);
    assert!(r.unwrap().is_some()); // two l's
    let r = search_pattern("hel*o", "hellllo", 0, false, &syn, 0);
    assert!(r.unwrap().is_some()); // four l's
}

#[test]
fn test_charset() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("[abc]", "xbz", 0, false, &syn, 0);
    assert!(r.unwrap().is_some());
    let r = search_pattern("[abc]", "xyz", 0, false, &syn, 0);
    assert!(r.unwrap().is_none());
}

#[test]
fn test_syntax_word() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // \sw matches word characters
    let r = search_pattern("\\sw+", "hello world", 0, false, &syn, 0);
    assert!(r.unwrap().is_some());
}

#[test]
fn default_syntax_lookup_uses_gnu_standard_classes() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    assert_eq!(
        syn.char_syntax('a'),
        crate::emacs_core::syntax::SyntaxClass::Word
    );
    assert_eq!(
        syn.char_syntax('$'),
        crate::emacs_core::syntax::SyntaxClass::Word
    );
    assert_eq!(
        syn.char_syntax('_'),
        crate::emacs_core::syntax::SyntaxClass::Symbol
    );
    assert_eq!(
        syn.char_syntax('-'),
        crate::emacs_core::syntax::SyntaxClass::Symbol
    );
    assert_eq!(
        syn.char_syntax(' '),
        crate::emacs_core::syntax::SyntaxClass::Whitespace
    );
    assert_eq!(
        syn.char_syntax('\u{4e2d}'),
        crate::emacs_core::syntax::SyntaxClass::Word
    );
}

#[test]
fn test_backreference() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("\\(a\\)\\1", "aa", 0, false, &syn, 0);
    assert!(r.unwrap().is_some());
    let r = search_pattern("\\(a\\)\\1", "ab", 0, false, &syn, 0);
    assert!(r.unwrap().is_none());
}

#[test]
fn backreference_to_open_group_is_invalid_like_gnu() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let err = search_pattern("\\([^ \t\n]+ \\1\\)", "hello hello", 0, false, &syn, 0)
        .expect_err("GNU signals invalid-regexp for a backreference before group end");
    assert_eq!(err.message, "Invalid back reference");
}

#[test]
fn test_alternation() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("\\(foo\\|bar\\)", "test bar baz", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    assert!(r.as_ref().unwrap().is_some(), "match failed");
    let (pos, regs) = r.unwrap().unwrap();
    assert_eq!(pos, 5, "match position");
    assert_eq!(regs.start[0], 5);
    assert_eq!(regs.end[0], 8);
}

#[test]
fn test_char_range() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("[0-9]+", "foo 123 bar", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    assert!(r.as_ref().unwrap().is_some(), "match failed");
    let (pos, _regs) = r.unwrap().unwrap();
    assert_eq!(pos, 4, "match position");
}

#[test]
fn test_fastmap_skips_positions() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // Pattern starts with 'z' — should skip to position where 'z' appears
    let r = search_pattern("zing", "aaaaaaaaaazing", 0, false, &syn, 0);
    assert!(r.unwrap().is_some());
    let r = search_pattern("zing", "aaaaaaaaaazing", 0, false, &syn, 0);
    let (pos, _) = r.unwrap().unwrap();
    assert_eq!(pos, 10);
}

#[test]
fn test_fastmap_literal_accurate() {
    crate::test_utils::init_test_tracing();
    // Verify fastmap is populated and accurate for a simple literal
    let compiled = regex_compile("hello", false, false).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(compiled.fastmap[b'h' as usize]);
    assert!(!compiled.fastmap[b'a' as usize]);
    assert!(!compiled.fastmap[b'z' as usize]);
}

#[test]
fn test_fastmap_charset() {
    crate::test_utils::init_test_tracing();
    // Verify fastmap for character class patterns
    let compiled = regex_compile("[abc]", false, false).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(compiled.fastmap[b'a' as usize]);
    assert!(compiled.fastmap[b'b' as usize]);
    assert!(compiled.fastmap[b'c' as usize]);
    assert!(!compiled.fastmap[b'd' as usize]);
}

#[test]
fn test_fastmap_case_fold() {
    crate::test_utils::init_test_tracing();
    // Case-folded pattern should match both cases
    let compiled = regex_compile("Hello", false, true).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(compiled.fastmap[b'h' as usize]);
    assert!(compiled.fastmap[b'H' as usize]);
}

#[test]
fn test_fastmap_alternation() {
    crate::test_utils::init_test_tracing();
    // Alternation: both branches should appear in fastmap
    let compiled = regex_compile("\\(foo\\|bar\\)", false, false).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(compiled.fastmap[b'f' as usize]);
    assert!(compiled.fastmap[b'b' as usize]);
    assert!(!compiled.fastmap[b'z' as usize]);
}

#[test]
fn test_fastmap_dot() {
    crate::test_utils::init_test_tracing();
    // AnyChar: everything except newline
    let compiled = regex_compile(".", false, false).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(compiled.fastmap[b'a' as usize]);
    assert!(compiled.fastmap[b'Z' as usize]);
    assert!(!compiled.fastmap[b'\n' as usize]);
}

#[test]
fn test_fastmap_anchor_then_literal() {
    crate::test_utils::init_test_tracing();
    // ^hello — anchor is zero-width, fastmap should see 'h'
    let compiled = regex_compile("^hello", false, false).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(compiled.fastmap[b'h' as usize]);
    assert!(!compiled.fastmap[b'x' as usize]);
}

#[test]
fn test_fastmap_charset_not() {
    crate::test_utils::init_test_tracing();
    // [^abc] should allow everything except a, b, c
    let compiled = regex_compile("[^abc]", false, false).unwrap();
    assert!(compiled.fastmap_accurate);
    assert!(!compiled.fastmap[b'a' as usize]);
    assert!(!compiled.fastmap[b'b' as usize]);
    assert!(!compiled.fastmap[b'c' as usize]);
    assert!(compiled.fastmap[b'd' as usize]);
    assert!(compiled.fastmap[b'z' as usize]);
}

#[test]
fn test_unterminated_charset_reports_gnu_ebrack() {
    crate::test_utils::init_test_tracing();
    match regex_compile("[invalid", false, false) {
        Ok(_) => panic!("unterminated charset should fail"),
        Err(err) => assert_eq!(err.message, "Unmatched [ or [^"),
    }
}

#[test]
fn test_trailing_backslash_reports_gnu_eescape() {
    crate::test_utils::init_test_tracing();
    match regex_compile("a\\", false, false) {
        Ok(_) => panic!("trailing backslash should fail"),
        Err(err) => assert_eq!(err.message, "Trailing backslash"),
    }
}

#[test]
fn test_unmatched_interval_reports_gnu_ebrace() {
    crate::test_utils::init_test_tracing();
    match regex_compile("a\\{2", false, false) {
        Ok(_) => panic!("unmatched interval should fail"),
        Err(err) => assert_eq!(err.message, "Unmatched \\{"),
    }
}

#[test]
fn test_multibyte_charset() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("[àáâ]", "hello à world", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    assert!(r.unwrap().is_some(), "should match à in text");
}

#[test]
fn test_multibyte_charset_no_match() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("[àáâ]", "hello world", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(
        r.unwrap().is_none(),
        "should not match when no accented chars"
    );
}

#[test]
fn test_multibyte_charset_range() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // Range of accented Latin characters: é (U+00E9) through ü (U+00FC)
    let r = search_pattern("[é-ü]", "hello ö world", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    assert!(r.unwrap().is_some(), "ö should be in range é-ü");
}

#[test]
fn test_multibyte_charset_range_no_match() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // 'a' (U+0061) is outside the range é (U+00E9) through ü (U+00FC)
    let r = search_pattern("[é-ü]", "hello a world", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_none(), "ASCII 'a' should not be in range é-ü");
}

#[test]
fn test_multibyte_charset_not() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // [^à] should match any character that is not à
    let r = search_pattern("[^à]", "à", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_none(), "[^à] should not match 'à'");

    let r = search_pattern("[^à]", "b", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_some(), "[^à] should match 'b'");
}

#[test]
fn test_multibyte_charset_mixed() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // Mix of ASCII and non-ASCII in one charset
    let r = search_pattern("[aéz]", "hello é world", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_some(), "should match é");

    let r = search_pattern("[aéz]", "hello z world", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_some(), "should also match z");
}

#[test]
fn test_multibyte_charset_cjk() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // CJK characters
    let r = search_pattern("[你好世]", "say 好 to the world", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_some(), "should match 好");
}

#[test]
fn test_multibyte_charset_match_position() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    let r = search_pattern("[àáâ]", "hello á world", 0, false, &syn, 0);
    let (pos, regs) = r.unwrap().unwrap();
    assert_eq!(pos, 6, "á starts at byte 6");
    assert_eq!(regs.end[0], 8, "á is 2 bytes in UTF-8, ends at byte 8");
}

// Regression tests for the byte-shift bug: when an alternation/quantifier
// splices an `on_failure_jump` (or similar) AHEAD of an already-emitted
// `Charset`/`CharsetNot` opcode, the opcode's byte position shifts.  The
// multibyte range table is kept in a side map keyed by that byte position;
// before the fix the keys were never updated, so the range table was orphaned
// and non-ASCII chars silently failed to match.  GNU returns 0 for all of the
// patterns below; neomacs returned `None` (matching char position 0 here).

#[test]
fn test_charset_before_alternation_shift() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // (string-match "[é]\\|x" (string ?é)) => 0 in GNU.
    // The `\\|x` second alternative splices an OnFailureJump before `[é]`.
    let r = search_pattern("[é]\\|x", "é", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    let (pos, _) = r
        .unwrap()
        .expect("[é]\\|x should match the lone é, not be orphaned by the splice");
    assert_eq!(pos, 0);
}

#[test]
fn test_charset_range_before_alternation_shift() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // (string-match "[ç-ï]\\|x" (string ?é)) => 0 in GNU (é is in ç..ï).
    let r = search_pattern("[ç-ï]\\|x", "é", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    let (pos, _) = r.unwrap().expect("[ç-ï]\\|x should match é (in range ç-ï)");
    assert_eq!(pos, 0);
}

#[test]
fn test_charset_range_before_quantifier_shift() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // (string-match "[ç-ï]*x" (string ?é ?é ?x)) => 0 in GNU.
    // The `*` splices OnFailureJumpLoop before `[ç-ï]`.
    let r = search_pattern("[ç-ï]*x", "ééx", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    let (pos, _) = r
        .unwrap()
        .expect("[ç-ï]*x should match \"ééx\" from the start");
    assert_eq!(pos, 0);
}

#[test]
fn test_charset_in_group_before_alternation_shift() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // (string-match "\\([ç-ï]\\)\\|z" (string ?é)) => 0 in GNU.
    // Group + alternation both splice bytes ahead of `[ç-ï]`.
    let r = search_pattern("\\([ç-ï]\\)\\|z", "é", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    let (pos, regs) = r
        .unwrap()
        .expect("\\([ç-ï]\\)\\|z should match é and capture it");
    assert_eq!(pos, 0);
    assert_eq!(regs.start[1], 0, "group 1 should capture the é");
    assert_eq!(regs.end[1], 2, "é is 2 bytes in UTF-8");
}

#[test]
fn test_charset_optional_before_quantifier_shift() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // Greedy `?` also splices an OnFailureJump before the charset.
    // (string-match "[ç-ï]?x" (string ?é ?x)) => 0 in GNU.
    let r = search_pattern("[ç-ï]?x", "éx", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    let (pos, _) = r.unwrap().expect("[ç-ï]?x should match \"éx\"");
    assert_eq!(pos, 0);

    // Non-greedy `*?` truncates+re-extends the charset body to a new offset.
    // (string-match "[ç-ï]*?x" (string ?é ?é ?x)) => 0 in GNU.
    let r = search_pattern("[ç-ï]*?x", "ééx", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    let (pos, _) = r.unwrap().expect("[ç-ï]*?x should match \"ééx\"");
    assert_eq!(pos, 0);
}

#[test]
fn test_ascii_class_before_alternation_no_regression() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // ASCII-only classes use the bitmap (no side-map entry), so the splice
    // never affected them; assert they still behave correctly.
    let r = search_pattern("[a-c]\\|x", "b", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert_eq!(r.unwrap().expect("[a-c]\\|x should match b").0, 0);

    let r = search_pattern("[a-c]\\|x", "x", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert_eq!(
        r.unwrap().expect("[a-c]\\|x should match x via 2nd alt").0,
        0
    );

    let r = search_pattern("[a-c]\\|x", "z", 0, false, &syn, 0);
    assert!(r.is_ok());
    assert!(r.unwrap().is_none(), "[a-c]\\|x should not match z");

    // Quantifier over an ASCII class (`*`) plus a multibyte class to confirm
    // the two co-exist after a shift.
    let r = search_pattern("[a-c]*[ç-ï]", "abcé", 0, false, &syn, 0);
    assert!(r.is_ok(), "compile failed: {:?}", r.err());
    assert_eq!(r.unwrap().expect("[a-c]*[ç-ï] should match \"abcé\"").0, 0);
}

// ---------------------------------------------------------------------------
// GNU parity: descending intervals \{n,m\} with n>m must be rejected.
// (string-match "a\\{2,1\\}" "aa") -> (invalid-regexp "Invalid content of \\{\\}")
// ---------------------------------------------------------------------------

#[test]
fn test_descending_interval_reports_gnu_badbr() {
    crate::test_utils::init_test_tracing();
    // \{2,1\}, \{5,2\}, \{3,0\}: lower > upper must signal "Invalid content of \{\}".
    for pat in ["a\\{2,1\\}", "a\\{5,2\\}", "a\\{3,0\\}"] {
        match regex_compile(pat, false, false) {
            Ok(_) => panic!("descending interval {pat:?} should fail to compile"),
            Err(err) => assert_eq!(
                err.message, "Invalid content of \\{\\}",
                "wrong error for {pat:?}"
            ),
        }
    }
}

#[test]
fn test_ascending_and_unbounded_intervals_still_compile() {
    crate::test_utils::init_test_tracing();
    // Equal bounds, ascending bounds, and an unbounded upper must remain valid.
    for pat in ["a\\{2,3\\}", "a\\{2,2\\}", "a\\{2,\\}", "a\\{0,2\\}"] {
        assert!(
            regex_compile(pat, false, false).is_ok(),
            "valid interval {pat:?} should compile"
        );
    }
}

// ---------------------------------------------------------------------------
// GNU parity: a redundant trailing quantifier folds onto the preceding one.
// (string-match "a**" "aaa") -> 0  (GNU; neo previously returned nil)
// Also a*?*, a*+, a++, a???.
// ---------------------------------------------------------------------------

#[test]
fn test_stacked_quantifiers_fold_like_gnu() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // Each of these must compile and match at position 0 of "aaa",
    // exactly like GNU's quantifier folding.
    for pat in ["a**", "a*?*", "a*+", "a++", "a???", "a+*", "a?*", "a?+"] {
        let r = search_pattern(pat, "aaa", 0, false, &syn, 0);
        let r = r.unwrap_or_else(|e| panic!("{pat:?} failed to compile: {e:?}"));
        let (pos, _regs) = r.unwrap_or_else(|| panic!("{pat:?} should match \"aaa\""));
        assert_eq!(pos, 0, "{pat:?} should match at position 0");
    }
}

#[test]
fn test_stacked_greedy_star_consumes_all() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // `a**` folds to a greedy `a*`, so on "aaa" it consumes all three a's.
    let (pos, regs) = search_pattern("a**", "aaa", 0, false, &syn, 0)
        .unwrap()
        .expect("a** should match \"aaa\"");
    assert_eq!(pos, 0);
    assert_eq!(regs.end[0], 3, "greedy a** should consume all three a's");
}

#[test]
fn test_stacked_plus_requires_one() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // `a++` folds to a greedy `a+`: must match at least one `a`.
    let r = search_pattern("a++", "aaa", 0, false, &syn, 0).unwrap();
    let (pos, regs) = r.expect("a++ should match \"aaa\"");
    assert_eq!(pos, 0);
    assert_eq!(regs.end[0], 3);
    // `a+` (folded from a++) must NOT match a string with no `a`.
    let r = search_pattern("a++", "bbb", 0, false, &syn, 0).unwrap();
    assert!(r.is_none(), "a++ must not match a string with no a's");
}

#[test]
fn cntrl_class_excludes_del_like_gnu() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    // GNU `ISCNTRL(c)` is `((c) < ' ')` (regex-emacs.c:108), so `[[:cntrl:]]`
    // matches only 0x00..=0x1F.  In particular it must NOT match DEL (0x7F),
    // unlike the C-locale `iscntrl`.  This is the primitive behind json.el's
    // `(rx (in cntrl))`, which controls whether `json-encode-string` escapes a
    // character: DEL must pass through literally, only chars < 0x20 are escaped.

    // 0x1F (unit separator) is a control char and must match.
    assert!(
        search_pattern("[[:cntrl:]]", "\u{1f}", 0, false, &syn, 0)
            .unwrap()
            .is_some(),
        "[[:cntrl:]] must match 0x1F"
    );
    // 0x7F (DEL) is NOT a control char for Emacs regexp.
    assert!(
        search_pattern("[[:cntrl:]]", "\u{7f}", 0, false, &syn, 0)
            .unwrap()
            .is_none(),
        "[[:cntrl:]] must NOT match DEL (0x7F)"
    );
    // The same holds when combined with other class members, mirroring the
    // exact charset json.el compiles: `(rx (in ?\" ?\\ cntrl))`.
    assert!(
        search_pattern("[\"\\\\[:cntrl:]]", "\u{7f}", 0, false, &syn, 0)
            .unwrap()
            .is_none(),
        "[\"\\\\[:cntrl:]] must NOT match DEL (0x7F)"
    );
    // A boundary check: 0x20 (space) is not a control char.
    assert!(
        search_pattern("[[:cntrl:]]", " ", 0, false, &syn, 0)
            .unwrap()
            .is_none(),
        "[[:cntrl:]] must NOT match space"
    );
}

/// A `SyntaxLookup` mirroring the buffer-local syntax table used by the default
/// `*scratch*`/batch buffer (`lisp-interaction-mode` / `emacs-lisp-mode`): the
/// newline is comment-end (`Sendcomment`, syntax `>`) and the carriage return
/// is a symbol constituent (`Ssymbol`, syntax `_`) — neither is whitespace.
/// Space, tab and formfeed remain whitespace.  Everything else falls back to
/// the GNU standard-table classification.
struct LispModeSyntaxLookup;

impl SyntaxLookup for LispModeSyntaxLookup {
    fn char_syntax(&self, c: char) -> SyntaxClass {
        match c {
            '\n' => SyntaxClass::EndComment,
            '\r' => SyntaxClass::Symbol,
            _ => crate::emacs_core::syntax::standard_syntax_class_for_char(c),
        }
    }

    fn char_has_category(&self, c: char, cat: u8) -> bool {
        DefaultSyntaxLookup.char_has_category(c, cat)
    }
}

/// GNU `[[:space:]]` is `ISSPACE(c) == (BUFFER_SYNTAX(c) == Swhitespace)`
/// (regex-emacs.c:151,1618): it consults the ACTIVE syntax table's whitespace
/// class, NOT a fixed isspace/Unicode-whitespace set.  neomacs previously baked
/// space/tab/LF/CR/FF into the compile-time bitmap, so `[[:space:]]` matched LF
/// and CR even when the buffer's syntax table classified them otherwise.
///
/// Under a `lisp-interaction-mode`-style table (the default batch buffer) `\n`
/// is comment-end and `\r` is a symbol, so neither is whitespace and
/// `[[:space:]]` must NOT match them — matching GNU's
/// `(string-match "[[:space:]]" "\n")` => nil and `"\r"` => nil.  Space, tab and
/// formfeed are still whitespace and must match.
#[test]
fn posix_space_class_consults_syntax_table_excludes_newline_cr() {
    crate::test_utils::init_test_tracing();
    let syn = LispModeSyntaxLookup;

    // GNU: nil for LF and CR under emacs-lisp syntax.
    assert!(
        search_pattern("[[:space:]]", "\n", 0, false, &syn, 0)
            .unwrap()
            .is_none(),
        "[[:space:]] must NOT match LF when newline is comment-end (GNU nil)"
    );
    assert!(
        search_pattern("[[:space:]]", "\r", 0, false, &syn, 0)
            .unwrap()
            .is_none(),
        "[[:space:]] must NOT match CR when it is a symbol constituent (GNU nil)"
    );

    // GNU: 0 for space, tab and formfeed (still whitespace syntax).
    for (text, label) in [(" ", "space"), ("\t", "tab"), ("\u{0c}", "formfeed")] {
        assert!(
            search_pattern("[[:space:]]", text, 0, false, &syn, 0)
                .unwrap()
                .is_some(),
            "[[:space:]] must match {label} (whitespace syntax)"
        );
    }
}

/// Under the GNU *standard* syntax table (`init_syntax_once`, syntax.c:3686-3691,
/// the table used by `fundamental-mode` / `(standard-syntax-table)`), LF and CR
/// ARE whitespace, so `[[:space:]]` DOES match them.  This is the other half of
/// the syntax-table dependency and guards against over-correcting the fix.
#[test]
fn posix_space_class_matches_newline_cr_under_standard_syntax() {
    crate::test_utils::init_test_tracing();
    let syn = DefaultSyntaxLookup;
    for (text, label) in [
        ("\n", "LF"),
        ("\r", "CR"),
        (" ", "space"),
        ("\t", "tab"),
        ("\u{0c}", "formfeed"),
    ] {
        assert!(
            search_pattern("[[:space:]]", text, 0, false, &syn, 0)
                .unwrap()
                .is_some(),
            "[[:space:]] must match {label} under the standard syntax table"
        );
    }
}

/// `[[:blank:]]` is strictly ASCII space and tab (`ISBLANK`, regex-emacs.c:113):
/// it is NOT syntax-table-driven and must never match LF or CR regardless of the
/// syntax table.  Guards against the space fix accidentally touching blank.
#[test]
fn posix_blank_class_is_space_tab_only_independent_of_syntax() {
    crate::test_utils::init_test_tracing();
    for syn in [
        &DefaultSyntaxLookup as &dyn SyntaxLookup,
        &LispModeSyntaxLookup,
    ] {
        assert!(
            search_pattern("[[:blank:]]", " ", 0, false, syn, 0)
                .unwrap()
                .is_some(),
            "[[:blank:]] must match space"
        );
        assert!(
            search_pattern("[[:blank:]]", "\t", 0, false, syn, 0)
                .unwrap()
                .is_some(),
            "[[:blank:]] must match tab"
        );
        assert!(
            search_pattern("[[:blank:]]", "\n", 0, false, syn, 0)
                .unwrap()
                .is_none(),
            "[[:blank:]] must NOT match LF"
        );
        assert!(
            search_pattern("[[:blank:]]", "\r", 0, false, syn, 0)
                .unwrap()
                .is_none(),
            "[[:blank:]] must NOT match CR"
        );
    }
}
