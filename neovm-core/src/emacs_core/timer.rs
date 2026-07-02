//! `sleep-for` — the only timer-adjacent C builtin neomacs keeps.
//!
//! GNU implements the whole timer system in Lisp (`timer.el`): `run-at-time`,
//! `run-with-timer`, `run-with-idle-timer`, `timer-activate`, `cancel-timer`,
//! and the retrigger/`timer-max-repeats` logic all live there, storing timer
//! vectors in the `timer-list` / `timer-idle-list` variables. The C side only
//! *reads* those lists: `timer_check` (keyboard.c) fires due timers via
//! `timer-event-handler`, and `wait_reading_process_output` folds the next
//! timer deadline into its pselect timeout. neomacs mirrors that split — the
//! Lisp-visible timer surface comes from loading GNU's timer.el, the wait loop
//! reads `timer-list` (`next_due_gnu_timer_snapshot` /
//! `next_ordinary_gnu_timer_timeout` in keyboard.rs), and no native timer
//! store exists. A previous native `TimerManager` "second brain" (with its own
//! unregistered `run-at-time`/`timer-activate` builtins and a divergent
//! `now + interval` rescheduling rule) had no live writers and was removed.
//!
//! `sleep-for` stays native because GNU's is C (`Fsleep_for`, dispnew.c): it
//! parses SECONDS/MILLISECONDS and enters `wait_reading_process_output`.

use std::time::{Duration, Instant};

use super::error::{EvalResult, Flow, signal};
use super::value::{Value, ValueKind, VecLikeType};
use malachite::base::num::conversion::traits::RoundingFrom;
use malachite::base::rounding_modes::RoundingMode;

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_number(value: &Value) -> Result<f64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n as f64),
        ValueKind::Float => Ok(value.xfloat()),
        ValueKind::Veclike(VecLikeType::Bignum) => {
            Ok(f64::rounding_from(value.as_bignum().unwrap(), RoundingMode::Nearest).0)
        }
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("numberp"), *value],
        )),
    }
}

fn expect_fixnum_like(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("fixnump"), *value],
        )),
    }
}

/// `(sleep-for SECONDS &optional MILLISECONDS)` — GNU `Fsleep_for`
/// (dispnew.c): pause, reading process output, without redisplay or servicing
/// command input.
pub(crate) fn builtin_sleep_for(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("sleep-for", &args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol("sleep-for"), Value::fixnum(args.len() as i64)],
        ));
    }

    let secs = expect_number(&args[0])?;
    let millis = if args.len() > 1 {
        if args[1].is_nil() {
            0.0
        } else {
            // GNU Emacs requires a fixnum for the MILLISECONDS argument.
            expect_fixnum_like(&args[1])? as f64
        }
    } else {
        0.0
    };

    let total_secs = secs + millis / 1000.0;
    if total_secs > 0.0 {
        let total = Duration::from_secs_f64(total_secs);
        let _ = eval.wait_until(Instant::now() + total)?;
    }

    Ok(Value::NIL)
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
#[path = "timer_test.rs"]
mod tests;
