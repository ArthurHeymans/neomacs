//! Lisp bridge for compositor-owned neo-term terminal instances.
//!
//! The evaluator validates Lisp values and sends typed requests through
//! [`DisplayHost`]. PTY ownership, VT parsing, and rendering remain entirely
//! behind the display-runtime boundary.

use super::error::{EvalResult, signal};
use super::eval::{Context, TerminalCreateRequest, TerminalDisplayMode, TerminalFloatPlacement};
use super::value::Value;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalOperation {
    Create,
    Write,
    Resize,
    Destroy,
    SetFloat,
    GetText,
}

impl Display for TerminalOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Create => "neomacs-terminal-create",
            Self::Write => "neomacs-terminal-write",
            Self::Resize => "neomacs-terminal-resize",
            Self::Destroy => "neomacs-terminal-destroy",
            Self::SetFloat => "neomacs-terminal-set-float",
            Self::GetText => "neomacs-terminal-get-text",
        })
    }
}

fn terminal_error(message: impl Into<String>) -> super::error::Flow {
    signal("error", vec![Value::string(message.into())])
}

fn wrong_type(predicate: &str, value: Value) -> super::error::Flow {
    signal("wrong-type-argument", vec![Value::symbol(predicate), value])
}

fn positive_u16(
    value: Value,
    operation: TerminalOperation,
    argument: &str,
) -> Result<u16, super::error::Flow> {
    let integer = value.as_int().ok_or_else(|| wrong_type("fixnump", value))?;
    u16::try_from(integer)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| terminal_error(format!("{operation}: {argument} must be in 1..=65535")))
}

fn terminal_id(value: Value, operation: TerminalOperation) -> Result<u32, super::error::Flow> {
    let integer = value.as_int().ok_or_else(|| wrong_type("fixnump", value))?;
    u32::try_from(integer)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| terminal_error(format!("{operation}: terminal id must be positive")))
}

fn number(
    value: Value,
    operation: TerminalOperation,
    argument: &str,
) -> Result<f32, super::error::Flow> {
    let number = value
        .as_int()
        .map(|value| value as f32)
        .or_else(|| value.as_float().map(|value| value as f32))
        .ok_or_else(|| wrong_type("numberp", value))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(terminal_error(format!(
            "{operation}: {argument} must be finite"
        )))
    }
}

fn display_host(
    eval: &Context,
    operation: TerminalOperation,
) -> Result<&dyn super::eval::DisplayHost, super::error::Flow> {
    eval.display_host
        .as_deref()
        .ok_or_else(|| terminal_error(format!("{operation}: no GUI display host in this session")))
}

/// `(neomacs-terminal-create COLS ROWS MODE &optional SHELL)`.
pub(crate) fn builtin_neomacs_terminal_create(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    const OPERATION: TerminalOperation = TerminalOperation::Create;
    let cols = positive_u16(args[0], OPERATION, "COLS")?;
    let rows = positive_u16(args[1], OPERATION, "ROWS")?;
    let mode = match args[2]
        .as_int()
        .ok_or_else(|| wrong_type("fixnump", args[2]))?
    {
        0 => TerminalDisplayMode::Window,
        1 => TerminalDisplayMode::Inline,
        2 => TerminalDisplayMode::Floating,
        _ => {
            return Err(terminal_error(format!(
                "{OPERATION}: MODE must be 0, 1, or 2"
            )));
        }
    };
    let shell = match args.get(3).copied().unwrap_or(Value::NIL) {
        value if value.is_nil() => None,
        value => Some(
            value
                .as_lisp_string()
                .ok_or_else(|| wrong_type("stringp", value))?
                .as_utf8_str()
                .ok_or_else(|| terminal_error(format!("{OPERATION}: SHELL must be UTF-8")))?
                .to_owned(),
        ),
    };
    let id = display_host(eval, OPERATION)?
        .create_terminal(TerminalCreateRequest {
            cols,
            rows,
            mode,
            shell,
        })
        .map_err(terminal_error)?;
    Ok(if id == 0 {
        Value::NIL
    } else {
        Value::fixnum(i64::from(id))
    })
}

/// `(neomacs-terminal-write TERMINAL-ID STRING)`.
pub(crate) fn builtin_neomacs_terminal_write(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    const OPERATION: TerminalOperation = TerminalOperation::Write;
    let id = terminal_id(args[0], OPERATION)?;
    let data = args[1]
        .as_lisp_string()
        .ok_or_else(|| wrong_type("stringp", args[1]))?
        .as_bytes()
        .to_vec();
    display_host(eval, OPERATION)?
        .write_terminal(id, data)
        .map_err(terminal_error)?;
    Ok(Value::T)
}

/// `(neomacs-terminal-resize TERMINAL-ID COLS ROWS)`.
pub(crate) fn builtin_neomacs_terminal_resize(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    const OPERATION: TerminalOperation = TerminalOperation::Resize;
    let id = terminal_id(args[0], OPERATION)?;
    let cols = positive_u16(args[1], OPERATION, "COLS")?;
    let rows = positive_u16(args[2], OPERATION, "ROWS")?;
    display_host(eval, OPERATION)?
        .resize_terminal(id, cols, rows)
        .map_err(terminal_error)?;
    Ok(Value::T)
}

/// `(neomacs-terminal-destroy TERMINAL-ID)`.
pub(crate) fn builtin_neomacs_terminal_destroy(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    const OPERATION: TerminalOperation = TerminalOperation::Destroy;
    let id = terminal_id(args[0], OPERATION)?;
    display_host(eval, OPERATION)?
        .destroy_terminal(id)
        .map_err(terminal_error)?;
    Ok(Value::T)
}

/// `(neomacs-terminal-set-float TERMINAL-ID X Y OPACITY)`.
pub(crate) fn builtin_neomacs_terminal_set_float(
    eval: &mut Context,
    args: Vec<Value>,
) -> EvalResult {
    const OPERATION: TerminalOperation = TerminalOperation::SetFloat;
    let id = terminal_id(args[0], OPERATION)?;
    let x = number(args[1], OPERATION, "X")?;
    let y = number(args[2], OPERATION, "Y")?;
    let opacity = number(args[3], OPERATION, "OPACITY")?;
    if !(0.0..=1.0).contains(&opacity) {
        return Err(terminal_error(format!(
            "{OPERATION}: OPACITY must be in 0.0..=1.0"
        )));
    }
    display_host(eval, OPERATION)?
        .set_floating_terminal(id, TerminalFloatPlacement { x, y, opacity })
        .map_err(terminal_error)?;
    Ok(Value::T)
}

/// `(neomacs-terminal-get-text TERMINAL-ID)`.
pub(crate) fn builtin_neomacs_terminal_get_text(
    eval: &mut Context,
    args: Vec<Value>,
) -> EvalResult {
    const OPERATION: TerminalOperation = TerminalOperation::GetText;
    let id = terminal_id(args[0], OPERATION)?;
    Ok(display_host(eval, OPERATION)?
        .terminal_text(id)
        .map_err(terminal_error)?
        .map(Value::string)
        .unwrap_or(Value::NIL))
}
