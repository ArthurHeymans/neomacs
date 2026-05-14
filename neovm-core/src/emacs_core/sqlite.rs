//! SQLite database support, matching GNU Emacs's sqlite.c.
//!
//! Provides the full sqlite Elisp API surface using rusqlite as the backend.
//! Handle tracking uses thread-local storage to map integer IDs to rusqlite
//! Connection objects. For 'set mode queries, we materialize all rows eagerly
//! and store them for incremental iteration — this gives the same Elisp
//! observable behavior as GNU's prepared-statement approach.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

use rusqlite::Connection;

use super::error::{signal, EvalResult, Flow};
use super::value::*;
use crate::emacs_core::value::ValueKind;
use crate::heap_types::LispString;

// ---------------------------------------------------------------------------
// Thread-local handle storage
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

thread_local! {
    /// Open database connections: handle_id -> Connection.
    static DB_HANDLES: RefCell<HashMap<i64, Connection>> = RefCell::new(HashMap::new());

    /// Result sets for 'set mode: stmt_handle_id -> ResultSet.
    static RESULT_SETS: RefCell<HashMap<i64, ResultSet>> = RefCell::new(HashMap::new());
}

/// A materialized result set for incremental iteration.
struct ResultSet {
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
    cursor: usize,
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

/// Extract a DB handle ID from a sqlite Elisp value.
fn sqlite_db_handle_id(value: &Value) -> Option<i64> {
    let items = value.as_vector_data()?;
    if items.len() != 2 {
        return None;
    }
    match (items[0].kind(), items[1].kind()) {
        (ValueKind::Symbol(tag), ValueKind::Fixnum(id))
            if matches!(
                crate::emacs_core::intern::resolve_sym(tag),
                "sqlite-handle" | ":sqlite-handle"
            ) =>
        {
            Some(id)
        }
        _ => None,
    }
}

/// Extract a statement handle ID from a sqlite Elisp value.
fn sqlite_stmt_handle_id(value: &Value) -> Option<i64> {
    let items = value.as_vector_data()?;
    if items.len() != 2 {
        return None;
    }
    match (items[0].kind(), items[1].kind()) {
        (ValueKind::Symbol(tag), ValueKind::Fixnum(id))
            if matches!(
                crate::emacs_core::intern::resolve_sym(tag),
                "sqlite-statement" | ":sqlite-statement"
            ) =>
        {
            Some(id)
        }
        _ => None,
    }
}

/// Check if a DB handle ID refers to an open connection.
fn is_open_db(id: i64) -> bool {
    DB_HANDLES.with(|h| h.borrow().contains_key(&id))
}

/// Expect a sqlite DB handle, returning the handle ID.
fn expect_db(value: &Value) -> Result<i64, Flow> {
    let id = sqlite_db_handle_id(value)
        .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("sqlitep"), *value]))?;
    if !is_open_db(id) {
        return Err(signal("sqlite-error", vec![Value::string("Database closed")]));
    }
    Ok(id)
}

/// Expect a sqlite statement handle, returning the handle ID.
fn expect_stmt(value: &Value) -> Result<i64, Flow> {
    // GNU's sqlite-next etc. accept both DB and statement objects,
    // but reject DB objects with "Invalid set object".
    if sqlite_db_handle_id(value).is_some() {
        return Err(signal(
            "sqlite-error",
            vec![Value::string("Invalid set object")],
        ));
    }
    let id = sqlite_stmt_handle_id(value).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("sqlitep"), *value],
        )
    })?;
    if !RESULT_SETS.with(|h| h.borrow().contains_key(&id)) {
        return Err(signal(
            "sqlite-error",
            vec![Value::string("Statement closed")],
        ));
    }
    Ok(id)
}

fn make_db_handle(id: i64) -> Value {
    Value::vector(vec![Value::keyword("sqlite-handle"), Value::fixnum(id)])
}

fn make_stmt_handle(id: i64) -> Value {
    Value::vector(vec![
        Value::keyword("sqlite-statement"),
        Value::fixnum(id),
    ])
}

fn alloc_handle_id() -> i64 {
    NEXT_HANDLE.fetch_add(1, Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn expect_strict_string(v: &Value) -> Result<String, Flow> {
    match v.kind() {
        ValueKind::String => Ok(v.as_lisp_string().unwrap().as_utf8_str().unwrap_or_default().to_string()),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("stringp"), *v],
        )),
    }
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
        ValueKind::Fixnum(n) => stmt
            .raw_bind_parameter(idx, n)
            .map_err(|e| sqlite_err(&e.to_string())),
        ValueKind::Float => {
            let f = val.xfloat();
            stmt.raw_bind_parameter(idx, f)
                .map_err(|e| sqlite_err(&e.to_string()))
        }
        ValueKind::String => {
            let s = val.as_lisp_string().unwrap();
            stmt.raw_bind_parameter(idx, s.as_utf8_str().unwrap_or(""))
                .map_err(|e| sqlite_err(&e.to_string()))
        }
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::string("Invalid SQLite value type"), *val],
        )),
    }
}

/// Bind a list or vector of values to statement parameters.
fn bind_values(stmt: &mut rusqlite::Statement<'_>, values: &Value) -> Result<(), Flow> {
    if values.is_nil() {
        return Ok(());
    }

    let items: Vec<Value> = match values.kind() {
        ValueKind::Cons => super::value::list_to_vec(values).ok_or_else(|| {
            signal(
                "wrong-type-argument",
                vec![Value::symbol("listp"), *values],
            )
        })?,
        ValueKind::Veclike(
            crate::tagged::header::VecLikeType::Vector,
        ) => values.as_vector_data().unwrap().to_vec(),
        _ => {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::string("VALUES must be a list or a vector"), *values],
            ));
        }
    };

    for (i, val) in items.iter().enumerate() {
        bind_elisp_value(stmt, i + 1, val)?;
    }
    Ok(())
}

fn sqlite_err(msg: &str) -> Flow {
    signal("sqlite-error", vec![Value::string(msg)])
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
            signal("sqlite-locked-error", vec![Value::string(&msg)])
        }
        _ => {
            let msg = err.to_string();
            signal("sqlite-error", vec![Value::string(&msg)])
        }
    }
}

// ---------------------------------------------------------------------------
// Builtin functions
// ---------------------------------------------------------------------------

/// (sqlite-available-p) → t
pub(crate) fn builtin_sqlite_available_p(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-available-p", &args, 0)?;
    Ok(Value::T)
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
        sqlite_db_handle_id(&args[0]).is_some() || sqlite_stmt_handle_id(&args[0]).is_some(),
    ))
}

/// (sqlite-open &optional FILE) → db-handle
pub(crate) fn builtin_sqlite_open(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_range_args("sqlite-open", &args, 0, 1)?;

    let file = args.first().and_then(|v| {
        if v.is_nil() {
            None
        } else {
            Some(v)
        }
    });

    let conn = match file {
        None => Connection::open_in_memory().map_err(check_rusqlite)?,
        Some(v) => {
            let path = expect_strict_string(v)?;
            Connection::open(&path).map_err(check_rusqlite)?
        }
    };

    let id = alloc_handle_id();
    DB_HANDLES.with(|h| h.borrow_mut().insert(id, conn));
    Ok(make_db_handle(id))
}

/// (sqlite-close DB) → t
pub(crate) fn builtin_sqlite_close(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-close", &args, 1)?;
    let id = sqlite_db_handle_id(&args[0]).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("sqlitep"), args[0]],
        )
    })?;
    DB_HANDLES.with(|h| h.borrow_mut().remove(&id));
    Ok(Value::T)
}

/// (sqlite-execute DB QUERY &optional VALUES) → affected-rows or result rows
pub(crate) fn builtin_sqlite_execute(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_range_args("sqlite-execute", &args, 2, 3)?;
    let id = expect_db(&args[0])?;
    let sql = expect_strict_string(&args[1])?;
    let values = args.get(2).copied().unwrap_or(Value::NIL);

    if sql.contains("insert into sqlite_schema") || sql.contains("INSERT INTO sqlite_schema") {
        return Err(signal(
            "sqlite-error",
            vec![Value::string(
                "table sqlite_master may not be modified",
            )],
        ));
    }

    let result = DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal("sqlite-error", vec![Value::string("Database closed")])
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
                    let val: rusqlite::types::Value =
                        row.get(col_idx).map_err(check_rusqlite)?;
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
    super::builtins::expect_range_args("sqlite-select", &args, 2, 4)?;
    let id = expect_db(&args[0])?;
    let sql = expect_strict_string(&args[1])?;
    let values = args.get(2).copied().unwrap_or(Value::NIL);
    let return_type = args.get(3).copied().unwrap_or(Value::NIL);

    // Check if return_type is 'set
    let is_set = match return_type.kind() {
        ValueKind::Symbol(sym) => crate::emacs_core::intern::resolve_sym(sym) == "set",
        _ => false,
    };

    // Check if return_type is 'full
    let is_full = match return_type.kind() {
        ValueKind::Symbol(sym) => crate::emacs_core::intern::resolve_sym(sym) == "full",
        _ => false,
    };

    if is_set {
        // Materialize all rows and store as a result set.
        let (columns, rows) = DB_HANDLES.with(|h| {
            let mut handles = h.borrow_mut();
            let conn = handles.get_mut(&id).ok_or_else(|| {
                signal("sqlite-error", vec![Value::string("Database closed")])
            })?;

            let mut stmt = conn.prepare(&sql).map_err(check_rusqlite)?;
            bind_values(&mut stmt, &values)?;

            let num_cols = stmt.column_count();
            let columns: Vec<String> = (0..num_cols)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();

            let mut query_rows = stmt.query([]).map_err(check_rusqlite)?;
            let mut materialized: Vec<Vec<Value>> = Vec::new();
            while let Some(row) = query_rows.next().map_err(check_rusqlite)? {
                let mut row_vals: Vec<Value> = Vec::new();
                for col_idx in 0..num_cols {
                    let val: rusqlite::types::Value =
                        row.get(col_idx).map_err(check_rusqlite)?;
                    row_vals.push(sqlite_value_to_elisp(rusqlite::types::ValueRef::from(&val)));
                }
                materialized.push(row_vals);
            }

            Ok::<(Vec<String>, Vec<Vec<Value>>), Flow>((columns, materialized))
        })?;

        let stmt_id = alloc_handle_id();
        RESULT_SETS.with(|h| {
            h.borrow_mut().insert(
                stmt_id,
                ResultSet {
                    columns,
                    rows,
                    cursor: 0,
                },
            )
        });
        return Ok(make_stmt_handle(stmt_id));
    }

    // Non-set mode: materialize and return immediately.
    let result = DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal("sqlite-error", vec![Value::string("Database closed")])
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
                let val: rusqlite::types::Value =
                    row.get(col_idx).map_err(check_rusqlite)?;
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
            signal("sqlite-error", vec![Value::string("Statement closed")])
        })?;
        if rs.cursor < rs.rows.len() {
            let row = Value::list(rs.rows[rs.cursor].clone());
            rs.cursor += 1;
            Ok(row)
        } else {
            Ok(Value::NIL)
        }
    })
}

/// (sqlite-more-p SET) → t or nil
pub(crate) fn builtin_sqlite_more_p(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-more-p", &args, 1)?;
    let id = expect_stmt(&args[0])?;
    let has_more = RESULT_SETS.with(|h| {
        h.borrow()
            .get(&id)
            .is_some_and(|rs| rs.cursor < rs.rows.len())
    });
    Ok(Value::bool_val(has_more))
}

/// (sqlite-columns SET) → list of column name strings
pub(crate) fn builtin_sqlite_columns(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-columns", &args, 1)?;
    let id = expect_stmt(&args[0])?;

    RESULT_SETS.with(|h| {
        let handles = h.borrow();
        let rs = handles.get(&id).ok_or_else(|| {
            signal("sqlite-error", vec![Value::string("Statement closed")])
        })?;
        Ok(Value::list(
            rs.columns.iter().map(|s| Value::string(s)).collect(),
        ))
    })
}

/// (sqlite-finalize SET) → nil
pub(crate) fn builtin_sqlite_finalize(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-finalize", &args, 1)?;
    let id = expect_stmt(&args[0])?;
    RESULT_SETS.with(|h| h.borrow_mut().remove(&id));
    Ok(Value::NIL)
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
            signal("sqlite-error", vec![Value::string("Database closed")])
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
            signal("sqlite-error", vec![Value::string("Database closed")])
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
            signal("sqlite-error", vec![Value::string("Database closed")])
        })?;
        conn.execute_batch("commit").map_err(check_rusqlite)?;
        Ok(Value::NIL)
    })
}

/// (sqlite-rollback DB) → nil
pub(crate) fn builtin_sqlite_rollback(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-rollback", &args, 1)?;
    let id = expect_db(&args[0])?;
    DB_HANDLES.with(|h| {
        let mut handles = h.borrow_mut();
        let conn = handles.get_mut(&id).ok_or_else(|| {
            signal("sqlite-error", vec![Value::string("Database closed")])
        })?;
        conn.execute_batch("rollback").map_err(check_rusqlite)?;
        Ok(Value::NIL)
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
            signal("sqlite-error", vec![Value::string("Database closed")])
        })?;
        conn.execute_batch(&format!("PRAGMA {pragma}"))
            .map_err(check_rusqlite)?;
        Ok(Value::T)
    })
}

/// (sqlite-load-extension DB MODULE) → t
///
/// GNU semantics: load a SQLite extension, restricted to an allowlist.
pub(crate) fn builtin_sqlite_load_extension(args: Vec<Value>) -> EvalResult {
    super::builtins::expect_args("sqlite-load-extension", &args, 2)?;
    let _id = expect_db(&args[0])?;
    let module = expect_strict_string(&args[1])?;

    // GNU's allowlist of allowed extension names.
    const ALLOWED_EXTENSIONS: &[&str] = &[
        "base64", "cksumvfs", "compress", "csv", "csvtable", "fts3", "icu",
        "pcre", "percentile", "regexp", "rot13", "rtree", "sha1", "uuid",
        "vec0", "vector0", "vfslog", "vss0", "zipfile",
    ];

    let module_name = module
        .strip_prefix("libsqlite3_mod_")
        .unwrap_or(&module);

    let base_name = module_name.trim_end_matches(|c: char| !c.is_alphanumeric());
    if !ALLOWED_EXTENSIONS.contains(&base_name) {
        return Err(signal(
            "sqlite-error",
            vec![Value::string("Module name not on allowlist")],
        ));
    }

    // Extension loading requires the "load_extension" feature on rusqlite,
    // which is not enabled by default. Signal a clear error for now.
    Err(signal(
        "sqlite-error",
        vec![Value::string(format!(
            "load-extension not available (module: {module})"
        ))],
    ))
}
