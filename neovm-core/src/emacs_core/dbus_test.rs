use super::*;

#[test]
fn dbus_init_bus_contract() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        builtin_dbus_init_bus(vec![Value::keyword(":session")]).unwrap(),
        Value::fixnum(2)
    );
    assert_eq!(
        builtin_dbus_init_bus(vec![Value::keyword(":session-private")]).unwrap(),
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
    assert_eq!(
        builtin_dbus_get_unique_name(vec![Value::keyword(":system-private")]).unwrap(),
        Value::string(":1.0")
    );
}

#[test]
fn dbus_bus_name_domain_matches_gnu_symbols() {
    crate::test_utils::init_test_tracing();
    assert_eq!(
        DbusBusName::from_symbol_name(":system"),
        Some(DbusBusName::System)
    );
    assert_eq!(
        DbusBusName::from_symbol_name(":session"),
        Some(DbusBusName::Session)
    );
    assert_eq!(
        DbusBusName::from_symbol_name(":system-private"),
        Some(DbusBusName::SystemPrivate)
    );
    assert_eq!(
        DbusBusName::from_symbol_name(":session-private"),
        Some(DbusBusName::SessionPrivate)
    );
    assert_eq!(DbusBusName::from_symbol_name(":starter"), None);
    assert_eq!(DbusBusName::SystemPrivate.name(), ":system-private");
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
