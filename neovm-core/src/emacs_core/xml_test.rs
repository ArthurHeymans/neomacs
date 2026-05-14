use super::*;
use crate::emacs_core::value::ValueKind;

fn make_ctx() -> super::super::eval::Context {
    super::super::eval::Context::new()
}

#[test]
fn libxml_parse_xml_region_arity_and_nil_returns() {
    crate::test_utils::init_test_tracing();
    let mut ctx = make_ctx();

    // No args, no buffer → nil
    assert_eq!(
        builtin_libxml_parse_xml_region(&mut ctx, vec![]).unwrap(),
        Value::NIL
    );
    assert_eq!(
        builtin_libxml_parse_xml_region(&mut ctx, vec![Value::NIL]).unwrap(),
        Value::NIL
    );

    // Too many args → error
    let wrong_arity = builtin_libxml_parse_xml_region(
        &mut ctx,
        vec![
            Value::fixnum(1),
            Value::fixnum(1),
            Value::NIL,
            Value::NIL,
            Value::NIL,
        ],
    )
    .unwrap_err();
    match wrong_arity {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(
                sig.data,
                vec![Value::symbol("libxml-parse-xml-region"), Value::fixnum(5)]
            );
        }
        other => panic!("unexpected flow: {other:?}"),
    }

    // Wrong type for START → error
    let wrong_type =
        builtin_libxml_parse_xml_region(&mut ctx, vec![Value::string("x"), Value::fixnum(1)])
            .unwrap_err();
    match wrong_type {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("integer-or-marker-p"), Value::string("x")]
            );
        }
        other => panic!("unexpected flow: {other:?}"),
    }

    // Wrong type for BASE-URL → error (validated before buffer access)
    let wrong_base = builtin_libxml_parse_xml_region(
        &mut ctx,
        vec![Value::NIL, Value::NIL, Value::fixnum(42)],
    )
    .unwrap_err();
    match wrong_base {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(sig.data, vec![Value::symbol("stringp"), Value::fixnum(42)]);
        }
        other => panic!("unexpected flow: {other:?}"),
    }
}

#[test]
fn libxml_parse_html_region_arity_and_nil_returns() {
    crate::test_utils::init_test_tracing();
    let mut ctx = make_ctx();

    // No args, no buffer → nil
    assert_eq!(
        builtin_libxml_parse_html_region(&mut ctx, vec![]).unwrap(),
        Value::NIL
    );
    assert_eq!(
        builtin_libxml_parse_html_region(&mut ctx, vec![Value::NIL]).unwrap(),
        Value::NIL
    );

    // Too many args → error
    let wrong_arity = builtin_libxml_parse_html_region(
        &mut ctx,
        vec![
            Value::fixnum(1),
            Value::fixnum(1),
            Value::NIL,
            Value::NIL,
            Value::NIL,
        ],
    )
    .unwrap_err();
    match wrong_arity {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(
                sig.data,
                vec![Value::symbol("libxml-parse-html-region"), Value::fixnum(5)]
            );
        }
        other => panic!("unexpected flow: {other:?}"),
    }

    // Wrong type for START → error
    let wrong_type =
        builtin_libxml_parse_html_region(&mut ctx, vec![Value::string("x"), Value::fixnum(1)])
            .unwrap_err();
    match wrong_type {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("integer-or-marker-p"), Value::string("x")]
            );
        }
        other => panic!("unexpected flow: {other:?}"),
    }

    // Wrong type for BASE-URL → error (validated before buffer access)
    let wrong_base = builtin_libxml_parse_html_region(
        &mut ctx,
        vec![Value::NIL, Value::NIL, Value::fixnum(42)],
    )
    .unwrap_err();
    match wrong_base {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(sig.data, vec![Value::symbol("stringp"), Value::fixnum(42)]);
        }
        other => panic!("unexpected flow: {other:?}"),
    }
}

#[test]
fn libxml_available_p_returns_true_and_validates_arity() {
    crate::test_utils::init_test_tracing();
    assert_eq!(builtin_libxml_available_p(vec![]).unwrap(), Value::T);

    let libxml_arity = builtin_libxml_available_p(vec![Value::fixnum(1)]).unwrap_err();
    match libxml_arity {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(
                sig.data,
                vec![Value::symbol("libxml-available-p"), Value::fixnum(1)]
            );
        }
        other => panic!("unexpected flow: {other:?}"),
    }
}

#[test]
fn zlib_available_p_returns_true() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        crate::emacs_core::zlib::builtin_zlib_available_p(vec![]).unwrap(),
        Value::T
    );
    let zlib_arity =
        crate::emacs_core::zlib::builtin_zlib_available_p(vec![Value::fixnum(1)]).unwrap_err();
    match zlib_arity {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "wrong-number-of-arguments");
            assert_eq!(
                sig.data,
                vec![Value::symbol("zlib-available-p"), Value::fixnum(1)]
            );
        }
        other => panic!("unexpected flow: {other:?}"),
    }
}
