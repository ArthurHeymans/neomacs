use super::*;
use crate::emacs_core::error::Flow;
use crate::emacs_core::value::{Value, get_string_text_properties_for_value};

#[test]
fn ascii_width() {
    crate::test_utils::init_test_tracing();
    assert_eq!(char_width('a'), 1);
    assert_eq!(char_width(' '), 1);
    assert_eq!(char_width('Z'), 1);
}

#[test]
fn cjk_width() {
    crate::test_utils::init_test_tracing();
    assert_eq!(char_width('中'), 2);
    assert_eq!(char_width('日'), 2);
    assert_eq!(char_width('あ'), 2);
    assert_eq!(char_width('ア'), 2);
}

#[test]
fn gnu_default_emoji_symbol_widths() {
    crate::test_utils::init_test_tracing();
    assert_eq!(char_width('\u{2603}'), 1);
    assert_eq!(char_width('\u{2615}'), 2);
    assert_eq!(char_width('\u{263A}'), 1);
}

#[test]
fn control_char_width() {
    crate::test_utils::init_test_tracing();
    assert_eq!(char_width('\0'), 2);
    assert_eq!(char_width('\x01'), 2); // ^A
    assert_eq!(char_width('\n'), 0);
    assert_eq!(char_width('\x7f'), 2); // ^?
    assert_eq!(char_width('\u{0080}'), 4);
    assert_eq!(char_width('\u{009f}'), 4);
}

#[test]
fn string_width_mixed() {
    crate::test_utils::init_test_tracing();
    assert_eq!(string_width("hello"), 5);
    assert_eq!(string_width("中文"), 4);
    assert_eq!(string_width("hi中"), 4);
}

#[test]
fn builtin_string_bytes_counts_utf8_length() {
    crate::test_utils::init_test_tracing();
    let result = builtin_string_bytes(vec![Value::string("Aé中")]).unwrap();
    assert_eq!(result, Value::fixnum(6));
}

#[test]
fn builtin_char_displayable_p_matches_oracle_bounds_and_types() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_char_displayable_p(vec![Value::fixnum('a' as i64)]).unwrap(),
        Value::T
    );
    assert_eq!(
        builtin_char_displayable_p(vec![Value::fixnum(0x00E9)]).unwrap(),
        Value::symbol("unicode")
    );
    assert_eq!(
        builtin_char_displayable_p(vec![Value::fixnum(0x11_0000)]).unwrap(),
        Value::NIL
    );

    let overflow = builtin_char_displayable_p(vec![Value::fixnum(0x40_0000)])
        .expect_err("overflow char code should signal wrong-type-argument characterp");
    match overflow {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("characterp"), Value::fixnum(0x40_0000)]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }

    let non_number = builtin_char_displayable_p(vec![Value::symbol("x")])
        .expect_err("non-number should signal number-or-marker-p");
    match non_number {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("number-or-marker-p"), Value::symbol("x")]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }
}

#[test]
fn builtin_char_width_matches_oracle_control_and_bounds() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_char_width(vec![Value::fixnum(0)]).unwrap(),
        Value::fixnum(2)
    );
    assert_eq!(
        builtin_char_width(vec![Value::fixnum(9)]).unwrap(),
        Value::fixnum(8)
    );
    assert_eq!(
        builtin_char_width(vec![Value::fixnum(10)]).unwrap(),
        Value::fixnum(0)
    );
    assert_eq!(
        builtin_char_width(vec![Value::fixnum(0x80)]).unwrap(),
        Value::fixnum(4)
    );
    assert_eq!(
        builtin_char_width(vec![Value::fixnum(0x11_0000)]).unwrap(),
        Value::fixnum(1)
    );

    let negative = builtin_char_width(vec![Value::fixnum(-1)])
        .expect_err("negative character code should signal");
    match negative {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("characterp"), Value::fixnum(-1)]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }

    let overflow = builtin_char_width(vec![Value::fixnum(0x40_0000)])
        .expect_err("overflow character code should signal");
    match overflow {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("characterp"), Value::fixnum(0x40_0000)]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }
}

#[test]
fn builtin_char_or_string_p_respects_character_bounds() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_char_or_string_p(vec![Value::fixnum(0)]).unwrap(),
        Value::T
    );
    assert_eq!(
        builtin_char_or_string_p(vec![Value::fixnum(0x3F_FFFF)]).unwrap(),
        Value::T
    );
    assert_eq!(
        builtin_char_or_string_p(vec![Value::fixnum(-1)]).unwrap(),
        Value::NIL
    );
    assert_eq!(
        builtin_char_or_string_p(vec![Value::fixnum(0x40_0000)]).unwrap(),
        Value::NIL
    );
    assert_eq!(
        builtin_char_or_string_p(vec![Value::symbol("x")]).unwrap(),
        Value::NIL
    );
}

#[test]
fn builtin_max_char_optional_unicode_matches_oracle() {
    crate::test_utils::init_test_tracing();
    assert_eq!(builtin_max_char(vec![]).unwrap(), Value::fixnum(0x3F_FFFF));
    assert_eq!(
        builtin_max_char(vec![Value::NIL]).unwrap(),
        Value::fixnum(0x3F_FFFF)
    );
    assert_eq!(
        builtin_max_char(vec![Value::T]).unwrap(),
        Value::fixnum(0x10_FFFF)
    );
    assert_eq!(
        builtin_max_char(vec![Value::symbol("foo")]).unwrap(),
        Value::fixnum(0x10_FFFF)
    );

    let wrong_arity = builtin_max_char(vec![Value::fixnum(1), Value::fixnum(2)])
        .expect_err("max-char should reject more than one argument");
    match wrong_arity {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(sig.data, vec![Value::symbol("max-char"), Value::fixnum(2)]);
        }
        other => panic!("expected signal, got: {other:?}"),
    }
}

#[test]
fn builtin_coding_string_helpers_enforce_max_arity() {
    crate::test_utils::init_test_tracing();
    let encode_over = builtin_encode_coding_string(vec![
        Value::string("a"),
        Value::symbol("utf-8"),
        Value::NIL,
        Value::NIL,
        Value::NIL,
    ])
    .expect_err("encode-coding-string should reject more than four arguments");
    match encode_over {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(
                sig.data,
                vec![Value::symbol("encode-coding-string"), Value::fixnum(5)]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }

    let decode_over = builtin_decode_coding_string(vec![
        Value::string("a"),
        Value::symbol("utf-8"),
        Value::NIL,
        Value::NIL,
        Value::NIL,
    ])
    .expect_err("decode-coding-string should reject more than four arguments");
    match decode_over {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(
                sig.data,
                vec![Value::symbol("decode-coding-string"), Value::fixnum(5)]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }
}

#[test]
fn builtin_coding_string_helpers_runtime_match_oracle_core_cases() {
    crate::test_utils::init_test_tracing();
    let encoded = builtin_encode_coding_string(vec![Value::string("é"), Value::symbol("utf-8")])
        .expect("encode-coding-string should evaluate");
    let ls = encoded
        .as_lisp_string()
        .expect("encode-coding-string should return a string");
    assert_eq!(ls.as_bytes(), &[0xC3, 0xA9]);

    let decode_utf8 =
        builtin_decode_coding_string(vec![Value::string("é"), Value::symbol("utf-8")])
            .expect("decode-coding-string should evaluate");
    assert_eq!(decode_utf8, Value::string("é"));

    let nil_encode =
        builtin_encode_coding_string(vec![Value::string("é"), Value::NIL]).expect("nil coding");
    assert_eq!(nil_encode, Value::string("é"));

    let nil_decode =
        builtin_decode_coding_string(vec![Value::string("é"), Value::NIL]).expect("nil coding");
    assert_eq!(nil_decode, Value::string("é"));

    let coding_string =
        builtin_encode_coding_string(vec![Value::string("a"), Value::string("utf-8")])
            .expect_err("string coding-system should signal symbolp");
    match coding_string {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("symbolp"), Value::string("utf-8")]
            );
        }
        other => panic!("expected signal, got: {other:?}"),
    }

    let unknown_encode =
        builtin_encode_coding_string(vec![Value::string("a"), Value::symbol("vm-no-such-coding")])
            .expect_err("unknown coding-system should signal coding-system-error");
    match unknown_encode {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "coding-system-error");
            assert_eq!(sig.data, vec![Value::symbol("vm-no-such-coding")]);
        }
        other => panic!("expected signal, got: {other:?}"),
    }

    let unknown_decode =
        builtin_decode_coding_string(vec![Value::string("a"), Value::symbol("vm-no-such-coding")])
            .expect_err("unknown coding-system should signal coding-system-error");
    match unknown_decode {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "coding-system-error");
            assert_eq!(sig.data, vec![Value::symbol("vm-no-such-coding")]);
        }
        other => panic!("expected signal, got: {other:?}"),
    }

    let unibyte_val = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xE9]));
    let decoded_unibyte = builtin_decode_coding_string(vec![unibyte_val, Value::symbol("utf-8")])
        .expect("decode-coding-string should preserve invalid bytes");
    let decoded_ls = decoded_unibyte
        .as_lisp_string()
        .expect("decode-coding-string should return string");
    // 0xE9 is invalid UTF-8, so it becomes raw-byte char 0x3FFF00 + 0xE9
    let codes: Vec<u32> = crate::emacs_core::builtins::lisp_string_char_codes(decoded_ls);
    assert_eq!(codes, vec![0x3FFF00 + 0xE9]);

    let unibyte_val2 = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xE9]));
    let encoded_unibyte = builtin_encode_coding_string(vec![unibyte_val2, Value::symbol("utf-8")])
        .expect("encode-coding-string should preserve unibyte bytes");
    let encoded_ls = encoded_unibyte.as_lisp_string().unwrap();
    assert_eq!(encoded_ls.as_bytes(), &[0xE9]);
}

/// Issue #131: decoding valid UTF-8 for real Private-Use-Area glyphs (nerd-font
/// icons) must keep their real code points, while truly invalid bytes still
/// become eight-bit chars. Before the storage rework, decode-coding-string
/// routed through the in-Unicode sentinel storage-string, so a real U+E322
/// (weather icon) collided with the unibyte sentinel and an eight-bit raw byte
/// collided with the raw-byte sentinel — corrupting glyphs.
#[test]
fn decode_coding_string_keeps_real_pua_glyphs_issue_131() {
    crate::test_utils::init_test_tracing();
    // U+E0A0 (powerline) + U+E322 (nerd-font weather) + a genuine invalid byte.
    let mut input = Vec::new();
    input.extend_from_slice("\u{E0A0}\u{E322}".as_bytes()); // valid UTF-8 PUA
    input.push(0xFF); // invalid byte -> eight-bit char 0x3FFFFF
    let unibyte_val = Value::heap_string(crate::heap_types::LispString::from_unibyte(input));
    let decoded = builtin_decode_coding_string(vec![unibyte_val, Value::symbol("utf-8")])
        .expect("decode-coding-string should succeed");
    let ls = decoded.as_lisp_string().expect("string result");
    let codes = crate::emacs_core::builtins::lisp_string_char_codes(ls);
    assert_eq!(
        codes,
        vec![0xE0A0, 0xE322, 0x3FFF00 + 0xFF],
        "real PUA glyphs must keep their code points; only the invalid byte is eight-bit"
    );
}

#[test]
fn decode_coding_string_rejects_six_byte_utf8_emacs_sequence_as_raw_bytes() {
    crate::test_utils::init_test_tracing();
    // GNU Emacs' internal multibyte form has MAX_MULTIBYTE_LENGTH == 5.
    // This six-byte sequence previously decoded to 0x1003162F and then
    // tripped `char_string`'s valid-character assertion.
    let input = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
        0xFC, 0x90, 0x80, 0xB1, 0x98, 0xAF,
    ]));
    let decoded = builtin_decode_coding_string(vec![input, Value::symbol("utf-8")])
        .expect("decode-coding-string should keep malformed bytes raw");
    let ls = decoded.as_lisp_string().expect("string result");
    let codes = crate::emacs_core::builtins::lisp_string_char_codes(ls);
    assert_eq!(
        codes,
        vec![
            0x3FFF00 + 0xFC,
            0x3FFF00 + 0x90,
            0x3FFF00 + 0x80,
            0x3FFF00 + 0xB1,
            0x3FFF00 + 0x98,
            0x3FFF00 + 0xAF,
        ]
    );
}

#[test]
fn decode_coding_string_rejects_out_of_range_five_byte_utf8_emacs_sequence_as_raw_bytes() {
    crate::test_utils::init_test_tracing();
    // MAX_CHAR is 0x3FFFFF and MAX_5_BYTE_CHAR is 0x3FFF7F, so this
    // five-byte form would be out of Neo/GNU's valid Emacs character range.
    let input = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
        0xF8, 0x90, 0x80, 0x80, 0x80,
    ]));
    let decoded = builtin_decode_coding_string(vec![input, Value::symbol("utf-8")])
        .expect("decode-coding-string should keep out-of-range forms raw");
    let ls = decoded.as_lisp_string().expect("string result");
    let codes = crate::emacs_core::builtins::lisp_string_char_codes(ls);
    assert_eq!(
        codes,
        vec![
            0x3FFF00 + 0xF8,
            0x3FFF00 + 0x90,
            0x3FFF00 + 0x80,
            0x3FFF00 + 0x80,
            0x3FFF00 + 0x80,
        ]
    );
}

#[test]
fn encode_coding_string_extracts_multibyte_byte8_chars_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"Hello ");
    let mut char_buf = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
    for byte in [0xC3, 0xA9] {
        let len = crate::emacs_core::emacs_char::char_string(
            crate::emacs_core::emacs_char::byte8_to_char(byte),
            &mut char_buf,
        );
        bytes.extend_from_slice(&char_buf[..len]);
    }
    let source = Value::heap_string(crate::heap_types::LispString::from_emacs_bytes(bytes));

    for coding in ["utf-8", "utf-8-emacs", "raw-text", "no-conversion"] {
        let encoded = builtin_encode_coding_string(vec![source, Value::symbol(coding)])
            .expect("encode-coding-string should evaluate");
        let ls = encoded
            .as_lisp_string()
            .expect("encode-coding-string should return a string");
        assert_eq!(ls.as_bytes(), b"Hello \xC3\xA9", "{coding}");
        assert!(
            !ls.is_multibyte(),
            "{coding} result should be a unibyte byte string"
        );
    }
}

#[test]
fn decode_coding_string_extracts_multibyte_byte8_source_bytes_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut bytes = Vec::new();
    for byte in [0xC3, 0xA9] {
        let mut char_buf = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
        let len = crate::emacs_core::emacs_char::char_string(
            crate::emacs_core::emacs_char::byte8_to_char(byte),
            &mut char_buf,
        );
        bytes.extend_from_slice(&char_buf[..len]);
    }
    let source = Value::heap_string(crate::heap_types::LispString::from_emacs_bytes(bytes));

    let decoded = builtin_decode_coding_string(vec![source, Value::symbol("utf-8")])
        .expect("decode-coding-string should evaluate");
    let ls = decoded
        .as_lisp_string()
        .expect("decode-coding-string should return a string");
    assert_eq!(
        crate::emacs_core::builtins::lisp_string_char_codes(ls),
        [0xE9]
    );
    assert_eq!(ls.schars(), 1);
    assert_eq!(ls.as_bytes(), "é".as_bytes());
}

#[test]
fn nil_coding_string_respects_nocopy_identity() {
    crate::test_utils::init_test_tracing();
    let source = Value::string("abc");

    let encoded_copy =
        builtin_encode_coding_string(vec![source, Value::NIL, Value::NIL]).expect("nil coding");
    assert_eq!(encoded_copy.as_utf8_str(), Some("abc"));
    assert!(!crate::emacs_core::value::eq_value(&source, &encoded_copy));

    let encoded_nocopy =
        builtin_encode_coding_string(vec![source, Value::NIL, Value::T]).expect("nil coding");
    assert!(crate::emacs_core::value::eq_value(&source, &encoded_nocopy));

    let decoded_copy =
        builtin_decode_coding_string(vec![source, Value::NIL, Value::NIL]).expect("nil coding");
    assert_eq!(decoded_copy.as_utf8_str(), Some("abc"));
    assert!(!crate::emacs_core::value::eq_value(&source, &decoded_copy));

    let decoded_nocopy =
        builtin_decode_coding_string(vec![source, Value::NIL, Value::T]).expect("nil coding");
    assert!(crate::emacs_core::value::eq_value(&source, &decoded_nocopy));
}

#[test]
fn ascii_coding_string_respects_nocopy_fast_path_identity() {
    crate::test_utils::init_test_tracing();
    let source = Value::string("abc");

    let encoded_copy =
        builtin_encode_coding_string(vec![source, Value::symbol("utf-8"), Value::NIL])
            .expect("utf-8 encode");
    assert_eq!(encoded_copy.as_utf8_str(), Some("abc"));
    assert!(!crate::emacs_core::value::eq_value(&source, &encoded_copy));

    let encoded_nocopy =
        builtin_encode_coding_string(vec![source, Value::symbol("utf-8"), Value::T])
            .expect("utf-8 encode");
    assert!(crate::emacs_core::value::eq_value(&source, &encoded_nocopy));

    let decoded_copy =
        builtin_decode_coding_string(vec![source, Value::symbol("utf-8"), Value::NIL])
            .expect("utf-8 decode");
    assert_eq!(decoded_copy.as_utf8_str(), Some("abc"));
    assert!(!crate::emacs_core::value::eq_value(&source, &decoded_copy));

    let decoded_nocopy =
        builtin_decode_coding_string(vec![source, Value::symbol("utf-8"), Value::T])
            .expect("utf-8 decode");
    assert!(crate::emacs_core::value::eq_value(&source, &decoded_nocopy));
}

#[test]
fn ascii_coding_string_nocopy_allocates_when_eol_conversion_needed() {
    crate::test_utils::init_test_tracing();
    let encode_source = Value::string("a\nb");
    let encoded =
        builtin_encode_coding_string(vec![encode_source, Value::symbol("utf-8-dos"), Value::T])
            .expect("utf-8-dos encode");
    assert_eq!(encoded.as_lisp_string().unwrap().as_bytes(), b"a\r\nb");
    assert!(!crate::emacs_core::value::eq_value(
        &encode_source,
        &encoded
    ));

    let decode_source = Value::heap_string(crate::heap_types::LispString::from_unibyte(
        b"a\r\nb".to_vec(),
    ));
    let decoded =
        builtin_decode_coding_string(vec![decode_source, Value::symbol("utf-8-dos"), Value::T])
            .expect("utf-8-dos decode");
    assert_eq!(decoded.as_utf8_str(), Some("a\nb"));
    assert!(!crate::emacs_core::value::eq_value(
        &decode_source,
        &decoded
    ));
}

#[test]
fn encode_coding_string_buffer_destination_inserts_without_moving_point() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    let dest = eval.buffers.create_buffer("*encode-coding-string-dest*");
    eval.buffers
        .insert_lisp_string_into_buffer(dest, &crate::heap_types::LispString::from_utf8("XY"))
        .expect("insert destination seed");
    eval.buffers
        .goto_buffer_emacs_byte_pos(dest, crate::buffer::EmacsBytePos::new(1))
        .expect("move destination point");

    let produced = builtin_encode_coding_string_in_context(
        &mut eval,
        vec![
            Value::string("a\n"),
            Value::symbol("utf-8-dos"),
            Value::NIL,
            Value::make_buffer(dest),
        ],
    )
    .expect("encode-coding-string should insert in destination buffer");

    assert_eq!(produced, Value::fixnum(3));
    assert_eq!(
        eval.visible_variable_value_or_nil("last-coding-system-used"),
        Value::symbol("utf-8-dos")
    );
    let buf = eval.buffers.get(dest).expect("destination buffer");
    assert_eq!(buf.buffer_string(), "Xa\r\nY");
    assert_eq!(buf.point_emacs_byte_pos().get(), 1);
}

#[test]
fn decode_coding_string_buffer_destination_inserts_without_moving_point() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    let dest = eval.buffers.create_buffer("*decode-coding-string-dest*");
    let encoded = Value::heap_string(crate::heap_types::LispString::from_unibyte(
        b"a\r\nb".to_vec(),
    ));

    let produced = builtin_decode_coding_string_in_context(
        &mut eval,
        vec![
            encoded,
            Value::symbol("utf-8-dos"),
            Value::NIL,
            Value::make_buffer(dest),
        ],
    )
    .expect("decode-coding-string should insert in destination buffer");

    assert_eq!(produced, Value::fixnum(3));
    assert_eq!(
        eval.visible_variable_value_or_nil("last-coding-system-used"),
        Value::symbol("utf-8-dos")
    );
    let buf = eval.buffers.get(dest).expect("destination buffer");
    assert_eq!(buf.buffer_string(), "a\nb");
    assert_eq!(buf.point_emacs_byte_pos().get(), 0);
}

#[test]
fn builtin_coding_string_helpers_accept_iso_8859_15_alias() {
    crate::test_utils::init_test_tracing();
    let encoded =
        builtin_encode_coding_string(vec![Value::string("abc"), Value::symbol("iso-8859-15")])
            .expect("iso-8859-15 should be accepted as a known coding system");
    assert_eq!(encoded.as_utf8_str(), Some("abc"));

    let decoded =
        builtin_decode_coding_string(vec![Value::string("abc"), Value::symbol("iso-8859-15")])
            .expect("iso-8859-15 should be accepted as a known coding system");
    assert_eq!(decoded.as_utf8_str(), Some("abc"));
}

#[test]
fn decode_iso_8859_15_attaches_its_source_charset_property() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    let encoded = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
        0xE9, 0xA4,
    ]));

    let decoded = builtin_decode_coding_string_in_context(
        &mut eval,
        vec![encoded, Value::symbol("iso-8859-15")],
    )
    .expect("ISO-8859-15 decode should succeed");

    assert_eq!(
        decoded.as_lisp_string().and_then(|s| s.as_utf8_str()),
        Some("é€")
    );
    let props = get_string_text_properties_for_value(decoded)
        .expect("decoded ISO-8859-15 string should retain its source charset");
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].start, 0);
    assert_eq!(props[0].end, 2);
    assert_eq!(
        props[0].plist,
        Value::list(vec![Value::symbol("charset"), Value::symbol("iso-8859-15"),])
    );
}

#[test]
fn encode_lisp_string_emacs_internal_uses_utf8_emacs_alias() {
    crate::test_utils::init_test_tracing();
    let text = crate::heap_types::LispString::from_utf8("abc\n");

    assert_eq!(encode_lisp_string(&text, "emacs-internal"), b"abc\n");
    assert_eq!(encode_lisp_string(&text, "emacs-internal-dos"), b"abc\r\n");
}

#[test]
fn builtin_coding_string_helpers_accept_iso_8859_9_alias() {
    crate::test_utils::init_test_tracing();
    let encoded =
        builtin_encode_coding_string(vec![Value::string("abc"), Value::symbol("iso-8859-9")])
            .expect("iso-8859-9 should be accepted as a known coding system");
    assert_eq!(encoded.as_utf8_str(), Some("abc"));

    let decoded =
        builtin_decode_coding_string(vec![Value::string("abc"), Value::symbol("iso-8859-9")])
            .expect("iso-8859-9 should be accepted as a known coding system");
    assert_eq!(decoded.as_utf8_str(), Some("abc"));
}

#[test]
fn decode_latin1_attaches_charset_text_property() {
    crate::test_utils::init_test_tracing();
    let encoded = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xE9]));
    let decoded = builtin_decode_coding_string(vec![encoded, Value::symbol("latin-1")])
        .expect("latin-1 decode should succeed");
    if !decoded.is_string() {
        panic!("decode-coding-string should return a string");
    };
    let props = get_string_text_properties_for_value(decoded)
        .expect("decoded string should be propertized");
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].start, 0);
    assert_eq!(props[0].end, 1);
    assert_eq!(
        props[0].plist,
        Value::list(vec![Value::symbol("charset"), Value::symbol("iso-8859-1")])
    );
}

#[test]
fn decode_latin1_charset_property_spans_ascii_like_gnu() {
    crate::test_utils::init_test_tracing();
    let encoded = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
        b'A', 0xE9, b'B',
    ]));
    let decoded = builtin_decode_coding_string(vec![encoded, Value::symbol("latin-1")])
        .expect("latin-1 decode should succeed");

    assert_eq!(
        decoded.as_lisp_string().and_then(|s| s.as_utf8_str()),
        Some("AéB")
    );
    let props = get_string_text_properties_for_value(decoded)
        .expect("decoded Latin-1 string should be propertized");
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].start, 0);
    assert_eq!(props[0].end, 3);
    assert_eq!(
        props[0].plist,
        Value::list(vec![Value::symbol("charset"), Value::symbol("iso-8859-1")])
    );
}

#[test]
fn encode_no_conversion_preserves_unibyte_storage_bytes() {
    crate::test_utils::init_test_tracing();
    let source = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xE9]));
    let encoded =
        builtin_encode_coding_string(vec![source, Value::symbol("no-conversion")]).unwrap();
    if !encoded.is_string() {
        panic!("encode-coding-string should return a string");
    };
    assert!(!encoded.string_is_multibyte());
    let ls = encoded.as_lisp_string().unwrap();
    assert_eq!(ls.as_bytes(), &[0xE9]);
}

#[test]
fn decode_no_conversion_returns_unibyte_bytes_for_non_ascii_input() {
    crate::test_utils::init_test_tracing();
    let encoded =
        builtin_encode_coding_string(vec![Value::string("é"), Value::symbol("no-conversion")])
            .expect("encoding should succeed");
    let decoded =
        builtin_decode_coding_string(vec![encoded, Value::symbol("no-conversion")]).unwrap();
    if !decoded.is_string() {
        panic!("decode-coding-string should return a string");
    };
    assert!(!decoded.string_is_multibyte());
    let ls = decoded.as_lisp_string().unwrap();
    assert_eq!(ls.as_bytes(), &[0xC3, 0xA9]);
}

#[test]
fn decode_byte_preserving_ascii_uses_multibyte_fast_path() {
    crate::test_utils::init_test_tracing();
    for coding in ["binary", "no-conversion", "raw-text"] {
        let source = Value::heap_string(crate::heap_types::LispString::from_unibyte(
            b"Hello".to_vec(),
        ));
        let decoded =
            builtin_decode_coding_string(vec![source, Value::symbol(coding)]).expect("decode");
        assert!(
            decoded.string_is_multibyte(),
            "{coding} should decode ASCII as multibyte"
        );
        assert_eq!(decoded.as_lisp_string().unwrap().as_bytes(), b"Hello");
    }
}

#[test]
fn char_byte_conversion() {
    crate::test_utils::init_test_tracing();
    let s = "hello中文";
    assert_eq!(char_to_byte_pos(s, 5), 5);
    assert_eq!(char_to_byte_pos(s, 6), 8); // '中' is 3 bytes
    assert_eq!(byte_to_char_pos(s, 5), 5);
    assert_eq!(byte_to_char_pos(s, 8), 6);
}

#[test]
fn encoding_utf8() {
    crate::test_utils::init_test_tracing();
    let bytes = encode_string("hello", "utf-8");
    assert_eq!(bytes, b"hello");
    let decoded = decode_bytes(b"hello", "utf-8");
    assert_eq!(decoded, "hello");
}

#[test]
fn encoding_utf8_with_signature_consumes_leading_bom() {
    crate::test_utils::init_test_tracing();
    let decoded = decode_bytes(b"\xEF\xBB\xBF;;; fixture\n", "utf-8-with-signature-unix");
    assert_eq!(decoded, ";;; fixture\n");
}

#[test]
fn encoding_utf16_gnu_compatible_signatures_and_endianness() {
    crate::test_utils::init_test_tracing();
    assert_eq!(encode_string("a", "utf-16"), vec![0xfe, 0xff, 0x00, 0x61]);
    assert_eq!(
        encode_string("a", "utf-16-be"),
        vec![0xfe, 0xff, 0x00, 0x61]
    );
    assert_eq!(encode_string("a", "utf-16be"), vec![0x00, 0x61]);
    assert_eq!(
        encode_string("a", "utf-16-le"),
        vec![0xff, 0xfe, 0x61, 0x00]
    );
    assert_eq!(encode_string("a", "utf-16le"), vec![0x61, 0x00]);

    assert_eq!(decode_bytes(&[0x00, 0x61], "utf-16be"), "a");
    assert_eq!(decode_bytes(&[0x61, 0x00], "utf-16le"), "a");
    assert_eq!(
        decode_bytes(&[0xff, 0xfe, 0x3d, 0xd8, 0x00, 0xde], "utf-16-be"),
        "\u{1f600}"
    );

    let encoded =
        builtin_encode_coding_string(vec![Value::string("a"), Value::symbol("utf-16-be")])
            .expect("utf-16-be should be a valid coding system");
    let encoded_string = encoded
        .as_lisp_string()
        .expect("encode-coding-string should return a string");
    assert_eq!(encoded_string.as_bytes(), &[0xfe, 0xff, 0x00, 0x61]);
}

#[test]
fn encoding_utf8_dos_applies_eol_conversion() {
    crate::test_utils::init_test_tracing();
    let bytes = encode_string("a\nb", "utf-8-dos");
    assert_eq!(bytes, b"a\r\nb");
    let decoded = decode_bytes(b"a\r\nb", "utf-8-dos");
    assert_eq!(decoded, "a\nb");
}

#[test]
fn raw_text_dos_preserves_bytes_but_converts_eol() {
    crate::test_utils::init_test_tracing();
    let encoded =
        builtin_encode_coding_string(vec![Value::string("a\nb"), Value::symbol("raw-text-dos")])
            .unwrap();
    if !encoded.is_string() {
        panic!("encode-coding-string should return a string");
    };
    let ls = encoded.as_lisp_string().unwrap();
    assert_eq!(ls.as_bytes(), b"a\r\nb");

    let decoded = builtin_decode_coding_string(vec![
        Value::heap_string(crate::heap_types::LispString::from_unibyte(
            b"a\r\nb".to_vec(),
        )),
        Value::symbol("raw-text-dos"),
    ])
    .unwrap();
    if !decoded.is_string() {
        panic!("decode-coding-string should return a string");
    };
    let ls = decoded.as_lisp_string().unwrap();
    assert_eq!(ls.as_bytes(), b"a\nb");
}

#[test]
fn encode_coding_region_destination_t_returns_encoded_string() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .insert_lisp_string_into_buffer(current, &crate::heap_types::LispString::from_utf8("é"))
        .expect("insert source text");

    let encoded = builtin_encode_coding_region(
        &mut eval,
        vec![
            Value::fixnum(1),
            Value::fixnum(2),
            Value::symbol("utf-8"),
            Value::T,
        ],
    )
    .expect("encode-coding-region should return a string destination");
    let encoded = encoded.as_lisp_string().expect("encoded string");
    assert!(!encoded.is_multibyte());
    assert_eq!(encoded.as_bytes(), &[0xC3, 0xA9]);
    assert_eq!(
        eval.visible_variable_value_or_nil("last-coding-system-used"),
        Value::symbol("utf-8")
    );

    let buffer_text = eval
        .buffers
        .get(current)
        .expect("current buffer")
        .buffer_string();
    assert_eq!(buffer_text, "é");
}

#[test]
fn decode_coding_region_replaces_current_region() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .insert_lisp_string_into_buffer(
            current,
            &crate::heap_types::LispString::from_utf8("a\r\nb"),
        )
        .expect("insert encoded bytes");

    let produced = builtin_decode_coding_region(
        &mut eval,
        vec![
            Value::fixnum(1),
            Value::fixnum(5),
            Value::symbol("utf-8-dos"),
            Value::NIL,
        ],
    )
    .expect("decode-coding-region should replace the region");
    assert_eq!(produced, Value::fixnum(3));
    assert_eq!(
        eval.visible_variable_value_or_nil("last-coding-system-used"),
        Value::symbol("utf-8-dos")
    );

    let buffer = eval.buffers.get(current).expect("current buffer");
    assert_eq!(buffer.buffer_string(), "a\nb");
    assert_eq!(buffer.point_min_char_pos().get(), 0);
    assert_eq!(buffer.point_max_char_pos().get(), 3);
}

#[test]
fn decode_coding_region_into_unibyte_buffer_stores_internal_bytes() {
    // GNU `decode_coding_object` sets `dst_multibyte` from the destination
    // buffer's `enable-multibyte-characters` (coding.c:8153).  Decoding into a
    // *unibyte* buffer must store each decoded character's internal byte
    // sequence (dst_multibyte = 0), NOT truncate the character to one low byte.
    // `tit-dic-convert` (LEIM generation) relies on this idiom:
    //   (set-buffer-multibyte nil) (decode-coding-region ...) (set-buffer-multibyte t)
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .set_buffer_multibyte_flag(current, false)
        .expect("make current buffer unibyte");
    // Raw GB2312 bytes for 一 (U+4E00).
    eval.buffers
        .insert_lisp_string_into_buffer(
            current,
            &crate::heap_types::LispString::from_unibyte(vec![0xd2, 0xbb]),
        )
        .expect("insert raw GB2312 bytes");

    builtin_decode_coding_region(
        &mut eval,
        vec![
            Value::fixnum(1),
            Value::fixnum(3),
            Value::symbol("euc-china"),
            Value::NIL,
        ],
    )
    .expect("decode-coding-region should replace the region");

    let buffer = eval.buffers.get(current).expect("current buffer");
    assert!(!buffer.get_multibyte(), "destination buffer stays unibyte");
    let len: usize = buffer.total_emacs_byte_len().into();
    let bytes =
        buffer.buffer_substring_bytes_range(crate::buffer::EmacsByteRange::from_usize(0, len));
    // 一 = U+4E00; internal/utf-8 bytes E4 B8 80.  A later `set-buffer-multibyte
    // t` reinterprets these bytes back into the character.
    assert_eq!(bytes, vec![0xe4, 0xb8, 0x80]);
}

#[test]
fn undecided_write_encoding_preserves_bytes_and_converts_eol() {
    crate::test_utils::init_test_tracing();

    let encoded = builtin_encode_coding_string(vec![
        Value::string("alpha\nomega"),
        Value::symbol("undecided-unix"),
    ])
    .unwrap();
    let ls = encoded
        .as_lisp_string()
        .expect("encode-coding-string should return a string");
    assert_eq!(ls.as_bytes(), b"alpha\nomega");

    let encoded = builtin_encode_coding_string(vec![
        Value::string("alpha\nomega"),
        Value::symbol("undecided-dos"),
    ])
    .unwrap();
    let ls = encoded
        .as_lisp_string()
        .expect("encode-coding-string should return a string");
    assert_eq!(ls.as_bytes(), b"alpha\r\nomega");
}

#[test]
fn encoding_latin1() {
    crate::test_utils::init_test_tracing();
    let bytes = encode_string("café", "latin-1");
    assert_eq!(bytes.len(), 4); // é maps to 0xe9
    let decoded = decode_bytes(&[0x63, 0x61, 0x66, 0xe9], "latin-1");
    assert_eq!(decoded, "café");
}

#[test]
fn encoding_big5_decodes_generated_leim_dictionary_bytes_like_gnu() {
    crate::test_utils::init_test_tracing();
    assert_eq!(decode_bytes(&[0xa4, 0x40], "big5"), "\u{4e00}");
    assert_eq!(decode_bytes(&[0xa4, 0x40], "chinese-big5-unix"), "\u{4e00}");
    assert_eq!(decode_bytes(&[0xa4, 0x40], "cp950"), "\u{4e00}");
    assert_eq!(
        decode_bytes(&[0xa4, 0x40, b'\r', b'\n'], "big5-dos"),
        "一\n"
    );
    assert_eq!(encode_string("一", "big5"), vec![0xa4, 0x40]);
    assert_eq!(encode_string("一", "chinese-big5-unix"), vec![0xa4, 0x40]);
}

#[test]
fn encoding_gb2312_decodes_generated_leim_dictionary_bytes_like_gnu() {
    crate::test_utils::init_test_tracing();
    assert_eq!(decode_bytes(&[0xd2, 0xbb], "cn-gb-2312"), "一");
    assert_eq!(decode_bytes(&[0xd2, 0xbb], "chinese-iso-8bit-unix"), "一");
    assert_eq!(
        decode_bytes(&[0xd2, 0xbb, b'\r', b'\n'], "gb2312-dos"),
        "一\n"
    );
    assert_eq!(encode_string("一", "cn-gb-2312"), vec![0xd2, 0xbb]);
    assert_eq!(
        encode_string("一", "chinese-iso-8bit-unix"),
        vec![0xd2, 0xbb]
    );
}

#[test]
fn decode_coding_string_big5_marks_charset_like_gnu() {
    crate::test_utils::init_test_tracing();
    let decoded = builtin_decode_coding_string(vec![
        Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
            0xa4, 0x40,
        ])),
        Value::symbol("chinese-big5-unix"),
    ])
    .expect("decode-coding-string chinese-big5-unix should succeed");

    assert_eq!(
        decoded.as_lisp_string().and_then(|s| s.as_utf8_str()),
        Some("一")
    );
    let props = get_string_text_properties_for_value(decoded)
        .expect("decoded Big5 string should be propertized");
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].start, 0);
    assert_eq!(props[0].end, 1);
    assert_eq!(
        props[0].plist,
        Value::list(vec![Value::symbol("charset"), Value::symbol("big5")])
    );
}

#[test]
fn decode_coding_string_gb2312_marks_charset_like_gnu() {
    crate::test_utils::init_test_tracing();
    let decoded = builtin_decode_coding_string(vec![
        Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
            0xd2, 0xbb,
        ])),
        Value::symbol("cn-gb-2312-unix"),
    ])
    .expect("decode-coding-string cn-gb-2312-unix should succeed");

    assert_eq!(
        decoded.as_lisp_string().and_then(|s| s.as_utf8_str()),
        Some("一")
    );
    let props = get_string_text_properties_for_value(decoded)
        .expect("decoded GB2312 string should be propertized");
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].start, 0);
    assert_eq!(props[0].end, 1);
    assert_eq!(
        props[0].plist,
        Value::list(vec![
            Value::symbol("charset"),
            Value::symbol("chinese-gb2312"),
        ])
    );
}

#[test]
fn decode_coding_string_gb2312_extends_charset_after_first_non_ascii_like_gnu() {
    crate::test_utils::init_test_tracing();
    let decoded = builtin_decode_coding_string(vec![
        Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![
            b'A', 0xd2, 0xbb, b'B',
        ])),
        Value::symbol("cn-gb-2312-unix"),
    ])
    .expect("decode-coding-string cn-gb-2312-unix should succeed");

    assert_eq!(
        decoded.as_lisp_string().and_then(|s| s.as_utf8_str()),
        Some("A一B")
    );
    let props = get_string_text_properties_for_value(decoded)
        .expect("decoded GB2312 string should be propertized");
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].start, 1);
    assert_eq!(props[0].end, 3);
    assert_eq!(
        props[0].plist,
        Value::list(vec![
            Value::symbol("charset"),
            Value::symbol("chinese-gb2312"),
        ])
    );
}

#[test]
fn glyphless_display() {
    crate::test_utils::init_test_tracing();
    assert_eq!(glyphless_char_display('\x01'), "^A");
    assert_eq!(glyphless_char_display('\x7f'), "^?");
    assert_eq!(glyphless_char_display('\u{FEFF}'), "\\uFEFF");
}

#[test]
fn multibyte_detection() {
    crate::test_utils::init_test_tracing();
    assert!(!is_multibyte_string("hello"));
    assert!(is_multibyte_string("héllo"));
    assert!(is_multibyte_string("中文"));
}

#[test]
fn multibyte_detection_treats_unibyte_storage_as_unibyte() {
    crate::test_utils::init_test_tracing();
    assert!(!is_multibyte_string("abc"));
    // Pure ASCII is not multibyte
    assert!(!is_multibyte_string("hello"));
}

#[test]
fn builtin_multibyte_string_p_matches_oracle_non_string_and_unibyte_storage() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_multibyte_string_p(vec![Value::string("abc")]).unwrap(),
        Value::NIL
    );
    assert_eq!(
        builtin_multibyte_string_p(vec![Value::string("é")]).unwrap(),
        Value::T
    );

    let unibyte_val =
        Value::heap_string(crate::heap_types::LispString::from_unibyte(b"abc".to_vec()));
    assert_eq!(
        builtin_multibyte_string_p(vec![unibyte_val]).unwrap(),
        Value::NIL
    );

    assert_eq!(
        builtin_multibyte_string_p(vec![Value::fixnum(1)]).unwrap(),
        Value::NIL
    );
}

#[test]
fn builtin_unibyte_string_p_basics() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_unibyte_string_p(vec![Value::string("hello")]).unwrap(),
        Value::T
    );
    assert_eq!(
        builtin_unibyte_string_p(vec![Value::string("héllo")]).unwrap(),
        Value::NIL
    );
}

#[test]
fn builtin_unibyte_string_p_errors() {
    crate::test_utils::init_test_tracing();
    // Wrong arity signals error.
    assert!(builtin_unibyte_string_p(vec![]).is_err());
    // Non-string arg returns nil (type predicates don't error on wrong type).
    assert_eq!(
        builtin_unibyte_string_p(vec![Value::fixnum(1)]).unwrap(),
        Value::NIL
    );
}

#[test]
fn printable_check() {
    crate::test_utils::init_test_tracing();
    assert!(is_printable('a'));
    assert!(is_printable('中'));
    assert!(!is_printable('\x00'));
    assert!(!is_printable('\x7f'));
}

// ===========================================================================
// decode-coding-string charset text property for ISO-2022 / EUC / Shift-JIS
// (bug 13).  GNU's `decode_coding_iso_2022` annotates each decoded run with the
// source charset (`(charset CHARSET)`).  The pure decoders now emit those runs;
// here we drive them directly with deterministic OFFSET charsets so the result
// is reproducible in the bare test registry.
// ===========================================================================

/// Register a deterministic dim-2 ISO-2022 charset (`test-jis`, ISO final 'B',
/// OFFSET method) so `charset_decode_char` maps a 2-byte code to a non-ASCII
/// char and `charset_iso2022_designation` recognizes the `$B` designation.
fn register_test_jis() -> crate::emacs_core::intern::SymId {
    use crate::emacs_core::value::Value;
    let mut args = vec![Value::NIL; 17];
    args[0] = Value::symbol("test-jis");
    args[1] = Value::fixnum(2);
    args[2] = Value::vector(vec![
        Value::fixnum(33),
        Value::fixnum(126),
        Value::fixnum(33),
        Value::fixnum(126),
    ]);
    args[5] = Value::fixnum(66); // ISO final 'B'
    args[11] = Value::fixnum(0x10000); // code-offset -> decoded chars are non-ASCII
    crate::emacs_core::charset::builtin_define_charset_internal(args).unwrap();
    crate::emacs_core::intern::intern("test-jis")
}

#[test]
fn undecided_decode_uses_the_shared_iso_2022_detector() {
    crate::test_utils::init_test_tracing();
    register_test_jis();
    let mut eval = crate::emacs_core::Context::new();
    crate::emacs_core::coding::builtin_define_coding_system_internal(
        &mut eval.coding_systems,
        vec![
            Value::symbol("iso-2022-7bit"),
            Value::char('J'),
            Value::symbol("iso-2022"),
            Value::symbol("iso-2022"),
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::fixnum('?' as i64),
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::vector(vec![
                Value::symbol("ascii"),
                Value::NIL,
                Value::NIL,
                Value::NIL,
            ]),
            Value::cons(Value::fixnum(0), Value::fixnum(0)),
            Value::NIL,
            Value::fixnum(
                (crate::emacs_core::coding::IsoFlag::SevenBits as u32
                    | crate::emacs_core::coding::IsoFlag::Designation as u32)
                    .into(),
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        detect_undecided_coding(&eval.coding_systems, b"\x1b$B$3$s\x1b(B", false,),
        Some(crate::emacs_core::intern::intern("iso-2022-7bit"))
    );
}

fn charset_prop(charset: &str) -> Value {
    Value::list(vec![Value::symbol("charset"), Value::symbol(charset)])
}

#[test]
fn decode_iso2022_attaches_charset_property_for_designated_charset() {
    crate::test_utils::init_test_tracing();
    use crate::emacs_core::intern::intern;
    register_test_jis();
    let ascii = intern("ascii");
    // 7-bit ISO-2022 with escape designations (iso-2022-7bit profile).
    let spec = crate::emacs_core::coding::Iso2022Spec {
        initial: [Some(ascii), None, None, None],
        request: vec![],
        reg_usage: (0, 1),
        flags: enumflags2::BitFlags::from_flag(crate::emacs_core::coding::IsoFlag::SevenBits)
            | crate::emacs_core::coding::IsoFlag::Designation,
    };
    // "X" + ESC$B designation + two dim-2 chars + ESC(B (ascii) + "Y".
    let (bytes, runs) = decode_via_iso2022(b"X\x1b$B$3$s\x1b(BY", &spec);
    let text = crate::emacs_core::emacs_char::to_utf8_lossy(&bytes);
    // X + 2 kanji + Y == 4 chars.  The run starts at the first kanji and runs
    // to the end of the text: re-designating G0 back to `ascii` does not close
    // it, because GNU's `decode_coding_iso_2022` only moves `last_id` for a
    // non-ASCII charset.  GNU Emacs 31 decoding the same bytes returns
    // `#("XこんY" 1 4 (charset japanese-jisx0208))`.
    assert_eq!(text.chars().count(), 4);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].start, 1);
    assert_eq!(runs[0].end, 4);
    assert_eq!(runs[0].plist, charset_prop("test-jis"));
}

#[test]
fn iso2022_encoding_boundary_controls_final_ascii_reset() {
    crate::test_utils::init_test_tracing();
    use crate::emacs_core::intern::intern;

    let jis = register_test_jis();
    let spec = crate::emacs_core::coding::Iso2022Spec {
        initial: [Some(intern("ascii")), None, None, None],
        request: vec![],
        reg_usage: (0, 1),
        flags: enumflags2::BitFlags::from_flag(crate::emacs_core::coding::IsoFlag::SevenBits)
            | crate::emacs_core::coding::IsoFlag::Designation
            | crate::emacs_core::coding::IsoFlag::AsciiAtEol,
    };
    let source = crate::heap_types::LispString::from_utf8("\u{10000}");

    let complete = encode_via_iso2022(
        &source,
        &spec,
        &[intern("ascii"), jis],
        EncodingBoundary::CompleteText,
    );
    let file_region = encode_via_iso2022(
        &source,
        &spec,
        &[intern("ascii"), jis],
        EncodingBoundary::FileRegion,
    );

    assert_eq!(complete, b"\x1b$B!!\x1b(B");
    assert_eq!(file_region, b"\x1b$B!!");
}

#[test]
fn ccl_coding_system_executes_registered_identity_program() {
    crate::test_utils::init_test_tracing();
    crate::emacs_core::ccl::reset_ccl_registry();

    let mut eval = crate::emacs_core::Context::new();

    let program = Value::vector(
        [1, 5, 14, -249, -500, 22]
            .into_iter()
            .map(Value::fixnum)
            .collect(),
    );
    crate::emacs_core::ccl::builtin_register_ccl_program_impl(vec![
        Value::symbol("test-ccl-identity"),
        program,
    ])
    .expect("register compiled identity CCL program");

    crate::emacs_core::coding::builtin_define_coding_system_internal(
        &mut eval.coding_systems,
        vec![
            Value::symbol("test-ccl-identity-coding"),
            Value::char('C'),
            Value::symbol("ccl"),
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::fixnum('?' as i64),
            Value::NIL,
            Value::NIL,
            Value::symbol("unix"),
            Value::symbol("test-ccl-identity"),
            Value::symbol("test-ccl-identity"),
            Value::NIL,
        ],
    )
    .expect("define CCL coding system");

    assert_eq!(
        fmt(
            &mut eval,
            r#"(let* ((packet (unibyte-string 0 1 65 127 128 255))
                       (wire (encode-coding-string
                              packet 'test-ccl-identity-coding))
                       (decoded (decode-coding-string
                                 wire 'test-ccl-identity-coding)))
                  (list (append wire nil)
                        (append decoded nil)
                        (equal (append packet nil)
                               (append decoded nil))))"#,
        ),
        "OK ((0 1 65 127 128 255) (0 1 65 127 128 255) t)"
    );
}

#[test]
fn decode_euc_attaches_charset_property() {
    crate::test_utils::init_test_tracing();
    let jis = register_test_jis();
    // EUC profile: G1 = test-jis, decoded from GR (high-bit) bytes.
    let spec = crate::emacs_core::coding::Iso2022Spec {
        initial: [
            Some(crate::emacs_core::intern::intern("ascii")),
            Some(jis),
            None,
            None,
        ],
        request: vec![],
        reg_usage: (0, 1),
        flags: enumflags2::BitFlags::empty(),
    };
    // ASCII 'A' then one GR character (0xA4 0xB3 -> GL 0x24 0x33).
    let (bytes, runs) = decode_via_euc(b"A\xA4\xB3", &spec);
    let text = crate::emacs_core::emacs_char::to_utf8_lossy(&bytes);
    assert_eq!(text.chars().count(), 2);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].start, 1);
    assert_eq!(runs[0].end, 2);
    assert_eq!(runs[0].plist, charset_prop("test-jis"));
}

#[test]
fn decode_sjis_attaches_charset_property() {
    crate::test_utils::init_test_tracing();
    use crate::emacs_core::intern::intern;
    // Shift-JIS charset list is (ascii katakana-jisx0201 <kanji>); register a
    // deterministic dim-1 katakana charset and a dim-2 kanji charset.
    let mut kana_args = vec![Value::NIL; 17];
    kana_args[0] = Value::symbol("test-katakana");
    kana_args[1] = Value::fixnum(1);
    kana_args[2] = Value::vector(vec![Value::fixnum(33), Value::fixnum(126)]);
    kana_args[11] = Value::fixnum(0x20000);
    crate::emacs_core::charset::builtin_define_charset_internal(kana_args).unwrap();
    let jis = register_test_jis();
    let charsets = vec![intern("ascii"), intern("test-katakana"), jis];
    // 'A' + half-width katakana 0xA1 (GL 0x21) + a 2-byte kanji.
    // Pick SJIS bytes that map to a valid kanji code in test-jis.
    let kanji_sjis = jis_to_sjis(0x2433);
    let mut input = vec![b'A', 0xA1, kanji_sjis.0, kanji_sjis.1];
    let (bytes, runs) = decode_via_sjis(&input, &charsets);
    let text = crate::emacs_core::emacs_char::to_utf8_lossy(&bytes);
    assert_eq!(text.chars().count(), 3); // A + katakana + kanji
    // Two runs: katakana at [1,2), kanji at [2,3).
    assert_eq!(runs.len(), 2);
    assert_eq!((runs[0].start, runs[0].end), (1, 2));
    assert_eq!(runs[0].plist, charset_prop("test-katakana"));
    assert_eq!((runs[1].start, runs[1].end), (2, 3));
    assert_eq!(runs[1].plist, charset_prop("test-jis"));
    let _ = &mut input;
}

#[test]
fn decode_sjis_charset_runs_absorb_ascii_like_gnu() {
    crate::test_utils::init_test_tracing();
    use crate::emacs_core::intern::intern;
    // GNU's `decode_coding_sjis` only moves `last_id`/`last_offset` for a
    // non-ASCII charset, so ASCII characters never close the run being
    // accumulated.  A run ends only where the next *different* non-ASCII
    // charset starts, or at the end of the decoded text - trailing ASCII
    // included.  Real GNU Emacs 31 decoding the equivalent Shift-JIS bytes
    // produces exactly three runs:
    //
    //   #("ABあCDｱｲEFいGH" 2 5 (charset japanese-jisx0208)
    //                      5 9 (charset katakana-jisx0201)
    //                      9 12 (charset japanese-jisx0208))
    let mut kana_args = vec![Value::NIL; 17];
    kana_args[0] = Value::symbol("test-katakana");
    kana_args[1] = Value::fixnum(1);
    kana_args[2] = Value::vector(vec![Value::fixnum(33), Value::fixnum(126)]);
    kana_args[11] = Value::fixnum(0x20000);
    crate::emacs_core::charset::builtin_define_charset_internal(kana_args).unwrap();
    let jis = register_test_jis();
    let charsets = vec![intern("ascii"), intern("test-katakana"), jis];

    let kanji_one = jis_to_sjis(0x2433);
    let kanji_two = jis_to_sjis(0x2434);
    let input = vec![
        b'A',
        b'B',
        kanji_one.0,
        kanji_one.1,
        b'C',
        b'D',
        0xA1,
        0xA2,
        b'E',
        b'F',
        kanji_two.0,
        kanji_two.1,
        b'G',
        b'H',
    ];
    let (bytes, runs) = decode_via_sjis(&input, &charsets);
    let text = crate::emacs_core::emacs_char::to_utf8_lossy(&bytes);
    assert_eq!(text.chars().count(), 12);

    assert_eq!(runs.len(), 3);
    // The first kanji run swallows the following "CD" and stops where the
    // katakana charset takes over.
    assert_eq!((runs[0].start, runs[0].end), (2, 5));
    assert_eq!(runs[0].plist, charset_prop("test-jis"));
    // The katakana run swallows the following "EF".
    assert_eq!((runs[1].start, runs[1].end), (5, 9));
    assert_eq!(runs[1].plist, charset_prop("test-katakana"));
    // The final kanji run reaches the end of the text, trailing "GH" included.
    assert_eq!((runs[2].start, runs[2].end), (9, 12));
    assert_eq!(runs[2].plist, charset_prop("test-jis"));
}

// ---------------------------------------------------------------------------
// fix8 GROUP=coding bugs (1c), (7), (8): encode/decode-coding-region routing,
// in-place point restoration, and the alias stored in last-coding-system-used.
// ---------------------------------------------------------------------------

fn fmt(eval: &mut crate::emacs_core::Context, src: &str) -> String {
    crate::emacs_core::format_eval_result(&eval.eval_str(src))
}

/// `Context::new()` is bare (no `with-temp-buffer`); seed the current buffer.
fn fmt_buf(eval: &mut crate::emacs_core::Context, seed: &str, expr: &str) -> String {
    let src = format!("(progn (erase-buffer) (insert {seed}) {expr})");
    crate::emacs_core::format_eval_result(&eval.eval_str(&src))
}

// Bug (1c): encode-coding-region for a charset/ISO-2022-type coding (cn-gb-2312,
// like euc-jp/shift_jis/iso-2022-jp) used to drop the non-ASCII text: the region
// path called `encode_lisp_string` directly, which only knows the UTF-8 /
// single-byte / Big5 families and silently dropped every CJK character (its
// `push_encoded` had an empty `_ => {}` arm). The region path now routes through
// the same context-aware codec the string functions use, so region == string.
// (euc-jp/shift_jis/chinese-gbk need external charset maps not loaded in the bare
// `Context::new()`; cn-gb-2312's table is built in, so it exercises the same
// dropped path here and the full set is re-verified on the release binary.)
#[test]
fn encode_coding_region_charset_coding_matches_string_path() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    // GNU: 一 -> two eight-bit bytes 0xD2 0xBB, buffer-size 2 (was 0/dropped).
    assert_eq!(
        fmt_buf(
            &mut eval,
            "(string ?\u{4e00})",
            "(progn (encode-coding-region (point-min) (point-max) 'cn-gb-2312-unix) (buffer-size))",
        ),
        "OK 2"
    );
    // The stored bytes (as eight-bit chars) equal the string encoder's bytes.
    assert_eq!(
        fmt_buf(
            &mut eval,
            "(string ?\u{4e00})",
            "(progn (encode-coding-region (point-min) (point-max) 'cn-gb-2312-unix)
                    (mapcar (lambda (c) (logand c #xFF)) (append (buffer-string) nil)))",
        ),
        "OK (210 187)"
    );
}

// Bug (1c) decode side: decode-coding-region for a charset-type coding must also
// route through the working decoder. It decodes the two GB2312 bytes back to the
// single character 一, with the same `(charset chinese-gb2312)` text property GNU
// attaches; the old region path garbled them into raw eight-bit characters.
#[test]
fn decode_coding_region_charset_coding_decodes_and_marks_charset() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    assert_eq!(
        crate::emacs_core::format_eval_result(&eval.eval_str(
            "(progn (set-buffer-multibyte nil) (insert #xd2 #xbb) (set-buffer-multibyte t)
                    (decode-coding-region (point-min) (point-max) 'cn-gb-2312-unix)
                    (buffer-string))",
        )),
        "OK #(\"\u{4e00}\" 0 1 (charset chinese-gb2312))"
    );
    // Round-trip: encoding 一 then decoding it gives 一 back.
    assert_eq!(
        fmt_buf(
            &mut eval,
            "(string ?\u{4e00})",
            "(progn (encode-coding-region (point-min) (point-max) 'cn-gb-2312-unix)
                    (decode-coding-region (point-min) (point-max) 'cn-gb-2312-unix)
                    (buffer-string))",
        ),
        "OK #(\"\u{4e00}\" 0 1 (charset chinese-gb2312))"
    );
}

// Bug (7): decode/encode-coding-region used to leave point at the region END.
// GNU restores point: before the region it is unchanged, interior point moves to
// the region START, point at/after the end shifts by the size delta.
#[test]
fn coding_region_restores_point_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    // Interior point (3) over the whole "abcd" region -> region START (1).
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcd\"",
            "(progn (goto-char 3) (decode-coding-region (point-min) (point-max) 'utf-8) (point))",
        ),
        "OK 1"
    );
    // Point at region start stays at start.
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcd\"",
            "(progn (goto-char 1) (decode-coding-region (point-min) (point-max) 'utf-8) (point))",
        ),
        "OK 1"
    );
    // Point at region end stays at end (size unchanged -> 5).
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcd\"",
            "(progn (goto-char 5) (decode-coding-region (point-min) (point-max) 'utf-8) (point))",
        ),
        "OK 5"
    );
    // encode side behaves the same.
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcd\"",
            "(progn (goto-char 3) (encode-coding-region (point-min) (point-max) 'utf-8) (point))",
        ),
        "OK 1"
    );
}

// Bug (7), partial region: point before the region is unchanged; point after the
// region shifts by the produced-vs-original size delta (here 0).
#[test]
fn coding_region_restores_point_for_partial_region() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    // Point before the [3,5) region -> unchanged (1).
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcdef\"",
            "(progn (goto-char 1) (decode-coding-region 3 5 'utf-8) (point))",
        ),
        "OK 1"
    );
    // Point inside the region -> region START (3).
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcdef\"",
            "(progn (goto-char 4) (decode-coding-region 3 5 'utf-8) (point))",
        ),
        "OK 3"
    );
    // Point after the region -> unchanged (6) since the region size is preserved.
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"abcdef\"",
            "(progn (goto-char 6) (decode-coding-region 3 5 'utf-8) (point))",
        ),
        "OK 6"
    );
}

// Bug (8): last-coding-system-used must store the coding system AS PASSED (the
// alias), not its resolved base. utf-8 (its own base) is the control.
#[test]
fn last_coding_system_used_stores_passed_alias() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::Context::new();
    // chinese-gbk's base is chinese-gbk itself in the bare manager; use an alias
    // that resolves to a different base to prove we keep the alias.  cp936 is an
    // alias of chinese-gbk; GNU stores `cp936`, not `chinese-gbk`.
    assert_eq!(
        fmt(
            &mut eval,
            "(progn (encode-coding-string \"x\" 'cp936) last-coding-system-used)",
        ),
        "OK cp936"
    );
    // Control: utf-8 stores utf-8 (alias == base, always agreed).
    assert_eq!(
        fmt(
            &mut eval,
            "(progn (encode-coding-string \"x\" 'utf-8) last-coding-system-used)",
        ),
        "OK utf-8"
    );
    // The region path stores the alias too.
    assert_eq!(
        fmt_buf(
            &mut eval,
            "\"x\"",
            "(progn (encode-coding-region (point-min) (point-max) 'cp936) last-coding-system-used)",
        ),
        "OK cp936"
    );
}

/// EOL conversion belongs to encoding itself, not to individual encoders.
///
/// GNU never lets an encoder decide this.  `consume_chars' (src/coding.c:7607)
/// expands the newline while filling the character buffer that EVERY encoder
/// then reads; the eol block is src/coding.c:7683:
///
/// ```c
/// if (! EQ (eol_type, Qunix))
///   { if (c == '\n') { if (EQ (eol_type, Qdos)) *buf++ = '\r'; else c = '\r'; } }
/// ```
///
/// so `encode_coding_utf_8', `encode_coding_utf_16', `encode_coding_iso_2022'
/// and the rest never see a bare newline and cannot skip the conversion.  The
/// headroom comment just above ("Compensate for CRLF and conversion",
/// src/coding.c:7646) exists because of this expansion.
///
/// Ours used to be the inverse: an opt-in `encode_eol_bytes'/`encode_eol_text'
/// call that some encode paths made and others did not.  Measured against GNU
/// 31.0.90, the UTF-16, UTF-8-signature, UTF-7, HZ, emacs-mule, ISO-2022 and
/// elisp pre-write-conversion (vietnamese-viqr) paths wrote LF where GNU writes
/// CR LF (or CR), while the UTF-8, Latin-1, raw-text, EUC, Shift_JIS and
/// charset-list paths were already right.
///
/// Every row is asserted together on purpose.  The rows that were wrong prove
/// the fix; the rows that were already right are what stops the fix from
/// over-applying and emitting CR CR LF.  Expected values are GNU 31.0.90's
/// output for the same input, taken by running the probe rather than derived.
#[test]
fn eol_conversion_applies_to_every_encoder_not_just_some() {
    crate::test_utils::init_test_tracing();
    let result = crate::test_utils::runtime_startup_eval_one(
        r#"(mapcar (lambda (cs) (list cs (string-to-list (encode-coding-string "a\nb" cs))))
                  '(utf-8-unix
                    utf-8-dos
                    utf-8-mac
                    latin-1-dos
                    latin-1-mac
                    raw-text-dos
                    raw-text-mac
                    utf-8-with-signature-unix
                    utf-8-with-signature-dos
                    utf-8-with-signature-mac
                    utf-8-auto-dos
                    utf-16le-dos
                    utf-16le-mac
                    utf-16be-dos
                    utf-16-dos
                    utf-7-dos
                    utf-7-imap-dos
                    chinese-hz-dos
                    emacs-mule-dos
                    iso-2022-jp-dos
                    iso-2022-7bit-dos
                    japanese-iso-8bit-dos
                    korean-iso-8bit-dos
                    chinese-iso-8bit-dos
                    japanese-shift-jis-dos
                    shift_jis-dos
                    in-is13194-devanagari-dos
                    chinese-big5-dos
                    vietnamese-viqr-unix
                    vietnamese-viqr-dos
                    vietnamese-viqr-mac))"#,
    );
    assert_eq!(
        result,
        "OK ((utf-8-unix (97 10 98)) \
             (utf-8-dos (97 13 10 98)) \
             (utf-8-mac (97 13 98)) \
             (latin-1-dos (97 13 10 98)) \
             (latin-1-mac (97 13 98)) \
             (raw-text-dos (97 13 10 98)) \
             (raw-text-mac (97 13 98)) \
             (utf-8-with-signature-unix (239 187 191 97 10 98)) \
             (utf-8-with-signature-dos (239 187 191 97 13 10 98)) \
             (utf-8-with-signature-mac (239 187 191 97 13 98)) \
             (utf-8-auto-dos (239 187 191 97 13 10 98)) \
             (utf-16le-dos (97 0 13 0 10 0 98 0)) \
             (utf-16le-mac (97 0 13 0 98 0)) \
             (utf-16be-dos (0 97 0 13 0 10 0 98)) \
             (utf-16-dos (254 255 0 97 0 13 0 10 0 98)) \
             (utf-7-dos (97 13 10 98)) \
             (utf-7-imap-dos (97 13 10 98)) \
             (chinese-hz-dos (97 13 10 98)) \
             (emacs-mule-dos (97 13 10 98)) \
             (iso-2022-jp-dos (97 13 10 98)) \
             (iso-2022-7bit-dos (97 13 10 98)) \
             (japanese-iso-8bit-dos (97 13 10 98)) \
             (korean-iso-8bit-dos (97 13 10 98)) \
             (chinese-iso-8bit-dos (97 13 10 98)) \
             (japanese-shift-jis-dos (97 13 10 98)) \
             (shift_jis-dos (97 13 10 98)) \
             (in-is13194-devanagari-dos (97 13 10 98)) \
             (chinese-big5-dos (97 13 10 98)) \
             (vietnamese-viqr-unix (97 10 98)) \
             (vietnamese-viqr-dos (97 13 10 98)) \
             (vietnamese-viqr-mac (97 13 98)))",
        "every encoder must see an already-converted newline; the unix row and \
         the already-correct dos/mac rows guard against double conversion"
    );
}
