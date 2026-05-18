use super::*;
use crate::heap_types::LispString;

#[test]
fn substring_preserves_raw_unibyte_storage_semantics() {
    crate::test_utils::init_test_tracing();

    let source = Value::heap_string(LispString::from_unibyte(vec![0xff, b'a', b'b']));
    let result = builtin_substring(vec![source, Value::fixnum(1), Value::fixnum(3)])
        .expect("substring should accept raw unibyte storage");
    let string = result
        .as_lisp_string()
        .expect("substring should return a string");

    assert!(!string.is_multibyte());
    assert_eq!(string.as_bytes(), b"ab");
}

#[test]
fn substring_can_return_raw_non_utf8_unibyte_bytes() {
    crate::test_utils::init_test_tracing();

    let source = Value::heap_string(LispString::from_unibyte(vec![0xff, 0xfe, b'x']));
    let result = builtin_substring(vec![source, Value::fixnum(0), Value::fixnum(2)])
        .expect("substring should slice raw unibyte bytes");
    let string = result
        .as_lisp_string()
        .expect("substring should return a string");

    assert!(!string.is_multibyte());
    assert_eq!(string.as_bytes(), &[0xff, 0xfe]);
}

#[test]
fn substring_copies_text_properties_through_gnu_add_properties_order() {
    crate::test_utils::init_test_tracing();

    let mut eval = crate::emacs_core::eval::Context::new();
    let result = eval
        .eval_str(
            r#"(let* ((s (propertize "abcdef" 'face 'bold 'tag 'source))
                      (sub (substring s 1 5)))
                 (text-properties-at 0 sub))"#,
        )
        .expect("evaluation succeeds");

    assert_eq!(
        crate::emacs_core::print::print_value(&result),
        "(tag source face bold)"
    );
}

#[test]
fn concat_preserves_multibyte_text_properties_as_char_intervals() {
    crate::test_utils::init_test_tracing();

    let source = Value::string("é");
    let mut table = crate::buffer::text_props::TextPropertyTable::new();
    table.put_property(0, 1, Value::symbol("face"), Value::symbol("bold"));
    crate::emacs_core::value::set_string_text_properties_table_for_value(source, table);

    let result = builtin_concat(vec![Value::string("x"), source, Value::string("z")])
        .expect("concat should preserve string properties");
    let props = crate::emacs_core::value::get_string_text_properties_table_for_value(result)
        .expect("result should carry text properties");
    let intervals = props.intervals_snapshot();

    assert_eq!(intervals.len(), 1);
    assert_eq!((intervals[0].start, intervals[0].end), (1, 2));
    assert_eq!(
        intervals[0].properties.get(&Value::symbol("face")),
        Some(&Value::symbol("bold"))
    );
}

#[test]
fn concat_promotes_unibyte_high_bytes_when_result_is_multibyte_like_gnu() {
    crate::test_utils::init_test_tracing();

    let raw = Value::heap_string(LispString::from_unibyte(vec![0xff]));
    let result = builtin_concat(vec![Value::string("é"), raw])
        .expect("concat should promote raw unibyte bytes");
    let string = result.as_lisp_string().expect("concat returns a string");

    assert!(string.is_multibyte());
    assert_eq!(string.schars(), 2);
    assert_eq!(string.sbytes(), 4);
    assert_eq!(string.as_bytes(), &[0xc3, 0xa9, 0xc1, 0xbf]);
}

#[test]
fn concat_all_unibyte_strings_with_properties_stays_unibyte_like_gnu() {
    crate::test_utils::init_test_tracing();

    let raw = Value::heap_string(LispString::from_unibyte(vec![0xff]));
    let suffix = Value::heap_string(LispString::from_unibyte(vec![b'a']));
    let mut table = crate::buffer::text_props::TextPropertyTable::new();
    table.put_property(0, 1, Value::symbol("face"), Value::symbol("bold"));
    crate::emacs_core::value::set_string_text_properties_table_for_value(raw, table);

    let result = builtin_concat(vec![raw, suffix]).expect("concat should preserve unibyte storage");
    let string = result.as_lisp_string().expect("concat returns a string");
    let props = crate::emacs_core::value::get_string_text_properties_table_for_value(result)
        .expect("result should carry text properties");
    let intervals = props.intervals_snapshot();

    assert!(!string.is_multibyte());
    assert_eq!(string.as_bytes(), &[0xff, b'a']);
    assert_eq!(intervals.len(), 1);
    assert_eq!((intervals[0].start, intervals[0].end), (0, 1));
    assert_eq!(
        intervals[0].properties.get(&Value::symbol("face")),
        Some(&Value::symbol("bold"))
    );
}

#[test]
fn format_preserves_multibyte_text_properties_as_char_intervals() {
    crate::test_utils::init_test_tracing();

    let source = Value::string("éz");
    let mut table = crate::buffer::text_props::TextPropertyTable::new();
    table.put_property(0, 1, Value::symbol("face"), Value::symbol("bold"));
    crate::emacs_core::value::set_string_text_properties_table_for_value(source, table);

    let mut ctx = crate::emacs_core::eval::Context::new();
    let result = builtin_format_wrapper_strict_slice(&mut ctx, &[Value::string("%4s"), source])
        .expect("format should preserve string properties");
    let props = crate::emacs_core::value::get_string_text_properties_table_for_value(result)
        .expect("result should carry text properties");
    let intervals = props.intervals_snapshot();

    assert_eq!(result.as_utf8_str(), Some("  éz"));
    assert_eq!(intervals.len(), 1);
    assert_eq!((intervals[0].start, intervals[0].end), (2, 3));
    assert_eq!(
        intervals[0].properties.get(&Value::symbol("face")),
        Some(&Value::symbol("bold"))
    );
}

#[test]
fn format_preserves_percent_s_text_property_plist_order() {
    crate::test_utils::init_test_tracing();

    let source = Value::string("key");
    let mut table = crate::buffer::text_props::TextPropertyTable::new();
    table.put_property(
        0,
        3,
        Value::symbol("face"),
        Value::symbol("help-key-binding"),
    );
    table.put_property(
        0,
        3,
        Value::symbol("font-lock-face"),
        Value::symbol("help-key-binding"),
    );
    crate::emacs_core::value::set_string_text_properties_table_for_value(source, table);

    let mut ctx = crate::emacs_core::eval::Context::new();
    let result = builtin_format_wrapper_strict_slice(&mut ctx, &[Value::string("%s ok"), source])
        .expect("format should preserve string properties");
    let props = crate::emacs_core::value::get_string_text_properties_table_for_value(result)
        .expect("result should carry text properties");
    let intervals = props.intervals_snapshot();
    let ordered_keys: Vec<_> = intervals[0]
        .ordered_properties()
        .map(|(name, _)| name.as_symbol_name().unwrap().to_string())
        .collect();

    assert_eq!(result.as_utf8_str(), Some("key ok"));
    assert_eq!(
        ordered_keys,
        vec!["font-lock-face".to_string(), "face".to_string()]
    );
}

#[test]
fn format_percent_s_promotes_result_when_printer_outputs_non_ascii_text() {
    crate::test_utils::init_test_tracing();

    let mut ctx = crate::emacs_core::eval::Context::new();
    let result = builtin_format_wrapper_strict_slice(
        &mut ctx,
        &[
            Value::string("%S"),
            Value::list(vec![
                Value::string("é"),
                Value::fixnum(233),
                Value::string("é"),
            ]),
        ],
    )
    .expect("format should evaluate");
    let string = result.as_lisp_string().expect("format returns a string");

    assert!(string.is_multibyte());
    assert_eq!(result.as_utf8_str(), Some("(\"é\" 233 \"é\")"));
    assert_eq!(string.as_bytes(), b"(\"\xc3\xa9\" 233 \"\xc3\xa9\")");
}

#[test]
fn format_percent_c_matches_gnu_non_ascii_multibyte_width_and_precision() {
    crate::test_utils::init_test_tracing();

    fn run_format(fmt: &str, arg: Value) -> LispString {
        let mut ctx = crate::emacs_core::eval::Context::new();
        let result = builtin_format_wrapper_strict_slice(&mut ctx, &[Value::string(fmt), arg])
            .expect("format should evaluate");
        result
            .as_lisp_string()
            .expect("format should return a string")
            .clone()
    }

    fn codes(string: &LispString) -> Vec<u32> {
        if !string.is_multibyte() {
            return string.as_bytes().iter().map(|byte| *byte as u32).collect();
        }
        let mut out = Vec::new();
        let mut pos = 0usize;
        while pos < string.as_bytes().len() {
            let (code, len) = crate::emacs_core::emacs_char::string_char(&string.as_bytes()[pos..]);
            out.push(code);
            pos += len;
        }
        out
    }

    let ascii = run_format("%05c", Value::fixnum(b'x' as i64));
    assert!(!ascii.is_multibyte());
    assert_eq!(ascii.as_bytes(), b"    x");

    let latin1 = run_format("%c", Value::fixnum(0x80));
    assert!(latin1.is_multibyte());
    assert_eq!(latin1.as_bytes(), &[0xc2, 0x80]);
    assert_eq!(codes(&latin1), vec![0x80]);

    let nonunicode = run_format("%2c", Value::fixnum(0x11_0000));
    assert!(nonunicode.is_multibyte());
    assert_eq!(nonunicode.as_bytes(), &[b' ', 0xf4, 0x90, 0x80, 0x80]);
    assert_eq!(codes(&nonunicode), vec![b' ' as u32, 0x11_0000]);

    let empty_nonunicode = run_format("%.0c", Value::fixnum(0x11_0000));
    assert!(empty_nonunicode.is_multibyte());
    assert!(empty_nonunicode.as_bytes().is_empty());

    let padded_empty_nonunicode = run_format("%2.0c", Value::fixnum(0x11_0000));
    assert!(padded_empty_nonunicode.is_multibyte());
    assert_eq!(padded_empty_nonunicode.as_bytes(), b"  ");
}

#[test]
fn format_string_width_and_precision_use_gnu_display_width() {
    crate::test_utils::init_test_tracing();

    let mut ctx = crate::emacs_core::eval::Context::new();
    let wide_padded =
        builtin_format_wrapper_strict_slice(&mut ctx, &[Value::string("%3s"), Value::string("界")])
            .expect("format should evaluate");
    assert_eq!(wide_padded.as_utf8_str(), Some(" 界"));

    let wide_truncated = builtin_format_wrapper_strict_slice(
        &mut ctx,
        &[Value::string("%.1s"), Value::string("界")],
    )
    .expect("format should evaluate");
    let wide_truncated = wide_truncated
        .as_lisp_string()
        .expect("format should return a string");
    assert!(wide_truncated.is_multibyte());
    assert!(wide_truncated.as_bytes().is_empty());
}

#[test]
fn format_percent_g_uses_gnu_fixed_precision_for_negative_exponents() {
    crate::test_utils::init_test_tracing();

    let mut ctx = crate::emacs_core::eval::Context::new();
    let result = builtin_format_wrapper_strict_slice(
        &mut ctx,
        &[
            Value::string("%.2g %.2g %.2g"),
            Value::make_float(0.00042),
            Value::make_float(0.0042),
            Value::make_float(42.0),
        ],
    )
    .expect("format should evaluate");

    assert_eq!(result.as_utf8_str(), Some("0.00042 0.0042 42"));
}
