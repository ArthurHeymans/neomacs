//! DBus compatibility builtins.
//!
//! NeoVM does not include DBus transport, but a subset of DBus primitives are
//! exposed for startup/runtime compatibility with expected arity and basic
//! error contracts.

use super::error::{EvalResult, Flow, signal};
use super::eval::Context;
use super::intern::resolve_sym;
use super::value::{Value, ValueKind};
use strum::{EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
enum DbusBusName {
    #[strum(serialize = ":system")]
    System,
    #[strum(serialize = ":session")]
    Session,
    #[strum(serialize = ":system-private")]
    SystemPrivate,
    #[strum(serialize = ":session-private")]
    SessionPrivate,
}

impl DbusBusName {
    fn from_symbol_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    fn unique_name(self) -> &'static str {
        match self {
            Self::System | Self::SystemPrivate => ":1.0",
            Self::Session | Self::SessionPrivate => ":1.1",
        }
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_range_args(
    name: &str,
    args: &[Value],
    min: usize,
    max: Option<usize>,
) -> Result<(), Flow> {
    let out_of_range = match max {
        Some(max) => args.len() < min || args.len() > max,
        None => args.len() < min,
    };
    if out_of_range {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_symbolp(value: &Value) -> Result<String, Flow> {
    match value.kind() {
        ValueKind::Symbol(id) => Ok(resolve_sym(id).to_owned()),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), *value],
        )),
    }
}

fn expect_wholenump(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Ok(n),
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("wholenump"), *value],
        )),
    }
}

fn dbus_error(msg: &str, details: Value) -> Flow {
    signal("dbus-error", vec![Value::string(msg), details])
}

fn recognized_bus_name(name: &str) -> Option<DbusBusName> {
    DbusBusName::from_symbol_name(name)
}

/// `(dbus--init-bus BUS &optional PRIVATE)` -- initialize BUS and return
/// a numeric handle.
pub(crate) fn builtin_dbus_init_bus(args: Vec<Value>) -> EvalResult {
    expect_range_args("dbus--init-bus", &args, 1, Some(2))?;
    let bus = expect_symbolp(&args[0])?;
    if recognized_bus_name(&bus).is_some() {
        Ok(Value::fixnum(2))
    } else {
        Err(dbus_error("Wrong bus name", Value::symbol(bus)))
    }
}

/// `(dbus-get-unique-name BUS)` -- resolve unique name for BUS.
pub(crate) fn builtin_dbus_get_unique_name(args: Vec<Value>) -> EvalResult {
    expect_args("dbus-get-unique-name", &args, 1)?;
    let bus = expect_symbolp(&args[0])?;
    if let Some(bus_name) = recognized_bus_name(&bus) {
        Ok(Value::string(bus_name.unique_name()))
    } else {
        Err(dbus_error("Wrong bus name", Value::symbol(bus)))
    }
}

/// `(dbus-message-internal BUS-ID DESTINATION ... )` -- DBus call helper.
pub(crate) fn builtin_dbus_message_internal(ctx: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_range_args("dbus-message-internal", &args, 4, None)?;
    let message_type = expect_wholenump(&args[0])?;

    if message_type == 1 && args.len() >= 7 {
        let bus = args[1];
        let serial = ctx.dbus_next_serial;
        ctx.dbus_next_serial += 1;
        let key = Value::list(vec![Value::keyword(":serial"), bus, Value::fixnum(serial)]);
        let handler = args[6];
        let event = Value::list(vec![
            Value::symbol("dbus-event"),
            bus,
            Value::fixnum(2),
            Value::fixnum(serial),
            Value::string("org.freedesktop.DBus"),
            Value::NIL,
            Value::NIL,
            Value::NIL,
            Value::NIL,
            handler,
        ]);
        ctx.queue_special_event(event);
        return Ok(key);
    }

    match args[1].kind() {
        ValueKind::Symbol(_) => Ok(Value::NIL),
        ValueKind::String => {
            let dest = args[1]
                .as_runtime_string_owned()
                .expect("ValueKind::String must carry LispString payload");
            if !dest.contains(':') {
                Err(signal(
                    "dbus-error",
                    vec![Value::string("Address does not contain a colon")],
                ))
            } else if args.len() == 4 {
                Err(signal(
                    "wrong-number-of-arguments",
                    vec![Value::symbol("dbus-message-internal"), Value::fixnum(4)],
                ))
            } else {
                Ok(Value::NIL)
            }
        }
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[1]],
        )),
    }
}

#[cfg(test)]
#[path = "dbus_test.rs"]
mod tests;
