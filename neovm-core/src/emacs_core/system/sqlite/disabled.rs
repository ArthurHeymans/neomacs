//! GNU-compatible SQLite capability probes for feature-disabled builds.

use super::super::error::EvalResult;
use super::super::value::Value;

pub(crate) fn predicate(args: Vec<Value>) -> EvalResult {
    super::super::builtins::expect_args("sqlitep", &args, 1)?;
    Ok(Value::NIL)
}

pub(crate) fn available_p(args: Vec<Value>) -> EvalResult {
    super::super::builtins::expect_args("sqlite-available-p", &args, 0)?;
    Ok(Value::NIL)
}
