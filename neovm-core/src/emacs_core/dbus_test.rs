use super::*;

#[test]
fn dbus_init_bus_contract() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_dbus_init_bus(vec![Value::keyword(":session")]).unwrap(),
        Value::fixnum(2)
    );
    let err = builtin_dbus_init_bus(vec![Value::fixnum(1)]).unwrap_err();
    match err {
        Flow::Signal(sig) => assert_eq!(sig.symbol_name(), "wrong-type-argument"),
        other => panic!("expected signal, got {other:?}"),
    }
}

#[test]
fn dbus_get_unique_name_returns_compat_unique_name() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_dbus_get_unique_name(vec![Value::keyword(":system")]).unwrap(),
        Value::string(":1.0")
    );
}

#[test]
fn dbus_message_internal_validates_first_arg() {
    crate::test_utils::init_test_tracing();
    let mut ev = crate::emacs_core::eval::Context::new();
    let err = builtin_dbus_message_internal(
        &mut ev,
        vec![
            Value::keyword(":session"),
            Value::string("/"),
            Value::string("org.example"),
            Value::string("Ping"),
        ],
    )
    .unwrap_err();
    match err {
        Flow::Signal(sig) => assert_eq!(sig.symbol_name(), "wrong-type-argument"),
        other => panic!("expected signal, got {other:?}"),
    }
}
