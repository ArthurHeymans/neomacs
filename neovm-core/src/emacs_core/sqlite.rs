//! SQLite database support, matching GNU Emacs's sqlite.c.
//!
//! Provides the full sqlite Elisp API surface using rusqlite as the backend.
//! Handle tracking uses thread-local storage to map integer IDs to rusqlite
//! Connection objects. For 'set mode queries, live prepared statements are
//! kept in `RESULT_SETS`, mirroring GNU's PVEC_SQLITE statement objects.

use crate::emacs_core::error::LispCondition;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;
use std::sync::atomic::{AtomicI64, Ordering};

use rusqlite::{Connection, OpenFlags, ffi};
use strum::{EnumString, IntoStaticStr};

use super::error::{EvalResult, Flow, signal};
use super::value::*;
use crate::buffer::CharPos0;
use crate::emacs_core::value::ValueKind;
use crate::heap_types::LispString;

// ---------------------------------------------------------------------------
// Thread-local handle storage
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

thread_local! {
    /// Open database connections: handle_id -> Connection.
    static DB_HANDLES: RefCell<HashMap<i64, Connection>> = RefCell::new(HashMap::new());

    /// Live result sets for 'set mode: stmt_handle_id -> sqlite3_stmt.
    static RESULT_SETS: RefCell<HashMap<i64, ResultSet>> = RefCell::new(HashMap::new());
}

/// A live SQLite result set for incremental iteration.
struct ResultSet {
    db_id: i64,
    stmt: *mut ffi::sqlite3_stmt,
    eof: bool,
}

impl Drop for ResultSet {
    fn drop(&mut self) {
        if !self.stmt.is_null() {
            unsafe {
                ffi::sqlite3_finalize(self.stmt);
            }
            self.stmt = ptr::null_mut();
        }
    }
}

impl Drop for crate::tagged::header::SqliteObj {
    fn drop(&mut self) {
        // This Drop can run during thread-local DESTRUCTION (the heap is dropped
        // when its thread exits, which drops the SqliteObj it owns). At that
        // point the `RESULT_SETS`/`DB_HANDLES` thread-locals may already be
        // destroyed, so `with` would panic with `AccessError` ("cannot access a
        // Thread Local Storage value during or after destruction") and abort the
        // process. `try_with` tolerates that — if the registry is already gone
        // there is nothing left to remove.
        if self.is_statement {
            let _ = RESULT_SETS.try_with(|h| {
                h.borrow_mut().remove(&self.id);
            });
        } else {
            let _ = DB_HANDLES.try_with(|h| {
                h.borrow_mut().remove(&self.id);
            });
        }
    }
}

/// Reset all thread-local state (called between test runs).
pub(super) fn reset_sqlite_thread_locals() {
    NEXT_HANDLE.store(1, Ordering::SeqCst);
    DB_HANDLES.with(|h| h.borrow_mut().clear());
    RESULT_SETS.with(|h| h.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// Handle helpers
// ---------------------------------------------------------------------------

/// Extract a DB handle ID from an opaque sqlite Elisp value.
fn sqlite_db_handle_id(value: &Value) -> Option<i64> {
    let obj = value.as_sqlite()?;
    (!obj.is_statement).then_some(obj.id)
}

/// Extract a statement handle ID from an opaque sqlite Elisp value.
fn sqlite_stmt_handle_id(value: &Value) -> Option<i64> {
    let obj = value.as_sqlite()?;
    obj.is_statement.then_some(obj.id)
}

/// Check if a DB handle ID refers to an open connection.
fn is_open_db(id: i64) -> bool {
    DB_HANDLES.with(|h| h.borrow().contains_key(&id))
}

/// Expect a sqlite DB handle, returning the handle ID.
fn expect_db(value: &Value) -> Result<i64, Flow> {
    let id = sqlite_db_handle_id(value).ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("sqlitep"), *value],
        )
    })?;
    if !is_open_db(id) {
        return Err(signal(
            LispCondition::SqliteError,
            vec![Value::string("Database closed")],
        ));
    }
    Ok(id)
}

/// Expect a sqlite statement handle, returning the handle ID.
fn expect_stmt(value: &Value) -> Result<i64, Flow> {
    // GNU's sqlite-next etc. accept both DB and statement objects,
    // but reject DB objects with "Invalid set object".
    if sqlite_db_handle_id(value).is_some() {
        return Err(signal(
            LispCondition::SqliteError,
            vec![Value::string("Invalid set object")],
        ));
    }
    let id = sqlite_stmt_handle_id(value).ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("sqlitep"), *value],
        )
    })?;
    if !RESULT_SETS.with(|h| h.borrow().contains_key(&id)) {
        return Err(signal(
            LispCondition::SqliteError,
            vec![Value::string("Statement closed")],
        ));
    }
    Ok(id)
}

fn make_db_handle(id: i64) -> Value {
    Value::make_sqlite(false, id)
}

fn make_stmt_handle(id: i64) -> Value {
    Value::make_sqlite(true, id)
}

fn alloc_handle_id() -> i64 {
    NEXT_HANDLE.fetch_add(1, Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn expect_strict_string(v: &Value) -> Result<String, Flow> {
    match v.kind() {
        ValueKind::String => Ok(v
            .as_lisp_string()
            .unwrap()
            .as_utf8_str()
            .unwrap_or_default()
            .to_string()),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *v],
        )),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum SqliteReturnType {
    Set,
    Full,
}

impl SqliteReturnType {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn symbol_name(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum SqliteBindSymbol {
    False,
}

impl SqliteBindSymbol {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn symbol_name(self) -> &'static str {
        self.into()
    }
}

fn value_is_false_symbol(v: &Value) -> bool {
    SqliteBindSymbol::from_value(v) == Some(SqliteBindSymbol::False)
}

// ---------------------------------------------------------------------------
// Value conversion: SQLite types <-> Elisp types
// ---------------------------------------------------------------------------

/// Convert a rusqlite ValueRef to an Elisp Value.
/// Matches GNU's row_to_value:
///   NULL → nil, INTEGER → fixnum, REAL → float, TEXT → string, BLOB → unibyte string
fn sqlite_value_to_elisp(val: rusqlite::types::ValueRef<'_>) -> Value {
    match val {
        rusqlite::types::ValueRef::Null => Value::NIL,
        rusqlite::types::ValueRef::Integer(n) => Value::fixnum(n),
        rusqlite::types::ValueRef::Real(f) => Value::make_float(f),
        rusqlite::types::ValueRef::Text(s) => {
            let text = String::from_utf8_lossy(s).into_owned();
            Value::string(&text)
        }
        rusqlite::types::ValueRef::Blob(b) => {
            let ls = LispString::from_unibyte(b.to_vec());
            Value::heap_string(ls)
        }
    }
}

/// Bind an Elisp value to a rusqlite statement parameter.
fn bind_elisp_value(
    stmt: &mut rusqlite::Statement<'_>,
    idx: usize,
    val: &Value,
) -> Result<(), Flow> {
    match val.kind() {
        ValueKind::Nil => stmt
            .raw_bind_parameter(idx, rusqlite::types::Null)
            .map_err(|e| sqlite_err(&e.to_string())),
        ValueKind::T => stmt
            .raw_bind_parameter(idx, 1i64)
            .map_err(|e| sqlite_err(&e.to_string())),
        ValueKind::Symbol(_) if value_is_false_symbol(val) => stmt
            .raw_bind_parameter(idx, 0i64)
            .map_err(|e| sqlite_err(&e.to_string())),
        ValueKind::Fixnum(n) => stmt
            .raw_bind_parameter(idx, n)
            .map_err(|e| sqlite_err(&e.to_string())),
        ValueKind::Veclike(crate::tagged::header::VecLikeType::Bignum) => {
            let Some(n) = val.as_bignum().and_then(|n| i64::try_from(n).ok()) else {
                return Err(sqlite_err("bignum value out of range"));
            };
            stmt.raw_bind_parameter(idx, n)
                .map_err(|e| sqlite_err(&e.to_string()))
        }
        ValueKind::Float => {
            let f = val.xfloat();
            stmt.raw_bind_parameter(idx, f)
                .map_err(|e| sqlite_err(&e.to_string()))
        }
        ValueKind::String => {
            let s = val.as_lisp_string().unwrap();
            let coding_system =
                get_string_text_properties_table_for_value(*val).and_then(|table| {
                    table.get_property_at_char_pos(CharPos0::ZERO, Value::symbol("coding-system"))
                });
            let blob = coding_system.is_some_and(|coding| coding.is_symbol_named("binary"));
            if blob {
                if s.is_multibyte() {
                    return Err(sqlite_err("BLOB values must be unibyte"));
                }
                stmt.raw_bind_parameter(idx, s.as_bytes())
                    .map_err(|e| sqlite_err(&e.to_string()))
            } else {
                let text = s
                    .as_utf8_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| String::from_utf8_lossy(s.as_bytes()).into_owned());
                stmt.raw_bind_parameter(idx, text)
                    .map_err(|e| sqlite_err(&e.to_string()))
            }
        }
        _ => Err(sqlite_err("invalid argument")),
    }
}

unsafe fn sqlite_errmsg_for_db(db: *mut ffi::sqlite3) -> String {
    if db.is_null() {
        return "sqlite error".to_string();
    }
    let msg = unsafe { ffi::sqlite3_errmsg(db) };
    if msg.is_null() {
        "sqlite error".to_string()
    } else {
        unsafe { std::ffi::CStr::from_ptr(msg) }
            .to_string_lossy()
            .into_owned()
    }
}

unsafe fn bind_raw_value(
    db: *mut ffi::sqlite3,
    stmt: *mut ffi::sqlite3_stmt,
    idx: i32,
    val: &Value,
) -> Result<(), Flow> {
    let ret = match val.kind() {
        ValueKind::Nil => unsafe { ffi::sqlite3_bind_null(stmt, idx) },
        ValueKind::T => unsafe { ffi::sqlite3_bind_int(stmt, idx, 1) },
        ValueKind::Symbol(_) if value_is_false_symbol(val) => unsafe {
            ffi::sqlite3_bind_int(stmt, idx, 0)
        },
        ValueKind::Fixnum(n) => unsafe { ffi::sqlite3_bind_int64(stmt, idx, n) },
        ValueKind::Veclike(crate::tagged::header::VecLikeType::Bignum) => {
            let Some(n) = val.as_bignum().and_then(|n| i64::try_from(n).ok()) else {
                return Err(sqlite_err("bignum value out of range"));
            };
            unsafe { ffi::sqlite3_bind_int64(stmt, idx, n) }
        }
        ValueKind::Float => unsafe { ffi::sqlite3_bind_double(stmt, idx, val.xfloat()) },
        ValueKind::String => {
            let s = val.as_lisp_string().unwrap();
            let coding_system =
                get_string_text_properties_table_for_value(*val).and_then(|table| {
                    table.get_property_at_char_pos(CharPos0::ZERO, Value::symbol("coding-system"))
                });
            let blob = coding_system.is_some_and(|coding| coding.is_symbol_named("binary"));
            if blob {
                if s.is_multibyte() {
                    return Err(sqlite_err("BLOB values must be unibyte"));
                }
                unsafe {
                    ffi::sqlite3_bind_blob(
                        stmt,
                        idx,
                        s.as_bytes().as_ptr().cast(),
                        s.sbytes() as i32,
                        ffi::SQLITE_TRANSIENT(),
                    )
                }
            } else {
                unsafe {
                    ffi::sqlite3_bind_text(
                        stmt,
                        idx,
                        s.as_bytes().as_ptr().cast(),
                        s.sbytes() as i32,
                        ffi::SQLITE_TRANSIENT(),
                    )
                }
            }
        }
        _ => return Err(sqlite_err("invalid argument")),
    };
    if ret == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(sqlite_err(&unsafe { sqlite_errmsg_for_db(db) }))
    }
}

unsafe fn bind_raw_values(
    db: *mut ffi::sqlite3,
    stmt: *mut ffi::sqlite3_stmt,
    values: &Value,
) -> Result<(), Flow> {
    if values.is_nil() {
        return Ok(());
    }
    let items: Vec<Value> = match values.kind() {
        ValueKind::Cons => super::value::list_to_vec(values).ok_or_else(|| {
            signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("listp"), *values],
            )
        })?,
        ValueKind::Veclike(crate::tagged::header::VecLikeType::Vector) => {
            values.as_vector_data().unwrap().to_vec()
        }
        _ => {
            return Err(signal(
                LispCondition::SqliteError,
                vec![Value::string("VALUES must be a list or a vector")],
            ));
        }
    };
    unsafe {
        ffi::sqlite3_reset(stmt);
    }
    for (i, val) in items.iter().enumerate() {
        unsafe { bind_raw_value(db, stmt, i as i32 + 1, val)? };
    }
    Ok(())
}

unsafe fn raw_row_to_value(stmt: *mut ffi::sqlite3_stmt) -> Value {
    let len = unsafe { ffi::sqlite3_column_count(stmt) };
    let mut row = Vec::with_capacity(len as usize);
    for col in 0..len {
        let v = match unsafe { ffi::sqlite3_column_type(stmt, col) } {
            ffi::SQLITE_INTEGER => Value::make_int(unsafe { ffi::sqlite3_column_int64(stmt, col) }),
            ffi::SQLITE_FLOAT => {
                Value::make_float(unsafe { ffi::sqlite3_column_double(stmt, col) })
            }
            ffi::SQLITE_BLOB => {
                let len = unsafe { ffi::sqlite3_column_bytes(stmt, col) };
                let ptr = unsafe { ffi::sqlite3_column_blob(stmt, col) };
                let bytes = if ptr.is_null() || len <= 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize) }.to_vec()
                };
                Value::heap_string(LispString::from_unibyte(bytes))
            }
            ffi::SQLITE_TEXT => {
                let len = unsafe { ffi::sqlite3_column_bytes(stmt, col) };
                let ptr = unsafe { ffi::sqlite3_column_text(stmt, col) };
                let bytes = if ptr.is_null() || len <= 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize) }.to_vec()
                };
                let text = String::from_utf8_lossy(&bytes);
                Value::multibyte_string(text.into_owned())
            }
            ffi::SQLITE_NULL => Value::NIL,
            _ => Value::NIL,
        };
        row.push(v);
    }
    Value::list(row)
}

/// Bind a list or vector of values to statement parameters.
fn bind_values(stmt: &mut rusqlite::Statement<'_>, values: &Value) -> Result<(), Flow> {
    if values.is_nil() {
        return Ok(());
    }

    let items: Vec<Value> = match values.kind() {
        ValueKind::Cons => super::value::list_to_vec(values).ok_or_else(|| {
            signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("listp"), *values],
            )
        })?,
        ValueKind::Veclike(crate::tagged::header::VecLikeType::Vector) => {
            values.as_vector_data().unwrap().to_vec()
        }
        _ => {
            return Err(sqlite_err("VALUES must be a list or a vector"));
        }
    };

    for (i, val) in items.iter().enumerate() {
        bind_elisp_value(stmt, i + 1, val)?;
    }
    Ok(())
}

fn sqlite_err(msg: &str) -> Flow {
    signal(LispCondition::SqliteError, vec![Value::string(msg)])
}

fn check_rusqlite(err: rusqlite::Error) -> Flow {
    match &err {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseLocked,
                ..
            },
            _,
        )
        | rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy,
                ..
            },
            _,
        ) => {
            let msg = err.to_string();
            signal(LispCondition::SqliteLockedError, vec![Value::string(&msg)])
        }
        _ => {
            let msg = err.to_string();
            signal(LispCondition::SqliteError, vec![Value::string(&msg)])
        }
    }
}

// ---------------------------------------------------------------------------
// Builtin functions
// ---------------------------------------------------------------------------

/// Lisp-visible SQLite support follows GNU's HAVE_SQLITE3 build option.
///
/// The native Rust implementation stays compiled for internal coverage, but
/// the public Lisp API must expose the same configured surface as the GNU
/// oracle binary.  That binary was built without HAVE_SQLITE3, so only
/// `sqlitep' and `sqlite-available-p' are registered and both report no
/// SQLite object/support.
pub(crate) const SQLITE3_LISP_API_AVAILABLE: bool = false;

/// (sqlite-available-p) → t or nil
pub(crate) fn builtin_sqlite_available_p(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-available-p", &args, 0)?;
    Ok(Value::bool_val(SQLITE3_LISP_API_AVAILABLE))
}

/// (sqlite-version) → version string
pub(crate) fn builtin_sqlite_version(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-version", &args, 0)?;
    Ok(Value::string(rusqlite::version()))
}

/// (sqlitep OBJECT) → t or nil
pub(crate) fn builtin_sqlitep(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlitep", &args, 1)?;
    Ok(Value::bool_val(
        SQLITE3_LISP_API_AVAILABLE && args[0].is_sqlite(),
    ))
}

/// (sqlite-open &optional FILE READONLY DISABLE-URI) → db-handle
pub(crate) fn builtin_sqlite_open(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args_range("sqlite-open", &args, 0, 3)?;

    let file = args
        .first()
        .and_then(|v| if v.is_nil() { None } else { Some(v) });
    let readonly = args.get(1).is_none_or(|v| v.is_truthy());
    let disable_uri = args.get(2).is_some_and(|v| v.is_truthy());

    let conn = match file {
        None => Connection::open_in_memory().map_err(check_rusqlite)?,
        Some(v) => {
            let path = expect_strict_string(v)?;
            let mut flags = if readonly {
                OpenFlags::SQLITE_OPEN_READ_ONLY
            } else {
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
            };
            if !disable_uri {
                flags |= OpenFlags::SQLITE_OPEN_URI;
            }
            Connection::open_with_flags(&path, flags).map_err(check_rusqlite)?
        }
    };

    let id = alloc_handle_id();
    DB_HANDLES.with(|h| h.borrow_mut().insert(id, conn));
    Ok(make_db_handle(id))
}

/// (sqlite-close DB) → t
pub(crate) fn builtin_sqlite_close(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-close", &args, 1)?;
    let id = expect_db(&args[0])?;
    DB_HANDLES.with(|h| h.borrow_mut().remove(&id));
    Ok(Value::T)
}

/// (sqlite-execute DB QUERY &optional VALUES) → affected-rows or result rows
pub(crate) fn builtin_sqlite_execute(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args_range("sqlite-execute", &args, 2, 3)?;
    let id = expect_db(&args[0])?;
    let sql = expect_strict_string(&args[1])?;
    let values = args.get(2).copied().unwrap_or(Value::NIL);

    let result = DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;

        let mut stmt = conn.prepare(&sql).map_err(check_rusqlite)?;
        bind_values(&mut stmt, &values)?;

        let num_cols = stmt.column_count();
        let is_select = num_cols > 0;

        if is_select {
            let mut rows = stmt.query([]).map_err(check_rusqlite)?;
            let mut result_rows: Vec<Value> = Vec::new();
            while let Some(row) = rows.next().map_err(check_rusqlite)? {
                let mut row_vals: Vec<Value> = Vec::new();
                for col_idx in 0..num_cols {
                    let val: rusqlite::types::Value = row.get(col_idx).map_err(check_rusqlite)?;
                    row_vals.push(sqlite_value_to_elisp(rusqlite::types::ValueRef::from(&val)));
                }
                result_rows.push(Value::list(row_vals));
            }
            if result_rows.is_empty() {
                Ok(Value::NIL)
            } else {
                Ok(Value::list(result_rows))
            }
        } else {
            stmt.execute([]).map_err(check_rusqlite)?;
            let changes = conn.changes();
            Ok(Value::fixnum(changes as i64))
        }
    })?;

    Ok(result)
}

/// (sqlite-select DB QUERY &optional VALUES RETURN-TYPE) → results
pub(crate) fn builtin_sqlite_select(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args_range("sqlite-select", &args, 2, 4)?;
    let id = expect_db(&args[0])?;
    let sql = expect_strict_string(&args[1])?;
    let values = args.get(2).copied().unwrap_or(Value::NIL);
    let return_type = args.get(3).and_then(SqliteReturnType::from_value);
    let is_set = return_type == Some(SqliteReturnType::Set);
    let is_full = return_type == Some(SqliteReturnType::Full);

    if is_set {
        // Keep a live SQLite statement and step it incrementally, like GNU.
        let stmt = DB_HANDLES.with(|h| {
            let mut handles = h.borrow_mut();
            let conn = handles.get_mut(&id).ok_or_else(|| {
                signal(
                    LispCondition::SqliteError,
                    vec![Value::string("Database closed")],
                )
            })?;

            let sql_c =
                CString::new(sql.as_bytes()).map_err(|_| sqlite_err("embedded null byte"))?;
            let mut stmt = ptr::null_mut();
            let db = unsafe { conn.handle() };
            let ret = unsafe {
                ffi::sqlite3_prepare_v2(
                    db,
                    sql_c.as_ptr(),
                    sql.len() as i32,
                    &mut stmt,
                    ptr::null_mut(),
                )
            };
            if ret != ffi::SQLITE_OK {
                if !stmt.is_null() {
                    unsafe {
                        ffi::sqlite3_finalize(stmt);
                    }
                }
                let msg = unsafe { sqlite_errmsg_for_db(db) };
                return Err(sqlite_err(&msg));
            }
            if let Err(err) = unsafe { bind_raw_values(db, stmt, &values) } {
                unsafe {
                    ffi::sqlite3_finalize(stmt);
                }
                return Err(err);
            }
            Ok::<*mut ffi::sqlite3_stmt, Flow>(stmt)
        })?;

        let stmt_id = alloc_handle_id();
        RESULT_SETS.with(|h| {
            h.borrow_mut().insert(
                stmt_id,
                ResultSet {
                    db_id: id,
                    stmt,
                    eof: false,
                },
            )
        });
        return Ok(make_stmt_handle(stmt_id));
    }

    // Non-set mode: materialize and return immediately.
    let result = DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;

        let mut stmt = conn.prepare(&sql).map_err(check_rusqlite)?;
        bind_values(&mut stmt, &values)?;

        let num_cols = stmt.column_count();
        let mut column_names: Vec<Value> = Vec::new();
        if is_full {
            for i in 0..num_cols {
                column_names.push(Value::string(stmt.column_name(i).unwrap_or("?")));
            }
        }

        let mut rows = stmt.query([]).map_err(check_rusqlite)?;
        let mut result_rows: Vec<Value> = Vec::new();
        while let Some(row) = rows.next().map_err(check_rusqlite)? {
            let mut row_vals: Vec<Value> = Vec::new();
            for col_idx in 0..num_cols {
                let val: rusqlite::types::Value = row.get(col_idx).map_err(check_rusqlite)?;
                row_vals.push(sqlite_value_to_elisp(rusqlite::types::ValueRef::from(&val)));
            }
            result_rows.push(Value::list(row_vals));
        }

        if is_full {
            let mut full_result = vec![Value::list(column_names)];
            full_result.extend(result_rows);
            Ok(Value::list(full_result))
        } else if result_rows.is_empty() {
            Ok(Value::NIL)
        } else {
            Ok(Value::list(result_rows))
        }
    })?;

    Ok(result)
}

/// (sqlite-next SET) → row or nil
pub(crate) fn builtin_sqlite_next(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-next", &args, 1)?;
    let id = expect_stmt(&args[0])?;

    RESULT_SETS.with(|h| {
        let mut handles = h.borrow_mut();
        let rs = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Statement closed")],
            )
        })?;
        if rs.eof {
            return Ok(Value::NIL);
        }
        let ret = unsafe { ffi::sqlite3_step(rs.stmt) };
        if ret == ffi::SQLITE_ROW {
            Ok(unsafe { raw_row_to_value(rs.stmt) })
        } else if ret == ffi::SQLITE_DONE {
            rs.eof = true;
            Ok(Value::NIL)
        } else {
            let msg = DB_HANDLES.with(|dbs| {
                dbs.borrow()
                    .get(&rs.db_id)
                    .map(|conn| unsafe { sqlite_errmsg_for_db(conn.handle()) })
                    .unwrap_or_else(|| "sqlite error".to_string())
            });
            Err(sqlite_err(&msg))
        }
    })
}

/// (sqlite-more-p SET) → t or nil
pub(crate) fn builtin_sqlite_more_p(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-more-p", &args, 1)?;
    let id = expect_stmt(&args[0])?;
    let has_more = RESULT_SETS.with(|h| h.borrow().get(&id).is_some_and(|rs| !rs.eof));
    Ok(Value::bool_val(has_more))
}

/// (sqlite-columns SET) → list of column name strings
pub(crate) fn builtin_sqlite_columns(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-columns", &args, 1)?;
    let id = expect_stmt(&args[0])?;

    RESULT_SETS.with(|h| {
        let handles = h.borrow();
        let rs = handles.get(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Statement closed")],
            )
        })?;
        let count = unsafe { ffi::sqlite3_column_count(rs.stmt) };
        let mut columns = Vec::with_capacity(count as usize);
        for i in 0..count {
            let name = unsafe { ffi::sqlite3_column_name(rs.stmt, i) };
            let text = if name.is_null() {
                "?".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(name) }
                    .to_string_lossy()
                    .into_owned()
            };
            columns.push(Value::string(text));
        }
        Ok(Value::list(columns))
    })
}

/// (sqlite-finalize SET) → t
pub(crate) fn builtin_sqlite_finalize(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-finalize", &args, 1)?;
    let id = expect_stmt(&args[0])?;
    RESULT_SETS.with(|h| h.borrow_mut().remove(&id));
    Ok(Value::T)
}

/// (sqlite-execute-batch DB STATEMENTS) → t or nil
pub(crate) fn builtin_sqlite_execute_batch(
    _ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::builtins::expect_args("sqlite-execute-batch", &args, 2)?;
    let id = expect_db(&args[0])?;
    let statements = expect_strict_string(&args[1])?;

    DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;
        conn.execute_batch(&statements).map_err(check_rusqlite)?;
        Ok(Value::T)
    })
}

/// (sqlite-transaction DB) → t
pub(crate) fn builtin_sqlite_transaction(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-transaction", &args, 1)?;
    let id = expect_db(&args[0])?;
    DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;
        conn.execute_batch("begin").map_err(check_rusqlite)?;
        Ok(Value::T)
    })
}

/// (sqlite-commit DB) → nil
pub(crate) fn builtin_sqlite_commit(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-commit", &args, 1)?;
    let id = expect_db(&args[0])?;
    DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;
        conn.execute_batch("commit").map_err(check_rusqlite)?;
        Ok(Value::T)
    })
}

/// (sqlite-rollback DB) → nil
pub(crate) fn builtin_sqlite_rollback(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-rollback", &args, 1)?;
    let id = expect_db(&args[0])?;
    DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;
        conn.execute_batch("rollback").map_err(check_rusqlite)?;
        Ok(Value::T)
    })
}

/// (sqlite-pragma DB PRAGMA) → t
pub(crate) fn builtin_sqlite_pragma(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-pragma", &args, 2)?;
    let id = expect_db(&args[0])?;
    let pragma = expect_strict_string(&args[1])?;
    DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal(
                LispCondition::SqliteError,
                vec![Value::string("Database closed")],
            )
        })?;
        conn.execute_batch(&format!("PRAGMA {pragma}"))
            .map_err(check_rusqlite)?;
        Ok(Value::T)
    })
}

/// (sqlite-load-extension DB MODULE) → t
///
/// GNU semantics: load a SQLite extension, restricted to an allowlist.
pub(crate) fn builtin_sqlite_load_extension(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::builtins::expect_args("sqlite-load-extension", &args, 2)?;
    let id = expect_db(&args[0])?;
    let module = expect_strict_string(&args[1])?;

    // GNU's allowlist of allowed extension names.
    const ALLOWED_EXTENSIONS: &[&str] = &[
        "base64",
        "cksumvfs",
        "compress",
        "csv",
        "csvtable",
        "fts3",
        "icu",
        "pcre",
        "percentile",
        "regexp",
        "rot13",
        "rtree",
        "sha1",
        "uuid",
        "vec0",
        "vector0",
        "vfslog",
        "vss0",
        "zipfile",
    ];

    let file_name = std::path::Path::new(&module)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(module.as_str());
    let module_name = file_name
        .strip_prefix("libsqlite3_mod_")
        .unwrap_or(file_name);
    let allowed = ALLOWED_EXTENSIONS.iter().any(|allow| {
        let Some(suffix) = module_name.strip_prefix(allow) else {
            return false;
        };
        !suffix.is_empty()
            && (suffix == ".so" || suffix == ".dylib" || suffix.eq_ignore_ascii_case(".dll"))
    });
    if !allowed {
        return Err(signal(
            LispCondition::SqliteError,
            vec![Value::string("Module name not on allowlist")],
        ));
    }

    let expanded = super::fileio::builtin_expand_file_name(eval, vec![args[1], Value::NIL])?;
    let Some(expanded_ls) = eval.lisp_string(expanded) else {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), expanded],
        ));
    };
    // Issue #131: build the extension path CString from the real Emacs/OS bytes
    // (on Unix this is the file-name byte sequence), not the PUA-sentinel storage
    // string, so a non-UTF-8 path is preserved.
    let Ok(ext_fn) = CString::new(expanded_ls.as_bytes()) else {
        return Ok(Value::NIL);
    };

    let loaded = DB_HANDLES.with(|handles| -> Result<bool, Flow> {
        let handles = handles.borrow();
        let conn = handles
            .get(&id)
            .ok_or_else(|| sqlite_err("Database closed"))?;
        let sdb = unsafe { conn.handle() };
        let enable = unsafe {
            ffi::sqlite3_db_config(
                sdb,
                ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
                1,
                ptr::null_mut::<std::ffi::c_int>(),
            )
        };
        if enable != ffi::SQLITE_OK {
            return Ok(false);
        }
        let mut err_msg: *mut std::os::raw::c_char = ptr::null_mut();
        let result =
            unsafe { ffi::sqlite3_load_extension(sdb, ext_fn.as_ptr(), ptr::null(), &mut err_msg) };
        unsafe {
            ffi::sqlite3_db_config(
                sdb,
                ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
                0,
                ptr::null_mut::<std::ffi::c_int>(),
            );
        }
        if !err_msg.is_null() {
            unsafe { ffi::sqlite3_free(err_msg.cast()) };
        }
        Ok(result == ffi::SQLITE_OK)
    })?;

    Ok(Value::bool_val(loaded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_symbol_domains_match_gnu_symbols() {
        assert_eq!(
            SqliteReturnType::from_value(&Value::symbol("set")),
            Some(SqliteReturnType::Set)
        );
        assert_eq!(
            SqliteReturnType::from_value(&Value::symbol("full")),
            Some(SqliteReturnType::Full)
        );
        assert_eq!(SqliteReturnType::Set.symbol_name(), "set");
        assert_eq!(SqliteReturnType::Full.symbol_name(), "full");
        assert_eq!(SqliteReturnType::from_value(&Value::symbol("rows")), None);
        assert_eq!(SqliteReturnType::from_value(&Value::NIL), None);

        assert_eq!(
            SqliteBindSymbol::from_value(&Value::symbol("false")),
            Some(SqliteBindSymbol::False)
        );
        assert_eq!(SqliteBindSymbol::False.symbol_name(), "false");
        assert!(value_is_false_symbol(&Value::symbol("false")));
        assert!(!value_is_false_symbol(&Value::keyword(":false")));
        assert_eq!(SqliteBindSymbol::from_value(&Value::T), None);
    }
}
