//! Time and date builtins for the Elisp interpreter.
//!
//! Implements `current-time`, `float-time`, `time-add`, `time-subtract`,
//! `time-less-p`, `time-equal-p`, `current-time-string`, `current-time-zone`,
//! `encode-time`, `decode-time`, `time-convert`, and `set-time-zone-rule`.
//!
//! Uses `std::time::SystemTime`/`UNIX_EPOCH` for time operations.

use super::error::{EvalResult, Flow, signal};
use super::eval::Context;
use super::intern::{intern, resolve_sym};
use super::value::*;
use crate::emacs_core::value::ValueKind;
use malachite::base::num::conversion::traits::RoundingFrom;
use malachite::base::rounding_modes::RoundingMode;
use malachite::integer::Integer;
use std::cell::RefCell;
use std::ffi::{CStr, OsString};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumString, strum::IntoStaticStr)]
enum TimeConvertSymbolForm {
    #[strum(serialize = "integer")]
    Integer,
    #[strum(serialize = "list")]
    List,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TimeConvertForm {
    Integer,
    List,
    InputHz,
    ExplicitHz(Integer),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumString, strum::IntoStaticStr)]
enum TimeZoneSymbol {
    #[strum(serialize = "wall")]
    Wall,
}

impl TimeZoneSymbol {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

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

fn expect_min_max_args(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), Flow> {
    if args.len() < min || args.len() > max {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal time representation
// ---------------------------------------------------------------------------

/// Internal microsecond-precision time (seconds + microseconds since epoch).
/// Allows negative values for times before the epoch.
#[derive(Clone, Copy, Debug)]
struct TimeMicros {
    /// Total seconds (may be negative).
    secs: i64,
    /// Microseconds within the current second, always in [0, 999_999].
    usecs: i64,
    /// Picoseconds within the current microsecond, always in [0, 999_999].
    psecs: i64,
}

impl TimeMicros {
    fn now() -> Self {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => TimeMicros {
                secs: dur.as_secs() as i64,
                usecs: dur.subsec_micros() as i64,
                psecs: 0,
            },
            Err(e) => {
                let dur = e.duration();
                TimeMicros {
                    secs: -(dur.as_secs() as i64),
                    usecs: -(dur.subsec_micros() as i64),
                    psecs: 0,
                }
            }
        }
    }

    fn to_list(&self) -> Value {
        let high = self.secs >> 16;
        let low = self.secs & 0xFFFF;
        Value::list(vec![
            Value::fixnum(high),
            Value::fixnum(low),
            Value::fixnum(self.usecs),
            Value::fixnum(self.psecs),
        ])
    }

    fn to_float(&self) -> f64 {
        self.secs as f64 + self.usecs as f64 / 1_000_000.0
    }

    fn add(self, other: TimeMicros) -> TimeMicros {
        let mut psecs = self.psecs + other.psecs;
        let mut usecs = self.usecs + other.usecs;
        let mut secs = self.secs + other.secs;
        if psecs >= 1_000_000 {
            psecs -= 1_000_000;
            usecs += 1;
        } else if psecs < 0 {
            psecs += 1_000_000;
            usecs -= 1;
        }
        if usecs >= 1_000_000 {
            usecs -= 1_000_000;
            secs += 1;
        } else if usecs < 0 {
            usecs += 1_000_000;
            secs -= 1;
        }
        TimeMicros { secs, usecs, psecs }
    }

    fn sub(self, other: TimeMicros) -> TimeMicros {
        let mut psecs = self.psecs - other.psecs;
        let mut usecs = self.usecs - other.usecs;
        let mut secs = self.secs - other.secs;
        if psecs < 0 {
            psecs += 1_000_000;
            usecs -= 1;
        } else if psecs >= 1_000_000 {
            psecs -= 1_000_000;
            usecs += 1;
        }
        if usecs < 0 {
            usecs += 1_000_000;
            secs -= 1;
        } else if usecs >= 1_000_000 {
            usecs -= 1_000_000;
            secs += 1;
        }
        TimeMicros { secs, usecs, psecs }
    }

    fn from_ticks_hz(ticks: i64, hz: i64) -> Result<TimeMicros, Flow> {
        if hz <= 0 {
            return Err(signal(
                "error",
                vec![Value::string("Invalid time specification")],
            ));
        }

        let secs = ticks.div_euclid(hz);
        let rem = ticks.rem_euclid(hz) as i128;
        let hz = hz as i128;
        let micros_total = rem * 1_000_000;
        let usecs = (micros_total / hz) as i64;
        let psecs = (((micros_total % hz) * 1_000_000) / hz) as i64;
        Ok(TimeMicros { secs, usecs, psecs })
    }

    fn from_exact_ticks_hz(ticks: &Integer, hz: &Integer) -> Result<TimeMicros, Flow> {
        if hz <= &Integer::from(0) {
            return Err(signal(
                "error",
                vec![Value::string("Invalid time specification")],
            ));
        }

        let trillion = Integer::from(1_000_000_000_000i64);
        let million = Integer::from(1_000_000i64);
        let total_psecs = integer_div_floor(&(ticks * &trillion), hz);
        let secs = integer_div_floor(&total_psecs, &trillion);
        let rem_psecs = &total_psecs - &secs * &trillion;
        let usecs = &rem_psecs / &million;
        let psecs = &rem_psecs - &usecs * &million;

        let secs = i64::try_from(&secs)
            .map_err(|_| signal("error", vec![Value::string("Time value out of range")]))?;
        let usecs = i64::try_from(&usecs)
            .map_err(|_| signal("error", vec![Value::string("Time value out of range")]))?;
        let psecs = i64::try_from(&psecs)
            .map_err(|_| signal("error", vec![Value::string("Time value out of range")]))?;

        Ok(TimeMicros { secs, usecs, psecs })
    }

    fn to_ticks_hz(&self, hz: i64) -> Value {
        self.to_ticks_hz_integer(&Integer::from(hz))
    }

    fn to_ticks_hz_integer(&self, hz: &Integer) -> Value {
        let trillion = Integer::from(1_000_000_000_000i64);
        let total_psecs = Integer::from(self.secs) * &trillion
            + Integer::from(self.usecs) * Integer::from(1_000_000i64)
            + Integer::from(self.psecs);
        let ticks = integer_div_floor(&(total_psecs * hz), &trillion);
        Value::cons(Value::make_integer(ticks), Value::make_integer(hz.clone()))
    }

    fn less_than(self, other: TimeMicros) -> bool {
        if self.secs != other.secs {
            self.secs < other.secs
        } else if self.usecs != other.usecs {
            self.usecs < other.usecs
        } else {
            self.psecs < other.psecs
        }
    }

    fn equal(self, other: TimeMicros) -> bool {
        self.secs == other.secs && self.usecs == other.usecs && self.psecs == other.psecs
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimeInputForm {
    Scalar,
    List,
    TicksHz,
}

#[derive(Clone, Debug)]
struct ParsedTime {
    time: TimeMicros,
    hz: i64,
    form: TimeInputForm,
    exact_ticks_hz: Option<(Integer, Integer)>,
}

/// Parse a time value from a Lisp argument.
///
/// Accepts:
///   - nil            -> current time
///   - integer        -> seconds since epoch
///   - float          -> seconds since epoch (with fractional part)
///   - (TICKS . HZ)   -> modern GNU timestamp cons
///   - (HIGH LOW)     -> high*65536 + low seconds, 0 usecs
///   - (HIGH LOW USEC)       -> with microseconds
///   - (HIGH LOW USEC PSEC)  -> with picoseconds
fn parse_time(val: &Value) -> Result<TimeMicros, Flow> {
    Ok(parse_time_detailed(val)?.time)
}

fn parse_time_detailed(val: &Value) -> Result<ParsedTime, Flow> {
    use crate::emacs_core::value::VecLikeType;
    // Bignum seconds-since-epoch values get truncated to i64;
    // Emacs's GNU encoding of large times uses (HIGH LOW) cons
    // pairs anyway, so a bignum here usually only occurs for
    // tests that compute (1+ most-positive-fixnum) etc.
    if let ValueKind::Veclike(VecLikeType::Bignum) = val.kind() {
        let f = f64::rounding_from(val.as_bignum().unwrap(), RoundingMode::Nearest).0;
        return Ok(ParsedTime {
            time: TimeMicros {
                secs: f as i64,
                usecs: 0,
                psecs: 0,
            },
            hz: 1,
            form: TimeInputForm::Scalar,
            exact_ticks_hz: None,
        });
    }
    match val.kind() {
        ValueKind::Nil => Ok(ParsedTime {
            time: TimeMicros::now(),
            hz: 1_000_000_000_000,
            form: TimeInputForm::List,
            exact_ticks_hz: None,
        }),
        ValueKind::Fixnum(n) => Ok(ParsedTime {
            time: TimeMicros {
                secs: n,
                usecs: 0,
                psecs: 0,
            },
            hz: 1,
            form: TimeInputForm::Scalar,
            exact_ticks_hz: Some((Integer::from(n), Integer::from(1))),
        }),
        ValueKind::Float => {
            let f = val.xfloat();
            if !f.is_finite() {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid time specification")],
                ));
            }
            let (ticks, hz) = float_to_exact_ticks_hz(f)?;
            let hz_i64 = i64::try_from(&hz)
                .map_err(|_| signal("error", vec![Value::string("Time value out of range")]))?;
            Ok(ParsedTime {
                time: TimeMicros::from_exact_ticks_hz(&ticks, &hz)?,
                hz: hz_i64,
                form: TimeInputForm::List,
                exact_ticks_hz: Some((ticks, hz)),
            })
        }
        ValueKind::Cons => {
            let high = val.cons_car();
            let low_or_tail = val.cons_cdr();
            if !low_or_tail.is_cons() {
                let ticks = high.as_int().ok_or_else(|| {
                    signal("wrong-type-argument", vec![Value::symbol("integerp"), high])
                })?;
                let hz = low_or_tail.as_int().ok_or_else(|| {
                    signal(
                        "wrong-type-argument",
                        vec![Value::symbol("integerp"), low_or_tail],
                    )
                })?;
                return Ok(ParsedTime {
                    time: TimeMicros::from_ticks_hz(ticks, hz)?,
                    hz,
                    form: TimeInputForm::TicksHz,
                    exact_ticks_hz: Some((Integer::from(ticks), Integer::from(hz))),
                });
            }

            let items = list_to_vec(val)
                .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("listp"), *val]))?;
            if items.len() < 2 {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("listp"), *val],
                ));
            }
            let high = items[0].as_int().ok_or_else(|| {
                signal(
                    "wrong-type-argument",
                    vec![Value::symbol("integerp"), items[0]],
                )
            })?;
            let low = items[1].as_int().ok_or_else(|| {
                signal(
                    "wrong-type-argument",
                    vec![Value::symbol("integerp"), items[1]],
                )
            })?;
            let usec = if items.len() > 2 {
                items[2].as_int().unwrap_or(0)
            } else {
                0
            };
            let psec = if items.len() > 3 {
                items[3].as_int().unwrap_or(0)
            } else {
                0
            };
            let secs = high * 65536 + low;
            Ok(ParsedTime {
                time: TimeMicros {
                    secs,
                    usecs: usec,
                    psecs: psec,
                },
                hz: time_convert_default_hz(val),
                form: TimeInputForm::List,
                exact_ticks_hz: None,
            })
        }
        other => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("numberp"), *val],
        )),
    }
}

fn float_to_exact_ticks_hz(f: f64) -> Result<(Integer, Integer), Flow> {
    if f == 0.0 {
        return Ok((Integer::from(0), Integer::from(1)));
    }

    let bits = f.to_bits();
    let sign = if bits >> 63 == 0 { 1i128 } else { -1i128 };
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, unbiased_exponent) = if exponent_bits == 0 {
        (Integer::from(fraction), -1022)
    } else {
        (Integer::from((1u64 << 52) | fraction), exponent_bits - 1023)
    };
    let scale = (52 - unbiased_exponent).max(0);

    let mut ticks = if unbiased_exponent >= 52 {
        let shift = (unbiased_exponent - 52) as u32;
        mantissa << shift
    } else {
        mantissa
    };
    if sign < 0 {
        ticks = -ticks;
    }

    let hz = Integer::from(1) << (scale as u32);
    Ok((ticks, hz))
}

fn integer_div_floor(n: &Integer, d: &Integer) -> Integer {
    let q = n / d;
    let r = n - &q * d;
    if r != 0 && n < &Integer::from(0) {
        q - Integer::from(1)
    } else {
        q
    }
}

fn time_hz_gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.abs()
}

fn time_hz_lcm(a: i64, b: i64) -> i64 {
    if a <= 0 || b <= 0 {
        return 1;
    }
    let gcd = time_hz_gcd(a, b);
    a.saturating_div(gcd).saturating_mul(b)
}

fn time_arithmetic_hz(a: &ParsedTime, b: &ParsedTime) -> i64 {
    time_hz_lcm(a.hz, b.hz)
}

fn time_arithmetic_result(result: TimeMicros, a: ParsedTime, b: ParsedTime) -> Value {
    let hz = time_arithmetic_hz(&a, &b);
    if hz == 1 {
        return Value::make_int(result.secs);
    }

    if a.form == TimeInputForm::TicksHz || b.form == TimeInputForm::TicksHz {
        return result.to_ticks_hz(hz);
    }

    result.to_list()
}

// ---------------------------------------------------------------------------
// Date/time breakdown helpers (UTC only, no chrono)
// ---------------------------------------------------------------------------

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(month: i64, year: i64) -> i64 {
    match month {
        1 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 30,
    }
}

fn days_in_year(year: i64) -> i64 {
    if is_leap_year(year) { 366 } else { 365 }
}

/// Decoded time in UTC: (sec min hour day month year dow dst utcoff).
struct DecodedTime {
    sec: i64,
    min: i64,
    hour: i64,
    day: i64,   // 1-based
    month: i64, // 1-based
    year: i64,
    dow: i64, // 0=Sunday, 1=Monday, ..., 6=Saturday
}

struct ZonedDecodedTime {
    time: DecodedTime,
    dst: Value,
    utcoff: i64,
}

/// Break epoch seconds into UTC date/time components.
fn decode_epoch_secs(total_secs: i64) -> DecodedTime {
    // Handle the time-of-day part
    let mut days = total_secs.div_euclid(86400);
    let day_secs = total_secs.rem_euclid(86400);

    let sec = day_secs % 60;
    let min = (day_secs / 60) % 60;
    let hour = day_secs / 3600;

    // Day of week: epoch (1970-01-01) was Thursday (4).
    // dow: 0=Sunday
    let dow = ((days % 7) + 4).rem_euclid(7);

    // Compute year, month, day from days since epoch.
    let mut year: i64 = 1970;
    if days >= 0 {
        loop {
            let dy = days_in_year(year);
            if days < dy {
                break;
            }
            days -= dy;
            year += 1;
        }
    } else {
        loop {
            year -= 1;
            let dy = days_in_year(year);
            days += dy;
            if days >= 0 {
                break;
            }
        }
    }

    // Now `days` is day-of-year (0-based).
    let mut month: i64 = 1;
    loop {
        let dm = days_in_month(month, year);
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
        if month > 12 {
            break;
        }
    }
    let day = days + 1; // 1-based

    DecodedTime {
        sec,
        min,
        hour,
        day,
        month,
        year,
        dow,
    }
}

/// Encode date/time components to epoch seconds (UTC).
fn encode_to_epoch_secs(sec: i64, min: i64, hour: i64, day: i64, month: i64, year: i64) -> i64 {
    // Count days from epoch (1970-01-01) to the given date.
    let mut total_days: i64 = 0;

    if year >= 1970 {
        for y in 1970..year {
            total_days += days_in_year(y);
        }
    } else {
        for y in year..1970 {
            total_days -= days_in_year(y);
        }
    }

    // Add days for months in the target year.
    for m in 1..month {
        total_days += days_in_month(m, year);
    }

    // Add days within month (day is 1-based).
    total_days += day - 1;

    total_days * 86400 + hour * 3600 + min * 60 + sec
}

// ---------------------------------------------------------------------------
// Day/month name tables
// ---------------------------------------------------------------------------

const DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

#[derive(Clone, Debug)]
enum ZoneRule {
    Local,
    Utc,
    FixedOffset(i64),
    FixedNamedOffset(i64, String),
    TzString(String),
}

thread_local! {
    static TIME_ZONE_RULE: RefCell<ZoneRule> = RefCell::new(ZoneRule::Local);
}

/// Reset timezone rule to default (called from Context::new).
pub(crate) fn reset_timefns_thread_locals() {
    TIME_ZONE_RULE.with(|slot| *slot.borrow_mut() = ZoneRule::Local);
}

fn tz_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn invalid_time_zone_spec(spec: &Value) -> Flow {
    signal(
        "error",
        vec![Value::string("Invalid time zone specification"), *spec],
    )
}

fn format_fixed_offset_name(offset_secs: i64) -> String {
    if offset_secs == 0 {
        return "GMT".to_string();
    }
    let sign = if offset_secs < 0 { '-' } else { '+' };
    let abs_secs = offset_secs.abs();
    if abs_secs % 3600 == 0 {
        format!("{}{abs_hours:02}", sign, abs_hours = abs_secs / 3600)
    } else if abs_secs % 60 == 0 {
        let total_minutes = abs_secs / 60;
        format!(
            "{}{hours:02}{mins:02}",
            sign,
            hours = total_minutes / 60,
            mins = total_minutes % 60
        )
    } else {
        format!(
            "{}{hours:02}{mins:02}{secs:02}",
            sign,
            hours = abs_secs / 3600,
            mins = (abs_secs % 3600) / 60,
            secs = abs_secs % 60
        )
    }
}

#[cfg(unix)]
fn local_offset_name_at_epoch(epoch_secs: i64) -> (i64, String) {
    let mut time_val: libc::time_t = epoch_secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let tm_ptr = unsafe { libc::localtime_r(&mut time_val as *mut _, &mut tm as *mut _) };
    if tm_ptr.is_null() {
        return (0, "UTC".to_string());
    }
    let offset = tm.tm_gmtoff as i64;
    let name = if tm.tm_zone.is_null() {
        format_fixed_offset_name(offset)
    } else {
        unsafe { CStr::from_ptr(tm.tm_zone) }
            .to_string_lossy()
            .into_owned()
    };
    (offset, name)
}

#[cfg(not(unix))]
fn local_offset_name_at_epoch(_epoch_secs: i64) -> (i64, String) {
    (0, "UTC".to_string())
}

#[cfg(unix)]
fn local_decoded_time_at_epoch(epoch_secs: i64) -> Result<ZonedDecodedTime, Flow> {
    let mut time_val: libc::time_t = epoch_secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let tm_ptr = unsafe { libc::localtime_r(&mut time_val as *mut _, &mut tm as *mut _) };
    if tm_ptr.is_null() {
        return Err(signal(
            "error",
            vec![Value::string("Invalid time specification")],
        ));
    }

    let dst = match tm.tm_isdst {
        n if n < 0 => Value::fixnum(-1),
        0 => Value::NIL,
        _ => Value::T,
    };

    Ok(ZonedDecodedTime {
        time: DecodedTime {
            sec: tm.tm_sec as i64,
            min: tm.tm_min as i64,
            hour: tm.tm_hour as i64,
            day: tm.tm_mday as i64,
            month: tm.tm_mon as i64 + 1,
            year: tm.tm_year as i64 + 1900,
            dow: tm.tm_wday as i64,
        },
        utcoff: tm.tm_gmtoff as i64,
        dst,
    })
}

#[cfg(not(unix))]
fn local_decoded_time_at_epoch(epoch_secs: i64) -> Result<ZonedDecodedTime, Flow> {
    let time = decode_epoch_secs(epoch_secs);
    Ok(ZonedDecodedTime {
        time,
        dst: Value::NIL,
        utcoff: 0,
    })
}

#[cfg(unix)]
fn refresh_tz_env() {
    unsafe extern "C" {
        fn tzset();
    }
    unsafe {
        tzset();
    }
}

#[cfg(not(unix))]
fn refresh_tz_env() {}

struct ScopedTzEnv {
    previous: Option<OsString>,
}

impl ScopedTzEnv {
    fn new(spec: Option<&str>) -> Self {
        let previous = std::env::var_os("TZ");
        match spec {
            Some(v) => unsafe { std::env::set_var("TZ", v) },
            None => unsafe { std::env::remove_var("TZ") },
        }
        refresh_tz_env();
        Self { previous }
    }
}

impl Drop for ScopedTzEnv {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => unsafe { std::env::set_var("TZ", v) },
            None => unsafe { std::env::remove_var("TZ") },
        }
        refresh_tz_env();
    }
}

fn with_tz_env<T>(spec: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _lock = tz_env_lock().lock().expect("time zone env lock poisoned");
    let _guard = ScopedTzEnv::new(spec);
    f()
}

fn parse_zone_rule(zone: &Value) -> Result<ZoneRule, Flow> {
    match zone.kind() {
        ValueKind::Nil => Ok(ZoneRule::Local),
        ValueKind::T => Ok(ZoneRule::Utc),
        ValueKind::Symbol(_) => match TimeZoneSymbol::from_value(zone) {
            Some(TimeZoneSymbol::Wall) => Ok(ZoneRule::Local),
            None => Err(invalid_time_zone_spec(zone)),
        },
        ValueKind::Fixnum(n) => Ok(ZoneRule::FixedOffset(n)),
        ValueKind::String => Ok(ZoneRule::TzString(
            zone.as_runtime_string_owned()
                .expect("ValueKind::String must carry LispString payload"),
        )),
        ValueKind::Cons => {
            let items = list_to_vec(zone).ok_or_else(|| invalid_time_zone_spec(zone))?;
            if items.len() != 2 {
                return Err(invalid_time_zone_spec(zone));
            }
            let Some(offset) = items[0].as_int() else {
                return Err(invalid_time_zone_spec(zone));
            };
            let name = match items[1].kind() {
                ValueKind::String => items[1]
                    .as_runtime_string_owned()
                    .expect("ValueKind::String must carry LispString payload"),
                ValueKind::Symbol(id) => resolve_sym(id).to_owned(),
                _ => return Err(invalid_time_zone_spec(zone)),
            };
            Ok(ZoneRule::FixedNamedOffset(offset, name))
        }
        _ => Err(invalid_time_zone_spec(zone)),
    }
}

fn effective_zone_rule(zone: Option<&Value>) -> Result<ZoneRule, Flow> {
    match zone {
        None => TIME_ZONE_RULE.with(|slot| Ok(slot.borrow().clone())),
        Some(value) if value.is_nil() => TIME_ZONE_RULE.with(|slot| Ok(slot.borrow().clone())),
        Some(value) => parse_zone_rule(value),
    }
}

fn zone_rule_to_offset_name(rule: &ZoneRule, epoch_secs: i64) -> (i64, String) {
    match rule {
        ZoneRule::Local => local_offset_name_at_epoch(epoch_secs),
        ZoneRule::Utc => (0, "GMT".to_string()),
        ZoneRule::FixedOffset(offset) => (*offset, format_fixed_offset_name(*offset)),
        ZoneRule::FixedNamedOffset(offset, name) => (*offset, name.clone()),
        ZoneRule::TzString(spec) => {
            with_tz_env(Some(spec), || local_offset_name_at_epoch(epoch_secs))
        }
    }
}

pub(crate) fn zone_offset_name_for_time(
    zone: Option<&Value>,
    epoch_secs: i64,
) -> Result<(i64, String), Flow> {
    let rule = effective_zone_rule(zone)?;
    Ok(zone_rule_to_offset_name(&rule, epoch_secs))
}

fn decode_time_for_zone(rule: &ZoneRule, epoch_secs: i64) -> Result<ZonedDecodedTime, Flow> {
    match rule {
        ZoneRule::Local => local_decoded_time_at_epoch(epoch_secs),
        ZoneRule::Utc => Ok(ZonedDecodedTime {
            time: decode_epoch_secs(epoch_secs),
            dst: Value::NIL,
            utcoff: 0,
        }),
        ZoneRule::FixedOffset(offset) | ZoneRule::FixedNamedOffset(offset, _) => {
            Ok(ZonedDecodedTime {
                time: decode_epoch_secs(epoch_secs.saturating_add(*offset)),
                dst: Value::NIL,
                utcoff: *offset,
            })
        }
        ZoneRule::TzString(spec) => {
            with_tz_env(Some(spec), || local_decoded_time_at_epoch(epoch_secs))
        }
    }
}

fn decode_time_form_hz(time_arg: Option<&Value>) -> i64 {
    let Some(time_arg) = time_arg else {
        return 1_000_000;
    };

    match time_arg.kind() {
        ValueKind::Nil => 1_000_000,
        ValueKind::Float => 1_000_000,
        ValueKind::Cons => {
            if let Some(items) = list_to_vec(time_arg) {
                if items.len() >= 4 {
                    1_000_000_000_000
                } else if items.len() >= 3 {
                    1_000_000
                } else {
                    1
                }
            } else {
                1
            }
        }
        _ => 1,
    }
}

fn decode_time_second_value(
    time_arg: Option<&Value>,
    tm: TimeMicros,
    decoded_sec: i64,
    form: Option<&Value>,
) -> Value {
    if !matches!(form.map(|value| value.kind()), Some(ValueKind::T)) {
        return Value::fixnum(decoded_sec);
    }

    match decode_time_form_hz(time_arg) {
        1 => Value::fixnum(decoded_sec),
        1_000_000 => Value::cons(
            Value::make_int(decoded_sec * 1_000_000 + tm.usecs),
            Value::fixnum(1_000_000),
        ),
        1_000_000_000_000 => Value::cons(
            Value::make_int(decoded_sec * 1_000_000_000_000 + tm.usecs * 1_000_000 + tm.psecs),
            Value::make_int(1_000_000_000_000),
        ),
        _ => Value::fixnum(decoded_sec),
    }
}

fn time_convert_default_hz(value: &Value) -> i64 {
    match value.kind() {
        ValueKind::Fixnum(_) => 1,
        ValueKind::Float => 1_000_000,
        ValueKind::Cons => {
            let tail = value.cons_cdr();
            if !tail.is_cons() {
                return tail.as_int().filter(|hz| *hz > 0).unwrap_or(1);
            }
            if let Some(items) = list_to_vec(value) {
                match items.len() {
                    0 | 1 | 2 => 1,
                    3 => 1_000_000,
                    _ => 1_000_000_000_000,
                }
            } else {
                1
            }
        }
        _ => 1,
    }
}

fn require_integer_component(value: &Value) -> Result<i64, Flow> {
    value.as_int().ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("integerp"), *value],
        )
    })
}

fn encode_time_zone_offset(zone: &Value, approx_epoch_secs: i64) -> Result<i64, Flow> {
    let rule = effective_zone_rule(Some(zone))?;
    let initial = zone_rule_to_offset_name(&rule, approx_epoch_secs).0;
    Ok(match rule {
        ZoneRule::Local | ZoneRule::TzString(_) => {
            let adjusted_epoch = approx_epoch_secs - initial;
            zone_rule_to_offset_name(&rule, adjusted_epoch).0
        }
        _ => initial,
    })
}

// ---------------------------------------------------------------------------
// Pure builtins
// ---------------------------------------------------------------------------

fn invalid_time_frequency_error() -> Flow {
    signal("error", vec![Value::string("Invalid time frequency")])
}

fn current_time_list_enabled(eval: &Context) -> Result<bool, Flow> {
    eval.eval_symbol_by_id(intern("current-time-list"))
        .map(|value| value.is_truthy())
}

fn parse_time_convert_form(form: &Value, current_time_list: bool) -> Result<TimeConvertForm, Flow> {
    match form.kind() {
        ValueKind::Nil => {
            if current_time_list {
                Ok(TimeConvertForm::List)
            } else {
                Ok(TimeConvertForm::InputHz)
            }
        }
        ValueKind::T => Ok(TimeConvertForm::InputHz),
        ValueKind::Fixnum(hz) if hz > 0 => Ok(TimeConvertForm::ExplicitHz(Integer::from(hz))),
        ValueKind::Fixnum(_) => Err(invalid_time_frequency_error()),
        ValueKind::Veclike(VecLikeType::Bignum) => {
            let hz = form
                .as_bignum()
                .expect("ValueKind::Bignum must carry Integer payload")
                .clone();
            if hz > Integer::from(0) {
                Ok(TimeConvertForm::ExplicitHz(hz))
            } else {
                Err(invalid_time_frequency_error())
            }
        }
        ValueKind::Symbol(id) => match resolve_sym(id).parse::<TimeConvertSymbolForm>().ok() {
            Some(TimeConvertSymbolForm::List) => Ok(TimeConvertForm::List),
            Some(TimeConvertSymbolForm::Integer) => Ok(TimeConvertForm::Integer),
            None => Err(invalid_time_frequency_error()),
        },
        _ => Err(invalid_time_frequency_error()),
    }
}

fn current_time_value(current_time_list: bool) -> Value {
    let now = TimeMicros::now();
    if current_time_list {
        now.to_list()
    } else {
        now.to_ticks_hz(1_000_000_000)
    }
}

/// `(current-time)` -> `(HIGH LOW USEC PSEC)` or `(TICKS . HZ)`.
pub(crate) fn builtin_current_time(args: Vec<Value>) -> EvalResult {
    expect_args("current-time", &args, 0)?;
    Ok(current_time_value(true))
}

pub(crate) fn builtin_current_time_in_context(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args("current-time", &args, 0)?;
    Ok(current_time_value(current_time_list_enabled(eval)?))
}

/// `(float-time &optional TIME)` -> float seconds since epoch.
pub(crate) fn builtin_float_time(args: Vec<Value>) -> EvalResult {
    expect_min_max_args("float-time", &args, 0, 1)?;
    let tm = if args.is_empty() || args[0].is_nil() {
        TimeMicros::now()
    } else {
        parse_time(&args[0])?
    };
    Ok(Value::make_float(tm.to_float()))
}

/// `(time-add A B)` -> integer seconds or `(TICKS . HZ)`
pub(crate) fn builtin_time_add(args: Vec<Value>) -> EvalResult {
    expect_args("time-add", &args, 2)?;
    let a = parse_time_detailed(&args[0])?;
    let b = parse_time_detailed(&args[1])?;
    Ok(time_arithmetic_result(a.time.add(b.time), a, b))
}

/// `(time-subtract A B)` -> integer seconds or `(TICKS . HZ)`
pub(crate) fn builtin_time_subtract(args: Vec<Value>) -> EvalResult {
    expect_args("time-subtract", &args, 2)?;
    let a = parse_time_detailed(&args[0])?;
    let b = parse_time_detailed(&args[1])?;
    Ok(time_arithmetic_result(a.time.sub(b.time), a, b))
}

/// `(time-less-p A B)` -> t or nil
pub(crate) fn builtin_time_less_p(args: Vec<Value>) -> EvalResult {
    expect_args("time-less-p", &args, 2)?;
    let a = parse_time(&args[0])?;
    let b = parse_time(&args[1])?;
    Ok(Value::bool_val(a.less_than(b)))
}

/// `(time-equal-p A B)` -> t or nil
pub(crate) fn builtin_time_equal_p(args: Vec<Value>) -> EvalResult {
    expect_args("time-equal-p", &args, 2)?;
    // GNU timefns.c:time_cmp first treats identical Lisp objects as equal,
    // so `(time-equal-p nil nil)' and other `eq' inputs avoid validation.
    if args[0] == args[1] {
        return Ok(Value::T);
    }
    // GNU Ftime_equal_p also avoids interpreting one nil as "current time"
    // when the other argument is non-nil.
    if args[0].is_nil() || args[1].is_nil() {
        return Ok(Value::NIL);
    }
    let a = parse_time(&args[0])?;
    let b = parse_time(&args[1])?;
    Ok(Value::bool_val(a.equal(b)))
}

/// `(current-time-string &optional TIME ZONE)` -> human-readable string.
///
/// Returns a string like `"Mon Jan  2 15:04:05 2006"`.
pub(crate) fn builtin_current_time_string(args: Vec<Value>) -> EvalResult {
    expect_min_max_args("current-time-string", &args, 0, 2)?;
    let tm = if args.is_empty() || args[0].is_nil() {
        TimeMicros::now()
    } else {
        parse_time(&args[0])?
    };
    let (offset_secs, _) = zone_offset_name_for_time(args.get(1), tm.secs)?;
    let dt = decode_epoch_secs(tm.secs.saturating_add(offset_secs));

    // Format: "Dow Mon DD HH:MM:SS YYYY"
    // Day of month is right-justified in a 2-char field (space-padded).
    let s = format!(
        "{} {} {:2} {:02}:{:02}:{:02} {}",
        DAY_NAMES[dt.dow as usize],
        MONTH_NAMES[(dt.month - 1) as usize],
        dt.day,
        dt.hour,
        dt.min,
        dt.sec,
        dt.year,
    );
    Ok(Value::string(s))
}

/// `(current-time-zone &optional TIME ZONE)` -> `(OFFSET NAME)`.
pub(crate) fn builtin_current_time_zone(args: Vec<Value>) -> EvalResult {
    expect_min_max_args("current-time-zone", &args, 0, 2)?;
    let tm = if args.is_empty() || args[0].is_nil() {
        TimeMicros::now()
    } else {
        parse_time(&args[0])?
    };

    let rule = effective_zone_rule(args.get(1))?;

    let (offset, name) = zone_rule_to_offset_name(&rule, tm.secs);
    Ok(Value::list(vec![
        Value::fixnum(offset),
        Value::string(name),
    ]))
}

/// `(encode-time TIME &rest OBSOLESCENT-ARGUMENTS)` -> `(HIGH LOW)`
pub(crate) fn builtin_encode_time(args: Vec<Value>) -> EvalResult {
    let (sec, min, hour, day, month, year, zone) = if args.len() == 1 {
        let items = list_to_vec(&args[0])
            .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("listp"), args[0]]))?;
        if items.len() < 6 {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("listp"), args[0]],
            ));
        }
        (
            require_integer_component(&items[0])?,
            require_integer_component(&items[1])?,
            require_integer_component(&items[2])?,
            require_integer_component(&items[3])?,
            require_integer_component(&items[4])?,
            require_integer_component(&items[5])?,
            items.get(8).copied().unwrap_or(Value::NIL),
        )
    } else if args.len() < 6 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("encode-time"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    } else {
        (
            require_integer_component(&args[0])?,
            require_integer_component(&args[1])?,
            require_integer_component(&args[2])?,
            require_integer_component(&args[3])?,
            require_integer_component(&args[4])?,
            require_integer_component(&args[5])?,
            if args.len() > 6 {
                args.last().copied().unwrap_or(Value::NIL)
            } else {
                Value::NIL
            },
        )
    };

    let local_secs = encode_to_epoch_secs(sec, min, hour, day, month, year);
    let zone_offset = encode_time_zone_offset(&zone, local_secs)?;
    let total_secs = local_secs - zone_offset;
    let high = total_secs >> 16;
    let low = total_secs & 0xFFFF;
    Ok(Value::list(vec![Value::fixnum(high), Value::fixnum(low)]))
}

/// `(decode-time &optional TIME ZONE FORM)`
/// -> `(SECONDS MINUTES HOURS DAY MONTH YEAR DOW DST UTCOFF)`
pub(crate) fn builtin_decode_time(args: Vec<Value>) -> EvalResult {
    expect_min_max_args("decode-time", &args, 0, 3)?;
    let tm = if args.is_empty() || args[0].is_nil() {
        TimeMicros::now()
    } else {
        parse_time(&args[0])?
    };
    let rule = effective_zone_rule(args.get(1))?;
    let decoded = decode_time_for_zone(&rule, tm.secs)?;
    let dt = decoded.time;
    let sec = decode_time_second_value(args.first(), tm, dt.sec, args.get(2));
    Ok(Value::list(vec![
        sec,
        Value::fixnum(dt.min),
        Value::fixnum(dt.hour),
        Value::fixnum(dt.day),
        Value::fixnum(dt.month),
        Value::fixnum(dt.year),
        Value::fixnum(dt.dow),
        decoded.dst,
        Value::fixnum(decoded.utcoff),
    ]))
}

/// `(time-convert TIME &optional FORM)`
///
/// FORM controls the output format:
///   - nil             -> `current-time-list` dependent default
///   - `list`          -> `(HIGH LOW USEC PSEC)`
///   - `integer`       -> integer seconds
///   - `t`             -> `(TICKS . HZ)` (highest precision cons cell)
pub(crate) fn builtin_time_convert(args: Vec<Value>) -> EvalResult {
    builtin_time_convert_with_current_time_list(args, true)
}

pub(crate) fn builtin_time_convert_in_context(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    let current_time_list = current_time_list_enabled(eval)?;
    builtin_time_convert_with_current_time_list(args, current_time_list)
}

fn builtin_time_convert_with_current_time_list(
    args: Vec<Value>,
    current_time_list: bool,
) -> EvalResult {
    expect_min_max_args("time-convert", &args, 1, 2)?;
    let parsed = parse_time_detailed(&args[0])?;
    let tm = parsed.time;

    let form = if args.len() > 1 {
        &args[1]
    } else {
        &Value::NIL
    };

    match parse_time_convert_form(form, current_time_list)? {
        TimeConvertForm::List => Ok(tm.to_list()),
        TimeConvertForm::Integer => Ok(Value::fixnum(tm.secs)),
        TimeConvertForm::InputHz => {
            // GNU's default timestamp representation is (TICKS . HZ).
            let hz = parsed.hz;
            if let Some((ticks, exact_hz)) = parsed.exact_ticks_hz {
                Ok(Value::cons(
                    Value::make_integer(ticks),
                    Value::make_integer(exact_hz),
                ))
            } else {
                Ok(tm.to_ticks_hz(hz))
            }
        }
        TimeConvertForm::ExplicitHz(hz) => Ok(tm.to_ticks_hz_integer(&hz)),
    }
}

/// `(set-time-zone-rule ZONE)` -> nil.
pub(crate) fn builtin_set_time_zone_rule(args: Vec<Value>) -> EvalResult {
    expect_args("set-time-zone-rule", &args, 1)?;
    let rule = parse_zone_rule(&args[0])?;
    TIME_ZONE_RULE.with(|slot| *slot.borrow_mut() = rule);
    Ok(Value::NIL)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "timefns_test.rs"]
mod tests;
