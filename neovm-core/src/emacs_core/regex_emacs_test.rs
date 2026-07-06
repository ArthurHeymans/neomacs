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

    fn cache_key(&self) -> super::SyntaxCacheKey {
        // Test-only lookup, never routed through the pattern caches;
        // use a sentinel identity distinct from `Standard` so a cache
        // ever probed with it cannot hit standard-baked entries.
        super::SyntaxCacheKey::Table {
            id: usize::MAX,
            epoch: 0,
        }
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

// ===========================================================================
// Pike VM (non-backtracking fast path) — differential fuzzer + unit tests.
//
// The Pike VM MUST be byte-exact with the backtracker for every eligible
// pattern.  These tests drive that: the differential fuzzer generates random
// elisp-subset patterns + random buffers and asserts the two engines return
// identical match-data (match/no-match AND every capture-group start/end)
// for anchored `re_match` and forward/backward `re_search`.
// ===========================================================================

/// Tiny deterministic xorshift RNG so fuzz failures reproduce from a seed.
struct FuzzRng(u64);

impl FuzzRng {
    fn new(seed: u64) -> Self {
        FuzzRng(seed ^ 0x9E3779B97F4A7C15)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// True with probability `num/den`.
    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next_u64() % den < num
    }
    fn pick_char(&mut self, xs: &[char]) -> char {
        xs[self.below(xs.len())]
    }
    fn pick_str<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
}

/// Normalised match result: `(match_start, group_starts, group_ends)` or
/// `None`.  Comparing these across engines is the byte-exactness assertion.
type FuzzResult = Option<(usize, Vec<i64>, Vec<i64>)>;

fn norm(r: Option<(usize, MatchRegisters)>) -> FuzzResult {
    r.map(|(pos, regs)| (pos, regs.start.clone(), regs.end.clone()))
}

/// Run the two engines for an anchored match and a forward/backward search;
/// return `(label, backtracker_result, pike_result)` for the first that
/// diverges, or `None` if all agree.
fn diff_engines(
    cp: &CompiledPattern,
    text: &[u8],
    start: usize,
    point: usize,
) -> Option<(&'static str, FuzzResult, FuzzResult)> {
    let syn = DefaultSyntaxLookup;
    let len = text.len();

    // Compare the two engines directly by pinning each one (bypassing the
    // production step-budget heuristic): the pure backtracker (oracle) vs the
    // pure Pike VM.  If the backtracker aborts on the fail-stack limit
    // (catastrophic backtracking) it returns a spurious `None`, so skip.
    macro_rules! compare {
        ($label:expr, $call:expr) => {{
            let _ = take_matcher_overflow();
            let bt = norm(with_backtracker_forced(|| $call));
            if take_matcher_overflow() {
                return None; // backtracker overflowed — skip this comparison
            }
            let pk = norm(with_pike_forced(|| $call));
            if bt != pk {
                return Some(($label, bt, pk));
            }
        }};
    }

    // Anchored (`re_match` / looking-at) at `start`.
    compare!("re_match", re_match(cp, text, start, len, &syn, point));
    // Forward search from `start`.
    let range = (len - start) as isize;
    compare!("re_search_fwd", re_search(cp, text, start, range, &syn, point));
    // Backward search from `start` back to 0.
    let brange = -(start as isize);
    compare!("re_search_bwd", re_search(cp, text, start, brange, &syn, point));

    None
}

/// True if this (pattern, case_fold, text, start, point) diverges — used by
/// the shrinker.
fn fuzz_diverges(pat: &str, case_fold: bool, text: &[u8], start: usize, point: usize) -> bool {
    match regex_compile(pat, false, case_fold) {
        Ok(cp) => {
            if !cp.pike_eligible {
                return false;
            }
            let start = start.min(text.len());
            let point = point.min(text.len());
            diff_engines(&cp, text, start, point).is_some()
        }
        Err(_) => false,
    }
}

// ---- random pattern generation (eligible subset only) --------------------

const FUZZ_LITERALS: &[char] = &[
    'a', 'b', 'c', 'A', 'Z', '0', '1', ' ', '_', '-', '!', ':', '\n', 'é', '中', '\u{2018}',
    '\u{2019}',
];
const FUZZ_QUANTIFIERS: &[&str] = &["*", "+", "?", "*?", "+?", "??"];
const FUZZ_SYNTAX: &[&str] = &["\\w", "\\W", "\\sw", "\\s_", "\\s-", "\\s.", "\\Sw", "\\S_"];
const FUZZ_ANCHORS: &[&str] = &["^", "$", "\\`", "\\'"];
const FUZZ_BOUNDARIES: &[&str] = &["\\b", "\\B", "\\<", "\\>", "\\_<", "\\_>"];
const FUZZ_POSIX: &[&str] = &[
    "[:alpha:]", "[:digit:]", "[:alnum:]", "[:space:]", "[:upper:]", "[:punct:]", "[:word:]",
];

fn gen_literal(rng: &mut FuzzRng, out: &mut String) {
    let c = rng.pick_char(FUZZ_LITERALS);
    // Escape regex metacharacters so they stay literal.
    if "\\.*+?[]^$".contains(c) {
        out.push('\\');
    }
    out.push(c);
}

fn gen_charclass(rng: &mut FuzzRng, out: &mut String) {
    out.push('[');
    if rng.chance(1, 3) {
        out.push('^');
    }
    let items = 1 + rng.below(3);
    for _ in 0..items {
        match rng.below(4) {
            0 => {
                // range x-y (keep ordered, ASCII letters/digits)
                let bases = [('a', 'z'), ('A', 'Z'), ('0', '9')];
                let (lo, hi) = bases[rng.below(bases.len())];
                let a = (lo as u8) + rng.below((hi as u8 - lo as u8) as usize + 1) as u8;
                let b = a + rng.below((hi as u8 - a) as usize + 1) as u8;
                out.push(a as char);
                out.push('-');
                out.push(b as char);
            }
            1 => out.push_str(rng.pick_str(FUZZ_POSIX)),
            _ => {
                // plain class char (avoid ] \ [ - ^ to keep the class valid)
                let safe = ['a', 'q', 'Z', '3', '_', '中', 'é'];
                out.push(rng.pick_char(&safe));
            }
        }
    }
    out.push(']');
}

/// Generate a quantifiable atom.  Returns true if the atom may take a
/// postfix quantifier.
///
/// Two flags keep the forced-backtracker ORACLE tractable (the Pike VM is
/// linear regardless; the constraint is only about the oracle used to check
/// it), by never generating a quantifier over a body that can iterate with
/// zero progress — the sole source of *exponential* backtracking:
///   * `allow_quant` — FALSE inside a quantified atom, so quantifiers never
///     nest (`\(a*\)*`).
///   * `must_consume` — TRUE inside a quantified atom, so its body cannot be
///     nullable: zero-width atoms (anchors / boundaries) are excluded, so
///     every arm consumes ≥1 char (`\(?:A[中]\|\<\)*` is never generated).
///
/// Everything else is still covered: single quantifiers, non-nullable
/// `\(?:\w\|\s_\)+`, the O(n^2) `a*a*b`, alternation, captures, anchors and
/// boundaries at non-quantified positions, case-fold and multibyte.
/// (Nullable non-capturing loops are covered separately by targeted unit
/// tests on short input where the backtracker stays fast.)
fn gen_atom(
    rng: &mut FuzzRng,
    out: &mut String,
    depth: u32,
    allow_quant: bool,
    must_consume: bool,
) -> bool {
    let choices = if depth == 0 { 5 } else { 8 };
    // When the atom must consume, remap the two zero-width choices (4, 5)
    // onto consuming atoms.
    let pick = match rng.below(choices) {
        4 | 5 if must_consume => rng.below(4),
        n => n,
    };
    match pick {
        0 => {
            gen_literal(rng, out);
            true
        }
        1 => {
            out.push('.');
            true
        }
        2 => {
            gen_charclass(rng, out);
            true
        }
        3 => {
            out.push_str(rng.pick_str(FUZZ_SYNTAX));
            true
        }
        4 => {
            out.push_str(rng.pick_str(FUZZ_BOUNDARIES));
            false
        }
        5 => {
            out.push_str(rng.pick_str(FUZZ_ANCHORS));
            false
        }
        6 => {
            // shy group
            out.push_str("\\(?:");
            gen_regex(rng, out, depth - 1, allow_quant, must_consume);
            out.push_str("\\)");
            true
        }
        _ => {
            // capturing group
            out.push_str("\\(");
            gen_regex(rng, out, depth - 1, allow_quant, must_consume);
            out.push_str("\\)");
            true
        }
    }
}

fn gen_term(rng: &mut FuzzRng, out: &mut String, depth: u32, allow_quant: bool, must_consume: bool) {
    let will_quant = allow_quant && rng.chance(1, 2);
    // A quantified atom's body must not quantify (no nesting) and must
    // consume (no nullable loop body).
    let quantifiable = gen_atom(
        rng,
        out,
        depth,
        allow_quant && !will_quant,
        must_consume || will_quant,
    );
    if will_quant && quantifiable {
        out.push_str(rng.pick_str(FUZZ_QUANTIFIERS));
    }
}

fn gen_concat(
    rng: &mut FuzzRng,
    out: &mut String,
    depth: u32,
    allow_quant: bool,
    must_consume: bool,
) {
    let terms = 1 + rng.below(4);
    for _ in 0..terms {
        gen_term(rng, out, depth, allow_quant, must_consume);
    }
}

fn gen_regex(
    rng: &mut FuzzRng,
    out: &mut String,
    depth: u32,
    allow_quant: bool,
    must_consume: bool,
) {
    let arms = 1 + rng.below(3);
    for i in 0..arms {
        if i > 0 {
            out.push_str("\\|");
        }
        gen_concat(rng, out, depth, allow_quant, must_consume);
    }
}

// ---- random buffer generation --------------------------------------------

fn gen_text(rng: &mut FuzzRng, max_len: usize) -> Vec<u8> {
    let len = rng.below(max_len);
    let mut v = Vec::with_capacity(len);
    for _ in 0..len {
        match rng.below(10) {
            0 => v.push(b'\n'),
            1 => v.push(b' '),
            2 => v.push(b'_'),
            3 => v.push(b'-'),
            4 => v.push(b"abcABZ019:!"[rng.below(11)]),
            5 => v.extend_from_slice("é".as_bytes()),
            6 => v.extend_from_slice("中".as_bytes()),
            7 => v.extend_from_slice("\u{2018}".as_bytes()),
            8 => v.push(0x80 + rng.below(0x80) as u8), // raw high byte
            _ => v.push(b'a' + rng.below(26) as u8),
        }
    }
    v
}

/// Greedily shrink a failing case for a readable report.
fn shrink(
    pat: &str,
    case_fold: bool,
    text: &[u8],
    start: usize,
    point: usize,
) -> (String, Vec<u8>, usize, usize) {
    let mut pat = pat.to_string();
    let mut text = text.to_vec();
    let mut start = start.min(text.len());
    let mut point = point.min(text.len());

    // Bound the total number of `fuzz_diverges` probes (each recompiles the
    // pattern) so a large case can't make shrinking run for minutes.
    let mut budget: u32 = 3000;
    macro_rules! probe {
        ($pat:expr, $text:expr, $s:expr, $p:expr) => {{
            if budget == 0 {
                false
            } else {
                budget -= 1;
                fuzz_diverges($pat, case_fold, $text, $s, $p)
            }
        }};
    }

    // Shrink text (drop bytes while still diverging, keeping valid indices).
    let mut changed = true;
    while changed && budget > 0 {
        changed = false;
        let mut i = 0;
        while i < text.len() {
            let mut cand = text.clone();
            cand.remove(i);
            let s = start.min(cand.len());
            let p = point.min(cand.len());
            if probe!(&pat, &cand, s, p) {
                text = cand;
                start = s;
                point = p;
                changed = true;
            } else {
                i += 1;
            }
        }
    }
    // Shrink start / point toward 0.
    while start > 0 && probe!(&pat, &text, start - 1, point) {
        start -= 1;
    }
    while point > 0 && probe!(&pat, &text, start, point - 1) {
        point -= 1;
    }
    // Shrink pattern by trying to drop single chars (best-effort; may break
    // group balance, in which case the divergence check fails and we skip).
    changed = true;
    while changed && budget > 0 {
        changed = false;
        let chars: Vec<char> = pat.chars().collect();
        for i in 0..chars.len() {
            let cand: String = chars
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, c)| *c)
                .collect();
            if probe!(&cand, &text, start, point) {
                pat = cand;
                changed = true;
                break;
            }
        }
    }
    (pat, text, start, point)
}

fn run_fuzz(cases: usize, base_seed: u64, text_max: usize) {
    let mut eligible = 0usize;
    let mut compiled = 0usize;
    for i in 0..cases {
        let seed = base_seed.wrapping_add(i as u64);
        let mut rng = FuzzRng::new(seed);
        let case_fold = rng.chance(1, 3);
        let mut pat = String::new();
        let depth = 2 + rng.below(2) as u32;
        gen_regex(&mut rng, &mut pat, depth, true, false);

        let cp = match regex_compile(&pat, false, case_fold) {
            Ok(cp) => cp,
            Err(_) => continue, // invalid random pattern; skip
        };
        compiled += 1;
        if !cp.pike_eligible {
            continue;
        }
        eligible += 1;

        let text = gen_text(&mut rng, text_max);
        let start = rng.below(text.len() + 1);
        let point = rng.below(text.len() + 1);

        if let Some((label, bt, pk)) = diff_engines(&cp, &text, start, point) {
            // Print the RAW case first so it survives even if shrinking is slow.
            eprintln!(
                "PIKE/BACKTRACKER DIVERGENCE ({label}) at seed {seed}: pattern={pat:?} \
                 case_fold={case_fold} text={text:?} start={start} point={point} bt={bt:?} pike={pk:?}"
            );
            let (spat, stext, sstart, spoint) = shrink(&pat, case_fold, &text, start, point);
            panic!(
                "PIKE/BACKTRACKER DIVERGENCE ({label}) at seed {seed}:\n\
                 pattern   = {pat:?}  (case_fold={case_fold})\n\
                 text      = {text:?}\n\
                 start={start} point={point}\n\
                 backtrack = {bt:?}\n\
                 pike      = {pk:?}\n\
                 --- shrunk ---\n\
                 pattern   = {spat:?}\n\
                 text      = {stext:?}\n\
                 start={sstart} point={spoint}"
            );
        }
    }
    // Sanity: the generator should still exercise the Pike path on a large
    // fraction of cases (many random patterns are legitimately ineligible —
    // `??`, `\{n,m\}`, nullable non-greedy loops, capture-in-nullable-loop).
    assert!(
        eligible * 4 > compiled,
        "fuzzer should exercise the Pike path on many cases (eligible={eligible} compiled={compiled})"
    );
    assert!(eligible > cases / 8, "too few eligible cases: {eligible}/{cases}");
}

/// Fast differential fuzz that runs on every `cargo test` — a few thousand
/// cases is enough to catch gross divergences quickly.
#[test]
fn pike_fuzz_smoke() {
    crate::test_utils::init_test_tracing();
    // Short texts keep the forced-backtracker oracle fast (catastrophic
    // patterns stay bounded), so the smoke run finishes in seconds.
    run_fuzz(4_000, 0x1234_5678, 16);
}

/// The full 100k+ differential fuzz — the primary correctness gate.  Ignored
/// by default (slow); run explicitly with `--ignored`.
#[test]
#[ignore = "long-running differential fuzz; run explicitly"]
fn pike_fuzz_differential() {
    crate::test_utils::init_test_tracing();
    // Two independent seed streams for broader coverage (>>100k cases).
    run_fuzz(200_000, 0xF00D_BEEF, 28);
    run_fuzz(120_000, 0x0BAD_C0DE, 20);
}

// ---- targeted unit tests: Pike vs backtracker byte-exactness -------------

/// Assert both engines agree on a concrete (pattern, text) case.
fn assert_engines_agree(pat: &str, case_fold: bool, text: &[u8]) {
    let cp = regex_compile(pat, false, case_fold).expect("compile");
    assert!(cp.pike_eligible, "pattern {pat:?} should be pike-eligible");
    for start in 0..=text.len() {
        if let Some((label, bt, pk)) = diff_engines(&cp, text, start, start) {
            panic!("{label} divergence for {pat:?} @ {start}: bt={bt:?} pike={pk:?}");
        }
    }
}

#[test]
fn pike_greedy_and_captures() {
    // NB: `a??` (non-greedy optional) is intentionally Pike-INELIGIBLE (its
    // keep-string jump can't be modelled), so it is not asserted here.
    for p in [
        "a*", "a+", "a?", "a*?", "a+?", "\\(a*\\)\\(a*\\)", "\\(a\\|ab\\)\\(c\\|bcd\\)",
        "\\(?:ab\\)+", "a.*b", "a.*?b", "\\(a+\\)+", "\\(.*\\)\\(.*\\)",
    ] {
        assert_engines_agree(p, false, b"aaabcaabcd");
        assert_engines_agree(p, false, b"");
        assert_engines_agree(p, false, b"abababab");
    }
}

#[test]
fn pike_alternation_priority() {
    // Leftmost-greedy: left arm wins when both can match.
    assert_engines_agree("\\(a\\|ab\\)", false, b"ab");
    assert_engines_agree("\\(ab\\|a\\)", false, b"ab");
    assert_engines_agree("a\\|ab\\|abc", false, b"abc");
}

#[test]
fn pike_anchors_boundaries() {
    for p in ["^ab", "ab$", "\\<ab\\>", "\\_<a_b\\_>", "\\bword\\b", "\\`start", "end\\'"] {
        assert_engines_agree(p, false, b"ab word a_b start end\nab");
        assert_engines_agree(p, false, b"xx ab\nword\n");
    }
}

#[test]
fn pike_charsets_syntax_multibyte_casefold() {
    assert_engines_agree("[a-z]+", false, b"Hello World");
    assert_engines_agree("[^a-z ]+", false, b"Hello World 123");
    assert_engines_agree("[[:alpha:]]+", false, b"abc123 def");
    assert_engines_agree("\\w+", false, b"foo_bar baz");
    assert_engines_agree("\\(?:\\w\\|\\s_\\)+", false, b"a-b_c d");
    assert_engines_agree("[A-Z]+", true, b"hello WORLD"); // case fold
    assert_engines_agree("\u{2018}\\(\\w+\\)\u{2019}", false, "\u{2018}sym\u{2019}".as_bytes());
    assert_engines_agree("é+", false, "café ééé".as_bytes());
}

/// `a*a*b` — the classic catastrophic-backtracking pattern — must be handled
/// by the Pike VM in LINEAR time (was cubic in the backtracker).  This test
/// would blow up / time out if it ever fell back to the backtracker.
#[test]
fn pike_no_catastrophic_backtracking() {
    let cp = regex_compile("a*a*b", false, false).expect("compile");
    assert!(cp.pike_eligible, "a*a*b must be pike-eligible");

    // Linear check: a growing run of 'a' with no trailing 'b' fails FAST.
    // The backtracker is O(n^3) here; the Pike VM is O(n*m).  A generous
    // wall-clock ceiling that only the linear engine can meet.
    let syn = DefaultSyntaxLookup;
    let n = 20_000usize;
    let mut text = vec![b'a'; n];
    text.push(b'!');
    let t0 = std::time::Instant::now();
    assert!(re_match(&cp, &text, 0, text.len(), &syn, 0).is_none());
    let elapsed = t0.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "a*a*b over {n} a's took {elapsed:?} — not linear (Pike path not taken?)"
    );

    // And it still finds the real match when a 'b' is present.
    let mut text2 = vec![b'a'; 500];
    text2.push(b'b');
    let r = re_match(&cp, &text2, 0, text2.len(), &syn, 0);
    assert_eq!(r.map(|(e, _)| e), Some(501));
}




/// Nullable NON-capturing loops (empty-matchable body, no capture in the
/// cycle) stay Pike-eligible — the `seen`-set handles termination and there
/// are no captures to lose.  These are excluded from the fuzzer (the
/// backtracker oracle is catastrophically slow on them) so pin them here on
/// short input where the backtracker stays fast.  Any capture INSIDE the
/// loop makes the pattern ineligible (see `has_capturing_epsilon_cycle`).
#[test]
fn pike_nullable_noncapturing_loops() {
    for p in [
        "\\(?:a\\|\\)*b",
        "\\(?:\\<\\)*a",
        "\\(?:x*\\)*y",   // inner star, outer star, shy — nullable, no capture
        "\\(?:\\b\\)*.",
        "\\(?:a\\|b\\|\\)+c",
    ] {
        let cp = regex_compile(p, false, false).expect("compile");
        // These are eligible ONLY because the epsilon cycle carries no
        // capture group; assert that so the test documents the boundary.
        assert!(cp.pike_eligible, "{p:?} should be pike-eligible (no capture in cycle)");
        // Keep texts SHORT (<=3 chars): these nullable-loop patterns blow up
        // exponentially in the forced-backtracker oracle on longer input.
        for text in [&b""[..], b"a", b"b", b"ab", b"ba", b"xy", b"c", b"axy"] {
            if let Some((label, bt, pk)) = diff_engines(&cp, text, 0, 0) {
                panic!("{label} divergence for {p:?} on {text:?}: bt={bt:?} pike={pk:?}");
            }
        }
    }
}

/// The dual: a capture INSIDE a nullable loop is Pike-INELIGIBLE (GNU's
/// empty-loop capture semantics), so it must fall back to the backtracker.
#[test]
fn pike_capture_in_nullable_loop_is_ineligible() {
    for p in ["\\(a*\\)*", "\\(a\\|\\)*", "\\(?:\\(x*\\)\\)*", "\\(.??\\)+"] {
        let cp = regex_compile(p, false, false).expect("compile");
        assert!(
            !cp.pike_eligible,
            "{p:?} has a capture in a nullable loop; must be Pike-ineligible"
        );
    }
}


