use super::*;
use crate::emacs_core::eval::Context;
use crate::emacs_core::value::{ValueKind, VecLikeType};
use crate::emacs_core::{print, string_escape};

/// Test helper: create a minimal eval context for widget-apply tests.
fn test_eval_ctx() -> crate::emacs_core::eval::Context {
    crate::emacs_core::eval::Context::new()
}

/// Test helper that calls an evaluator builtin and keeps the
/// context alive for the remainder of the test. Previously the
/// `call_fns_builtin!` macro created a short-lived `Context::new()`
/// inside its block expression and returned the builtin's result;
/// the context was then dropped at the end of the expression,
/// destroying the tagged heap and leaving the returned Value
/// pointing at freed memory. `.as_str()` on the stale Value hit
/// `BUG: StringObj header.kind = VecLike` from `tagged/value.rs`.
///
/// Each call to this helper allocates a boxed `Context` in a
/// thread-local so the returned Value's heap memory lives until
/// the test function returns.
macro_rules! call_fns_builtin {
    ($builtin:ident, $args:expr) => {{
        use std::cell::RefCell;
        thread_local! {
            static TEST_CTX: RefCell<Option<Box<Context>>> = const { RefCell::new(None) };
        }
        TEST_CTX.with(|slot| {
            let mut new_ctx = Box::new(Context::new());
            let result = $builtin(&mut new_ctx, $args);
            // Replace any prior held context (previous test calls
            // in the same thread) — the new one owns the heap
            // that holds the returned Value.
            *slot.borrow_mut() = Some(new_ctx);
            result
        })
    }};
}

// ---- Base64 standard ----

#[test]
fn base64_encode_empty() {
    crate::test_utils::init_test_tracing();
    let r = builtin_base64_encode_string(vec![Value::string(""), Value::T]).unwrap();
    assert_eq!(r.as_utf8_str(), Some(""));
}

#[test]
fn base64_encode_hello() {
    crate::test_utils::init_test_tracing();
    let r = builtin_base64_encode_string(vec![Value::string("Hello"), Value::T]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("SGVsbG8="));
}

#[test]
fn base64_encode_padding_1() {
    crate::test_utils::init_test_tracing();
    // "a" -> "YQ=="
    let r = builtin_base64_encode_string(vec![Value::string("a"), Value::T]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("YQ=="));
}

#[test]
fn base64_encode_padding_2() {
    crate::test_utils::init_test_tracing();
    // "ab" -> "YWI="
    let r = builtin_base64_encode_string(vec![Value::string("ab"), Value::T]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("YWI="));
}

#[test]
fn base64_encode_no_padding_3() {
    crate::test_utils::init_test_tracing();
    // "abc" -> "YWJj" (no padding needed)
    let r = builtin_base64_encode_string(vec![Value::string("abc"), Value::T]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("YWJj"));
}

#[test]
fn base64_roundtrip() {
    crate::test_utils::init_test_tracing();
    let original = "The quick brown fox jumps over the lazy dog";
    let encoded = builtin_base64_encode_string(vec![Value::string(original), Value::T]).unwrap();
    let decoded = builtin_base64_decode_string(vec![encoded]).unwrap();
    assert_eq!(decoded.as_utf8_str(), Some(original));
}

#[test]
fn base64_decode_invalid() {
    crate::test_utils::init_test_tracing();
    // Invalid base64 now signals an error (matching GNU Emacs)
    let r = builtin_base64_decode_string(vec![Value::string("!!!!")]);
    assert!(r.is_err());
}

#[test]
fn base64_decode_string_ignore_invalid() {
    crate::test_utils::init_test_tracing();
    let decoded =
        builtin_base64_decode_string(vec![Value::string("!!!!"), Value::NIL, Value::T]).unwrap();
    let decoded = decoded.as_lisp_string().unwrap();
    assert_eq!(decoded.as_bytes(), b"");
    assert!(!decoded.is_multibyte());
}

#[test]
fn base64_decode_string_rejects_malformed_padding_like_gnu() {
    crate::test_utils::init_test_tracing();
    for input in ["Zg=", "Zm9vYmE", "Zm9vYmFy=", "Zg=Zg="] {
        let decoded = builtin_base64_decode_string(vec![Value::string(input)]);
        assert!(decoded.is_err(), "{input} should signal invalid base64");
    }
}

#[test]
fn base64_encode_string_rejects_multibyte_non_ascii_like_gnu() {
    crate::test_utils::init_test_tracing();
    let encoded = builtin_base64_encode_string(vec![Value::string("é"), Value::T]);
    match encoded {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data,
                vec![Value::string(
                    "Multibyte character in data for base64 encoding"
                )]
            );
        }
        other => panic!("expected multibyte base64 error, got {other:?}"),
    }
}

#[test]
fn base64_encode_string_preserves_unibyte_raw_bytes_like_gnu() {
    crate::test_utils::init_test_tracing();
    let input = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xE9]));
    let encoded = builtin_base64_encode_string(vec![input, Value::T]).unwrap();
    assert_eq!(encoded.as_utf8_str(), Some("6Q=="));
}

// ---- Base64 URL ----

#[test]
fn base64url_encode_no_pad() {
    crate::test_utils::init_test_tracing();
    let r = builtin_base64url_encode_string(vec![Value::string("a"), Value::T]).unwrap();
    // URL-safe, no padding
    assert_eq!(r.as_utf8_str(), Some("YQ"));
}

#[test]
fn base64url_encode_with_pad() {
    crate::test_utils::init_test_tracing();
    let r = builtin_base64url_encode_string(vec![Value::string("a")]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("YQ=="));
}

#[test]
fn base64url_roundtrip() {
    crate::test_utils::init_test_tracing();
    let original = "Hello+World/Foo";
    let encoded = builtin_base64url_encode_string(vec![Value::string(original), Value::T]).unwrap();
    let decoded = builtin_base64_decode_string(vec![encoded, Value::T]).unwrap();
    assert_eq!(decoded.as_utf8_str(), Some(original));
}

#[test]
fn base64url_decode_basic() {
    crate::test_utils::init_test_tracing();
    let decoded = builtin_base64url_decode_string(vec![Value::string("YQ")]).unwrap();
    assert_eq!(decoded.as_utf8_str(), Some("a"));
}

#[test]
fn base64url_decode_invalid() {
    crate::test_utils::init_test_tracing();
    let decoded = builtin_base64url_decode_string(vec![Value::string("!!!!")]).unwrap();
    assert!(decoded.is_nil());
}

#[test]
fn base64url_decode_ignore_invalid() {
    crate::test_utils::init_test_tracing();
    let decoded = builtin_base64url_decode_string(vec![Value::string("!!!!"), Value::T]).unwrap();
    let decoded = decoded.as_lisp_string().unwrap();
    assert_eq!(decoded.as_bytes(), b"");
    assert!(!decoded.is_multibyte());
}

#[test]
fn base64url_uses_dash_underscore() {
    crate::test_utils::init_test_tracing();
    let input = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xFF]));
    let std_enc = builtin_base64_encode_string(vec![input, Value::T]).unwrap();
    let input = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xFF]));
    let url_enc = builtin_base64url_encode_string(vec![input, Value::NIL]).unwrap();
    assert_eq!(std_enc.as_utf8_str(), Some("/w=="));
    assert_eq!(url_enc.as_utf8_str(), Some("_w=="));
}

#[test]
fn base64url_encode_string_rejects_multibyte_non_ascii_like_gnu() {
    crate::test_utils::init_test_tracing();
    let encoded = builtin_base64url_encode_string(vec![Value::string("é"), Value::T]);
    match encoded {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data,
                vec![Value::string(
                    "Multibyte character in data for base64 encoding"
                )]
            );
        }
        other => panic!("expected multibyte base64 error, got {other:?}"),
    }
}

#[test]
fn base64_region_eval_encode_decode_roundtrip() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("Hi");
    }

    let encoded = builtin_base64_encode_region(&mut eval, vec![Value::fixnum(1), Value::fixnum(3)])
        .expect("encode region should succeed");
    assert_eq!(encoded, Value::fixnum(4));
    let encoded_text = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_string();
    assert_eq!(encoded_text, "SGk=");

    let decoded = builtin_base64_decode_region(&mut eval, vec![Value::fixnum(1), Value::fixnum(5)])
        .expect("decode region should succeed");
    assert_eq!(decoded, Value::fixnum(2));
    let decoded_text = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_string();
    assert_eq!(decoded_text, "Hi");
}

#[test]
fn base64_region_eval_swapped_bounds_and_url_encoding() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("ab");
    }

    let encoded = builtin_base64url_encode_region(
        &mut eval,
        vec![Value::fixnum(3), Value::fixnum(1), Value::T],
    )
    .expect("url encode region should succeed");
    assert_eq!(encoded, Value::fixnum(3));
    let encoded_text = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_string();
    assert_eq!(encoded_text, "YWI");
}

#[test]
fn base64_region_preserves_unibyte_raw_bytes() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.set_multibyte_value(false);
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert_lisp_string(&crate::heap_types::LispString::from_unibyte(vec![0xFF]));
        buf.goto_emacs_byte_pos(crate::buffer::EmacsBytePos::new(0));
    }

    let encoded = builtin_base64_encode_region(&mut eval, vec![Value::fixnum(1), Value::fixnum(2)])
        .expect("encode raw-byte region should succeed");
    assert_eq!(encoded, Value::fixnum(4));
    let encoded_text = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_string();
    assert_eq!(encoded_text, "/w==");

    let decoded = builtin_base64_decode_region(&mut eval, vec![Value::fixnum(1), Value::fixnum(5)])
        .expect("decode raw-byte region should succeed");
    assert_eq!(decoded, Value::fixnum(1));
    let decoded_text = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_substring_lisp_string_range(crate::buffer::EmacsByteRange::from_usize(0, 1));
    assert_eq!(decoded_text.as_bytes(), &[0xFF]);
    assert!(!decoded_text.is_multibyte());
}

#[test]
fn base64_decode_region_noerror_semantics() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("%%");
    }

    let ignored = builtin_base64_decode_region(
        &mut eval,
        vec![Value::fixnum(1), Value::fixnum(3), Value::NIL, Value::T],
    )
    .expect("noerror decode should succeed");
    assert_eq!(ignored, Value::fixnum(0));
    let emptied = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_string();
    assert_eq!(emptied, "");

    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.insert("%%");
    }
    let strict = builtin_base64_decode_region(&mut eval, vec![Value::fixnum(1), Value::fixnum(3)]);
    match strict {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(sig.data, vec![Value::string("Invalid base64 data")]);
        }
        other => panic!("expected invalid base64 signal, got {other:?}"),
    }
    let unchanged = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .buffer_string();
    assert_eq!(unchanged, "%%");
}

#[test]
fn base64_region_eval_error_shapes() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("Hi");
    }

    let type_error = builtin_base64_encode_region(
        &mut eval,
        vec![Value::symbol("x"), Value::fixnum(2), Value::T],
    );
    match type_error {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data,
                vec![Value::symbol("integer-or-marker-p"), Value::symbol("x")]
            );
        }
        other => panic!("expected wrong-type-argument, got {other:?}"),
    }

    let range_error =
        builtin_base64_encode_region(&mut eval, vec![Value::fixnum(0), Value::fixnum(2)]);
    match range_error {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "args-out-of-range");
            assert_eq!(sig.data.len(), 3);
            assert!(sig.data[0].is_buffer());
            assert_eq!(sig.data[1], Value::fixnum(0));
            assert_eq!(sig.data[2], Value::fixnum(2));
        }
        other => panic!("expected args-out-of-range, got {other:?}"),
    }
}

// ---- MD5 ----

#[test]
fn md5_empty() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(builtin_md5, vec![Value::string("")]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("d41d8cd98f00b204e9800998ecf8427e"));
}

#[test]
fn md5_hello() {
    crate::test_utils::init_test_tracing();
    // md5("Hello") = 8b1a9953c4611296a827abf8c47804d7
    let r = call_fns_builtin!(builtin_md5, vec![Value::string("Hello")]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("8b1a9953c4611296a827abf8c47804d7"));
}

#[test]
fn md5_abc() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(builtin_md5, vec![Value::string("abc")]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("900150983cd24fb0d6963f7d28e17f72"));
}

#[test]
fn md5_fox() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_md5,
        vec![Value::string("The quick brown fox jumps over the lazy dog")]
    )
    .unwrap();
    assert_eq!(r.as_utf8_str(), Some("9e107d9d372bb6826bd81d3542a419d6"));
}

#[test]
fn md5_string_range_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_md5,
        vec![Value::string("abc"), Value::fixnum(2), Value::fixnum(1)]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "args-out-of-range");
            assert_eq!(
                sig.data,
                vec![Value::string("abc"), Value::fixnum(2), Value::fixnum(1)]
            );
        }
        other => panic!("expected args-out-of-range signal, got {other:?}"),
    }
}

#[test]
fn md5_string_index_type_error() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_md5,
        vec![Value::string("abc"), Value::T, Value::fixnum(1)]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(sig.data.first(), Some(&Value::symbol("integerp")));
        }
        other => panic!("expected wrong-type-argument signal, got {other:?}"),
    }
}

#[test]
fn md5_invalid_object_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(builtin_md5, vec![Value::NIL]) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data.first().and_then(|v| v.as_utf8_str()),
                Some("Invalid object argument")
            );
            assert_eq!(sig.data.get(1), Some(&Value::string("nil")));
        }
        other => panic!("expected error signal, got {other:?}"),
    }
}

#[test]
fn md5_unknown_coding_system_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_md5,
        vec![
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
            Value::symbol("no-such"),
        ]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "coding-system-error");
            assert_eq!(sig.data, vec![Value::symbol("no-such")]);
        }
        other => panic!("expected coding-system-error signal, got {other:?}"),
    }
}

#[test]
fn md5_unknown_coding_system_ignored_with_noerror() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_md5,
        vec![
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
            Value::symbol("no-such"),
            Value::T,
        ]
    )
    .unwrap();
    assert_eq!(r.as_utf8_str(), Some("900150983cd24fb0d6963f7d28e17f72"));
}

#[test]
fn md5_string_honors_utf16le_coding_system() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_md5,
        vec![
            Value::string("é"),
            Value::NIL,
            Value::NIL,
            Value::symbol("utf-16le"),
        ]
    )
    .unwrap();
    assert_eq!(r.as_utf8_str(), Some("ed71e8ffd3d8c47c1a2e22c53cd384aa"));
}

#[test]
fn md5_accepts_iso_8859_15_alias() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_md5,
        vec![
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
            Value::symbol("iso-8859-15"),
        ]
    )
    .unwrap();
    assert_eq!(r.as_utf8_str(), Some("900150983cd24fb0d6963f7d28e17f72"));
}

#[test]
fn md5_accepts_iso_8859_9_alias() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_md5,
        vec![
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
            Value::symbol("iso-8859-9"),
        ]
    )
    .unwrap();
    assert_eq!(r.as_utf8_str(), Some("900150983cd24fb0d6963f7d28e17f72"));
}

#[test]
fn md5_non_symbol_coding_system_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_md5,
        vec![
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
            Value::fixnum(1),
        ]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "coding-system-error");
            assert_eq!(sig.data, vec![Value::fixnum(1)]);
        }
        other => panic!("expected coding-system-error signal, got {other:?}"),
    }
}

#[test]
fn md5_eval_buffer_core_semantics() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("abc");
    }
    let id = eval.buffers.current_buffer().expect("current buffer").id;

    let full = builtin_md5(&mut eval, vec![Value::make_buffer(id)]).unwrap();
    assert_eq!(full.as_utf8_str(), Some("900150983cd24fb0d6963f7d28e17f72"));

    let swapped = builtin_md5(
        &mut eval,
        vec![Value::make_buffer(id), Value::fixnum(4), Value::fixnum(3)],
    )
    .unwrap();
    assert_eq!(
        swapped.as_utf8_str(),
        Some("4a8a08f09d37b73795649038408b5f33")
    );
}

#[test]
fn md5_eval_buffer_range_errors() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("abc");
    }
    let id = eval.buffers.current_buffer().expect("current buffer").id;

    match builtin_md5(&mut eval, vec![Value::make_buffer(id), Value::fixnum(5)]) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "args-out-of-range");
            assert_eq!(sig.data, vec![Value::fixnum(5), Value::NIL]);
        }
        other => panic!("expected args-out-of-range signal, got {other:?}"),
    }
}

#[test]
fn md5_eval_buffer_index_type_error() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let id = eval.buffers.current_buffer().expect("current buffer").id;

    match builtin_md5(
        &mut eval,
        vec![Value::make_buffer(id), Value::T, Value::fixnum(3)],
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data.first(),
                Some(&Value::symbol("integer-or-marker-p"))
            );
        }
        other => panic!("expected wrong-type-argument signal, got {other:?}"),
    }
}

#[test]
fn md5_eval_deleted_buffer_errors() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let id = eval.buffers.create_buffer("*md5-doomed*");
    assert!(eval.buffers.kill_buffer(id));

    match builtin_md5(&mut eval, vec![Value::make_buffer(id)]) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data.first().and_then(|v| v.as_utf8_str()),
                Some("Selecting deleted buffer")
            );
        }
        other => panic!("expected error signal, got {other:?}"),
    }
}

#[test]
fn md5_and_secure_hash_preserve_unibyte_raw_bytes() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let raw = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xFF]));
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.set_multibyte_value(false);
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert_lisp_string(&crate::heap_types::LispString::from_unibyte(vec![0xFF]));
    }
    let id = eval.buffers.current_buffer().expect("current buffer").id;

    let string_md5 = builtin_md5(&mut eval, vec![raw]).unwrap();
    assert_eq!(
        string_md5.as_utf8_str(),
        Some("00594fd4f42ba43fc1ca0427a0576295")
    );

    let buffer_md5 = builtin_md5(&mut eval, vec![Value::make_buffer(id)]).unwrap();
    assert_eq!(
        buffer_md5.as_utf8_str(),
        Some("00594fd4f42ba43fc1ca0427a0576295")
    );

    let string_sha1 = builtin_secure_hash(&mut eval, vec![Value::symbol("sha1"), raw]).unwrap();
    assert_eq!(
        string_sha1.as_utf8_str(),
        Some("85e53271e14006f0265921d02d4d736cdc580b0b")
    );

    let buffer_sha1 = builtin_secure_hash(
        &mut eval,
        vec![Value::symbol("sha1"), Value::make_buffer(id)],
    )
    .unwrap();
    assert_eq!(
        buffer_sha1.as_utf8_str(),
        Some("85e53271e14006f0265921d02d4d736cdc580b0b")
    );

    let buffer_hash = builtin_buffer_hash(&mut eval, vec![Value::make_buffer(id)]).unwrap();
    assert_eq!(
        buffer_hash.as_utf8_str(),
        Some("85e53271e14006f0265921d02d4d736cdc580b0b")
    );
}

// ---- secure-hash ----

#[test]
fn secure_hash_sha256_known() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_secure_hash,
        vec![Value::symbol("sha256"), Value::string("abc")]
    )
    .unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn secure_hash_sha1_known() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_secure_hash,
        vec![Value::symbol("sha1"), Value::string("abc")]
    )
    .unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("a9993e364706816aba3e25717850c26c9cd0d89d")
    );
}

#[test]
fn secure_hash_md5_known() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_secure_hash,
        vec![Value::symbol("md5"), Value::string("abc")]
    )
    .unwrap();
    assert_eq!(r.as_utf8_str(), Some("900150983cd24fb0d6963f7d28e17f72"));
}

#[test]
fn secure_hash_binary_string_uses_unibyte_storage() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_secure_hash,
        vec![
            Value::symbol("sha1"),
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
            Value::T,
        ]
    )
    .unwrap();

    let ls = r
        .as_lisp_string()
        .expect("binary secure-hash should return a string");
    assert_eq!(ls.sbytes(), 20);
    assert_eq!(ls.as_bytes().first(), Some(&169u8));

    let printed = print::print_value_bytes(&r);
    assert_eq!(printed.first(), Some(&b'"'));
    assert_eq!(printed.last(), Some(&b'"'));
}

#[test]
fn secure_hash_subrange_semantics() {
    crate::test_utils::init_test_tracing();
    let r = call_fns_builtin!(
        builtin_secure_hash,
        vec![
            Value::symbol("sha256"),
            Value::string("abcdef"),
            Value::fixnum(1),
            Value::fixnum(4),
        ]
    )
    .unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("a6b0f90d2ac2b8d1f250c687301aef132049e9016df936680e81fa7bc7d81d70")
    );
}

#[test]
fn secure_hash_invalid_algorithm_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_secure_hash,
        vec![Value::symbol("no-such"), Value::string("abc")]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data.first().and_then(|v| v.as_utf8_str()),
                Some("Invalid algorithm arg: no-such")
            );
        }
        other => panic!("expected error signal, got {other:?}"),
    }
}

#[test]
fn secure_hash_invalid_algorithm_type_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_secure_hash,
        vec![Value::fixnum(1), Value::string("abc")]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(sig.data.first(), Some(&Value::symbol("symbolp")));
        }
        other => panic!("expected wrong-type-argument signal, got {other:?}"),
    }
}

#[test]
fn secure_hash_invalid_object_errors() {
    crate::test_utils::init_test_tracing();
    match call_fns_builtin!(
        builtin_secure_hash,
        vec![Value::symbol("sha256"), Value::fixnum(123)]
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data.first().and_then(|v| v.as_utf8_str()),
                Some("Invalid object argument")
            );
            assert_eq!(sig.data.get(1), Some(&Value::fixnum(123)));
        }
        other => panic!("expected error signal, got {other:?}"),
    }
}

#[test]
fn secure_hash_eval_buffer_sha1() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("abc");
    }
    let id = eval.buffers.current_buffer().expect("current buffer").id;
    let r = builtin_secure_hash(
        &mut eval,
        vec![Value::symbol("sha1"), Value::make_buffer(id)],
    )
    .unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("a9993e364706816aba3e25717850c26c9cd0d89d")
    );
}

#[test]
fn secure_hash_eval_buffer_range_errors() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("abc");
    }
    let id = eval.buffers.current_buffer().expect("current buffer").id;

    match builtin_secure_hash(
        &mut eval,
        vec![
            Value::symbol("sha1"),
            Value::make_buffer(id),
            Value::fixnum(5),
        ],
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "args-out-of-range");
            assert_eq!(sig.data, vec![Value::fixnum(5), Value::NIL]);
        }
        other => panic!("expected args-out-of-range signal, got {other:?}"),
    }
}

#[test]
fn secure_hash_eval_buffer_index_type_error() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let id = eval.buffers.current_buffer().expect("current buffer").id;

    match builtin_secure_hash(
        &mut eval,
        vec![
            Value::symbol("sha1"),
            Value::make_buffer(id),
            Value::T,
            Value::fixnum(3),
        ],
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "wrong-type-argument");
            assert_eq!(
                sig.data.first(),
                Some(&Value::symbol("integer-or-marker-p"))
            );
        }
        other => panic!("expected wrong-type-argument signal, got {other:?}"),
    }
}

#[test]
fn secure_hash_eval_buffer_marker_range() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    {
        let buf = eval.buffers.current_buffer_mut().expect("current buffer");
        buf.delete_region(buf.point_min(), buf.point_max());
        buf.insert("abc");
    }
    let id = eval.buffers.current_buffer().expect("current buffer").id;
    let marker =
        crate::emacs_core::marker::make_registered_buffer_marker(&mut eval.buffers, id, 2, false);
    let r = builtin_secure_hash(
        &mut eval,
        vec![
            Value::symbol("sha1"),
            Value::make_buffer(id),
            marker,
            Value::fixnum(4),
        ],
    )
    .unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("5b2505039ac5af9e197f5dad04113906a9cf9a2a")
    );
}

#[test]
fn secure_hash_eval_deleted_buffer_errors() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let id = eval.buffers.create_buffer("*secure-doomed*");
    assert!(eval.buffers.kill_buffer(id));

    match builtin_secure_hash(
        &mut eval,
        vec![Value::symbol("sha1"), Value::make_buffer(id)],
    ) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data.first().and_then(|v| v.as_utf8_str()),
                Some("Selecting deleted buffer")
            );
        }
        other => panic!("expected error signal, got {other:?}"),
    }
}

#[test]
fn buffer_hash_eval_current_buffer_sha1() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let buf = eval.buffers.current_buffer_mut().expect("current buffer");
    buf.delete_region(buf.point_min(), buf.point_max());
    buf.insert("abc");
    let r = builtin_buffer_hash(&mut eval, vec![]).unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("a9993e364706816aba3e25717850c26c9cd0d89d")
    );
}

#[test]
fn buffer_hash_eval_by_name_sha1() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let buf = eval.buffers.current_buffer_mut().expect("current buffer");
    buf.delete_region(buf.point_min(), buf.point_max());
    buf.insert("abc");
    let name = eval
        .buffers
        .current_buffer()
        .expect("current buffer")
        .name_value();
    let r = builtin_buffer_hash(&mut eval, vec![name]).unwrap();
    assert_eq!(
        r.as_utf8_str(),
        Some("a9993e364706816aba3e25717850c26c9cd0d89d")
    );
}

#[test]
fn buffer_hash_eval_missing_name_errors() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    match builtin_buffer_hash(&mut eval, vec![Value::string("*missing*")]) {
        Err(Flow::Signal(sig)) => {
            assert_eq!(sig.symbol_name(), "error");
            assert_eq!(
                sig.data.first().and_then(|v| v.as_utf8_str()),
                Some("No buffer named *missing*")
            );
        }
        other => panic!("expected error signal, got {other:?}"),
    }
}

// ---- equal-including-properties ----

#[test]
fn equal_including_properties_strings() {
    crate::test_utils::init_test_tracing();
    let r =
        builtin_equal_including_properties(vec![Value::string("hello"), Value::string("hello")])
            .unwrap();
    assert!(r.is_truthy());
}

#[test]
fn equal_including_properties_distinguishes_string_text_properties() {
    crate::test_utils::init_test_tracing();
    let with_props = Value::string("abcd");
    crate::emacs_core::value::set_string_text_properties_for_value(
        with_props,
        vec![crate::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 3,
            plist: Value::list(vec![Value::symbol("face"), Value::symbol("bold")]),
        }],
    );
    let plain = Value::string("abcd");

    assert!(crate::emacs_core::value::equal_value(
        &with_props,
        &plain,
        0
    ));

    let equal_props = builtin_equal_including_properties(vec![with_props, plain])
        .expect("equal-including-properties");
    assert!(equal_props.is_nil());
}

#[test]
fn equal_including_properties_recurses_into_cons_string_text_properties() {
    crate::test_utils::init_test_tracing();
    let with_props = Value::string("abcd");
    crate::emacs_core::value::set_string_text_properties_for_value(
        with_props,
        vec![crate::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 3,
            plist: Value::list(vec![Value::symbol("face"), Value::symbol("bold")]),
        }],
    );
    let left = Value::cons(with_props, Value::NIL);
    let right = Value::cons(Value::string("abcd"), Value::NIL);

    assert!(crate::emacs_core::value::equal_value(&left, &right, 0));

    let equal_props =
        builtin_equal_including_properties(vec![left, right]).expect("equal-including-properties");
    assert!(equal_props.is_nil());
}

#[test]
fn string_make_multibyte_passthrough_ascii() {
    crate::test_utils::init_test_tracing();
    let s = Value::string("abc");
    let r = builtin_string_make_multibyte(vec![s]).unwrap();
    assert!(crate::emacs_core::value::eq_value(&s, &r));
    let ls = r.as_lisp_string().unwrap();
    assert!(!ls.is_multibyte());
    assert_eq!(r.as_utf8_str(), Some("abc"));
}

#[test]
fn string_make_multibyte_promotes_unibyte_byte() {
    crate::test_utils::init_test_tracing();
    let v = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xFF]));
    let r = builtin_string_make_multibyte(vec![v]).unwrap();
    let ls = r.as_lisp_string().unwrap();
    assert!(ls.is_multibyte());
    let codes: Vec<u32> = crate::emacs_core::builtins::lisp_string_char_codes(ls);
    assert_eq!(codes, vec![0x3FFFFF]);
}

#[test]
fn string_make_unibyte_passthrough_ascii() {
    crate::test_utils::init_test_tracing();
    let s = Value::string("abc");
    let r = builtin_string_make_unibyte(vec![s]).unwrap();
    assert!(crate::emacs_core::value::eq_value(&s, &r));
    let ls = r.as_lisp_string().unwrap();
    assert!(!ls.is_multibyte());
    assert_eq!(ls.as_bytes(), b"abc");
}

#[test]
fn string_make_unibyte_truncates_unicode_char_code() {
    crate::test_utils::init_test_tracing();
    let r = builtin_string_make_unibyte(vec![Value::string("😀")]).unwrap();
    let ls = r.as_lisp_string().unwrap();
    assert!(!ls.is_multibyte());
    // 😀 is U+1F600, truncated to byte: low byte is 0x00
    assert_eq!(ls.as_bytes(), &[0]);
}

// ---- compare-strings ----

#[test]
fn compare_strings_equal() {
    crate::test_utils::init_test_tracing();
    let r = builtin_compare_strings(vec![
        Value::string("hello"),
        Value::NIL,
        Value::NIL,
        Value::string("hello"),
        Value::NIL,
        Value::NIL,
    ])
    .unwrap();
    assert!(r.is_t());
}

#[test]
fn compare_strings_less() {
    crate::test_utils::init_test_tracing();
    let r = builtin_compare_strings(vec![
        Value::string("abc"),
        Value::NIL,
        Value::NIL,
        Value::string("abd"),
        Value::NIL,
        Value::NIL,
    ])
    .unwrap();
    // First diff at position 3, "c" < "d" so negative
    assert_eq!(r.as_int(), Some(-3));
}

#[test]
fn compare_strings_greater() {
    crate::test_utils::init_test_tracing();
    let r = builtin_compare_strings(vec![
        Value::string("abd"),
        Value::NIL,
        Value::NIL,
        Value::string("abc"),
        Value::NIL,
        Value::NIL,
    ])
    .unwrap();
    assert_eq!(r.as_int(), Some(3));
}

#[test]
fn compare_strings_ignore_case() {
    crate::test_utils::init_test_tracing();
    let r = builtin_compare_strings(vec![
        Value::string("Hello"),
        Value::NIL,
        Value::NIL,
        Value::string("hello"),
        Value::NIL,
        Value::NIL,
        Value::T, // IGNORE-CASE
    ])
    .unwrap();
    assert!(r.is_t());
}

#[test]
fn compare_strings_subrange() {
    crate::test_utils::init_test_tracing();
    // Compare "hel" from "hello" (chars 1-3) with "hel" from "help" (chars 1-3)
    let r = builtin_compare_strings(vec![
        Value::string("hello"),
        Value::fixnum(1),
        Value::fixnum(3),
        Value::string("help"),
        Value::fixnum(1),
        Value::fixnum(3),
    ])
    .unwrap();
    assert!(r.is_t());
}

#[test]
fn compare_strings_length_diff() {
    crate::test_utils::init_test_tracing();
    let r = builtin_compare_strings(vec![
        Value::string("ab"),
        Value::NIL,
        Value::NIL,
        Value::string("abc"),
        Value::NIL,
        Value::NIL,
    ])
    .unwrap();
    // "ab" shorter — negative
    assert!(r.as_int().unwrap() < 0);
}

#[test]
fn compare_strings_negative_bounds_and_too_large_end_match_gnu() {
    crate::test_utils::init_test_tracing();

    let negative_bounds = builtin_compare_strings(vec![
        Value::string("abcdef"),
        Value::fixnum(-3),
        Value::fixnum(-1),
        Value::string("cd"),
        Value::NIL,
        Value::NIL,
    ])
    .unwrap();
    assert_eq!(negative_bounds.as_int(), Some(1));

    let clamped_end = builtin_compare_strings(vec![
        Value::string("abc"),
        Value::fixnum(0),
        Value::fixnum(99),
        Value::string("abc"),
        Value::fixnum(0),
        Value::fixnum(99),
    ])
    .unwrap();
    assert_eq!(clamped_end, Value::T);
}

#[test]
fn compare_strings_reversed_and_out_of_range_bounds_signal_like_gnu() {
    crate::test_utils::init_test_tracing();

    for args in [
        vec![
            Value::string("abc"),
            Value::fixnum(3),
            Value::fixnum(2),
            Value::string("abc"),
            Value::NIL,
            Value::NIL,
        ],
        vec![
            Value::string("abc"),
            Value::fixnum(9),
            Value::NIL,
            Value::string(""),
            Value::NIL,
            Value::NIL,
        ],
        vec![
            Value::string("abc"),
            Value::NIL,
            Value::fixnum(-9),
            Value::string(""),
            Value::NIL,
            Value::NIL,
        ],
    ] {
        match builtin_compare_strings(args) {
            Err(Flow::Signal(sig)) => assert_eq!(sig.symbol_name(), "args-out-of-range"),
            other => panic!("expected args-out-of-range signal, got {other:?}"),
        }
    }
}

#[test]
fn compare_strings_ignore_case_uses_upcase_like_gnu() {
    crate::test_utils::init_test_tracing();

    let result = builtin_compare_strings(vec![
        Value::string("İ"),
        Value::NIL,
        Value::NIL,
        Value::string("i"),
        Value::NIL,
        Value::NIL,
        Value::T,
    ])
    .unwrap();
    assert_eq!(result.as_int(), Some(1));
}

// ---- string-version-lessp ----

#[test]
fn version_lessp_basic() {
    crate::test_utils::init_test_tracing();
    let r =
        builtin_string_version_lessp(vec![Value::string("foo2"), Value::string("foo10")]).unwrap();
    assert!(r.is_truthy());
}

#[test]
fn version_lessp_equal() {
    crate::test_utils::init_test_tracing();
    let r =
        builtin_string_version_lessp(vec![Value::string("foo10"), Value::string("foo10")]).unwrap();
    assert!(r.is_nil());
}

#[test]
fn version_lessp_alpha() {
    crate::test_utils::init_test_tracing();
    let r = builtin_string_version_lessp(vec![Value::string("abc"), Value::string("abd")]).unwrap();
    assert!(r.is_truthy());
}

#[test]
fn version_lessp_numeric_segments() {
    crate::test_utils::init_test_tracing();
    let r = builtin_string_version_lessp(vec![
        Value::string("emacs-27.1"),
        Value::string("emacs-27.2"),
    ])
    .unwrap();
    assert!(r.is_truthy());
}

#[test]
fn version_lessp_leading_zero_runs_match_gnu() {
    crate::test_utils::init_test_tracing();
    let equal_numeric =
        builtin_string_version_lessp(vec![Value::string("1"), Value::string("001")])
            .expect("string-version-lessp should evaluate");
    assert!(equal_numeric.is_nil());

    let reverse_equal_numeric =
        builtin_string_version_lessp(vec![Value::string("001"), Value::string("1")])
            .expect("string-version-lessp should evaluate");
    assert!(reverse_equal_numeric.is_nil());
}

// ---- string-collate-lessp ----

#[test]
fn collate_lessp_basic() {
    crate::test_utils::init_test_tracing();
    let r = builtin_string_collate_lessp(vec![Value::string("abc"), Value::string("abd")]).unwrap();
    assert!(r.is_truthy());
}

#[test]
fn collate_lessp_ignore_case() {
    crate::test_utils::init_test_tracing();
    let r = builtin_string_collate_lessp(vec![
        Value::string("ABC"),
        Value::string("abd"),
        Value::NIL, // locale
        Value::T,   // ignore-case
    ])
    .unwrap();
    assert!(r.is_truthy());
}

#[test]
fn collate_lessp_rejects_non_string_locale() {
    crate::test_utils::init_test_tracing();
    let err = builtin_string_collate_lessp(vec![
        Value::string("a"),
        Value::string("b"),
        Value::fixnum(42),
    ])
    .expect_err("non-nil locale must be a string");
    match err {
        Flow::Signal(sig) => {
            assert_eq!(
                sig.symbol,
                Value::symbol("wrong-type-argument").as_symbol_id().unwrap()
            );
            assert_eq!(sig.data[0], Value::symbol("stringp"));
        }
        other => panic!("expected signal, got {other:?}"),
    }
}

#[test]
fn collate_lessp_invalid_locale_signals_error() {
    crate::test_utils::init_test_tracing();
    let err = builtin_string_collate_lessp(vec![
        Value::string("a"),
        Value::string("b"),
        Value::string("neomacs-invalid-locale"),
    ])
    .expect_err("invalid explicit locale should signal error");
    match err {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol, Value::symbol("error").as_symbol_id().unwrap());
        }
        other => panic!("expected signal, got {other:?}"),
    }
}

// ---- string-collate-equalp ----

#[test]
fn collate_equalp_basic() {
    crate::test_utils::init_test_tracing();
    let r =
        builtin_string_collate_equalp(vec![Value::string("abc"), Value::string("abc")]).unwrap();
    assert!(r.is_truthy());
}

#[test]
fn collate_equalp_ignore_case() {
    crate::test_utils::init_test_tracing();
    let r = builtin_string_collate_equalp(vec![
        Value::string("ABC"),
        Value::string("abc"),
        Value::NIL,
        Value::T,
    ])
    .unwrap();
    assert!(r.is_truthy());
}

#[test]
fn collate_equalp_different() {
    crate::test_utils::init_test_tracing();
    let r =
        builtin_string_collate_equalp(vec![Value::string("abc"), Value::string("abd")]).unwrap();
    assert!(r.is_nil());
}

#[test]
fn collate_equalp_rejects_non_string_locale() {
    crate::test_utils::init_test_tracing();
    let err = builtin_string_collate_equalp(vec![
        Value::string("a"),
        Value::string("a"),
        Value::symbol("not-a-locale"),
    ])
    .expect_err("non-nil locale must be a string");
    match err {
        Flow::Signal(sig) => {
            assert_eq!(
                sig.symbol,
                Value::symbol("wrong-type-argument").as_symbol_id().unwrap()
            );
            assert_eq!(sig.data[0], Value::symbol("stringp"));
        }
        other => panic!("expected signal, got {other:?}"),
    }
}

// ---- widget-get / widget-put ----

#[test]
fn widget_get_found() {
    crate::test_utils::init_test_tracing();
    // Widget: (button :tag "OK" :value 42)
    let widget = Value::list(vec![
        Value::symbol("button"),
        Value::keyword("tag"),
        Value::string("OK"),
        Value::keyword("value"),
        Value::fixnum(42),
    ]);
    let r = builtin_widget_get(vec![widget, Value::keyword("value")]).unwrap();
    assert!(r.is_fixnum());
}

#[test]
fn widget_get_not_found() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![
        Value::symbol("button"),
        Value::keyword("tag"),
        Value::string("OK"),
    ]);
    let r = builtin_widget_get(vec![widget, Value::keyword("missing")]).unwrap();
    assert!(r.is_nil());
}

#[test]
fn widget_put_existing() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![
        Value::symbol("button"),
        Value::keyword("value"),
        Value::fixnum(1),
    ]);
    let r = builtin_widget_put(vec![widget, Value::keyword("value"), Value::fixnum(99)]).unwrap();
    assert!(r.is_fixnum());

    // Verify it was modified
    let got = builtin_widget_get(vec![widget, Value::keyword("value")]).unwrap();
    assert!(got.is_fixnum());
}

#[test]
fn widget_put_new_property() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![Value::symbol("button")]);
    let r =
        builtin_widget_put(vec![widget, Value::keyword("tag"), Value::string("Hello")]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("Hello"));

    let got = builtin_widget_get(vec![widget, Value::keyword("tag")]).unwrap();
    assert_eq!(got.as_utf8_str(), Some("Hello"));
}

#[test]
fn widget_apply_missing_property_signals_void_function_nil() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![Value::symbol("button")]);
    let mut ctx = test_eval_ctx();
    let err = builtin_widget_apply(&mut ctx, vec![widget, Value::keyword("action")])
        .expect_err("widget-apply should signal void-function for missing property");
    match err {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "void-function");
            assert_eq!(sig.data, vec![Value::NIL]);
        }
        other => panic!("unexpected flow: {other:?}"),
    }
}

#[test]
fn widget_apply_calls_symbol_property_with_widget_as_first_arg() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![
        Value::symbol("button"),
        Value::keyword("action"),
        Value::symbol("car"),
    ]);
    let mut ctx = test_eval_ctx();
    let r = builtin_widget_apply(&mut ctx, vec![widget, Value::keyword("action")]).unwrap();
    assert_eq!(r, Value::symbol("button"));
}

#[test]
fn widget_apply_passes_rest_arguments() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![
        Value::symbol("button"),
        Value::keyword("action"),
        Value::symbol("list"),
    ]);
    let mut ctx = test_eval_ctx();
    let r = builtin_widget_apply(
        &mut ctx,
        vec![
            widget,
            Value::keyword("action"),
            Value::fixnum(1),
            Value::fixnum(2),
        ],
    )
    .unwrap();
    assert_eq!(
        r,
        Value::list(vec![widget, Value::fixnum(1), Value::fixnum(2)])
    );
}

#[test]
fn widget_apply_non_callable_property_signals_invalid_function() {
    crate::test_utils::init_test_tracing();
    let widget = Value::list(vec![
        Value::symbol("button"),
        Value::keyword("action"),
        Value::fixnum(7),
    ]);
    let mut ctx = test_eval_ctx();
    let err = builtin_widget_apply(&mut ctx, vec![widget, Value::keyword("action")])
        .expect_err("widget-apply should reject non-callable property values");
    match err {
        Flow::Signal(sig) => {
            assert_eq!(sig.symbol_name(), "invalid-function");
            assert_eq!(sig.data, vec![Value::fixnum(7)]);
        }
        other => panic!("unexpected flow: {other:?}"),
    }
}

// ---- Line break in base64 ----

#[test]
fn base64_encode_line_break() {
    crate::test_utils::init_test_tracing();
    // A string long enough to trigger line breaks at column 76
    let long = "a".repeat(100);
    let encoded = builtin_base64_encode_string(vec![Value::string(long.clone())]).unwrap();
    let s = encoded.as_utf8_str().unwrap();
    assert!(s.contains('\n'));

    // No line break variant
    let encoded_no_lb = builtin_base64_encode_string(vec![Value::string(long), Value::T]).unwrap();
    let s2 = encoded_no_lb.as_utf8_str().unwrap();
    assert!(!s2.contains('\n'));
}

#[test]
fn base64_decode_ignores_whitespace() {
    crate::test_utils::init_test_tracing();
    // Encoded "Hello" with embedded whitespace
    let r = builtin_base64_decode_string(vec![Value::string("SGVs\nbG8=")]).unwrap();
    assert_eq!(r.as_utf8_str(), Some("Hello"));
}
