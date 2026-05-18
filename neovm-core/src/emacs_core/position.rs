use super::error::{Flow, signal};
use super::value::{Value, ValueKind, VecLikeType};
use crate::buffer::BufferManager;

pub(crate) fn fix_position_with_buffers(
    buffers: &BufferManager,
    value: &Value,
) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => {
            super::marker::marker_position_as_int_with_buffers(buffers, value)
        }
        ValueKind::Veclike(VecLikeType::Bignum) => Ok(fix_position_bignum(value)),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

pub(crate) fn fix_position_eval(eval: &super::eval::Context, value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ if super::marker::is_marker(value) => {
            super::marker::marker_position_as_int_eval(eval, value)
        }
        ValueKind::Veclike(VecLikeType::Bignum) => Ok(fix_position_bignum(value)),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

fn fix_position_bignum(value: &Value) -> i64 {
    let n = value.as_bignum().expect("bignum kind");
    if n >= &rug::Integer::from(0) {
        Value::MOST_POSITIVE_FIXNUM
    } else {
        Value::MOST_NEGATIVE_FIXNUM
    }
}
