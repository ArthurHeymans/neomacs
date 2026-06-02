use super::eval::Context;
use super::intern::resolve_sym;
use super::value::{Value, eq_value, list_to_vec};

fn eval(ctx: &mut Context, source: &str) -> Value {
    ctx.eval_str(source).expect("xwidget form should evaluate")
}

fn xwidget_context() -> Context {
    let mut ctx = Context::new();
    ctx.provide_value(Value::symbol("xwidget"), None)
        .expect("provide xwidget in minimal test runtime");
    ctx
}

#[test]
fn make_xwidget_builds_gnu_model_and_info_vector() {
    crate::test_utils::init_test_tracing();
    let mut ctx = xwidget_context();

    let xwidget = eval(&mut ctx, r#"(make-xwidget 'webkit "Title" 320 200)"#);
    assert!(xwidget.is_xwidget());
    assert!(eval(&mut ctx, "(xwidget-live-p (car xwidget-list))").is_t());

    let info = eval(&mut ctx, "(xwidget-info (car xwidget-list))");
    let slots = info
        .as_vector_data()
        .expect("xwidget-info vector")
        .as_slice();
    assert_eq!(slots.len(), 4);
    assert_eq!(slots[0], Value::symbol("webkit"));
    assert_eq!(slots[1].as_runtime_string_owned().as_deref(), Some("Title"));
    assert_eq!(slots[2], Value::fixnum(320));
    assert_eq!(slots[3], Value::fixnum(200));

    let listed = eval(&mut ctx, "(car (get-buffer-xwidgets (current-buffer)))");
    assert!(eq_value(&listed, &xwidget));
}

#[test]
fn xwidget_public_list_is_not_the_internal_owner_list() {
    crate::test_utils::init_test_tracing();
    let mut ctx = xwidget_context();

    let xwidget = eval(&mut ctx, r#"(make-xwidget 'webkit "Title" 10 20)"#);
    eval(&mut ctx, "(setq xwidget-list nil)");
    let listed = eval(&mut ctx, "(get-buffer-xwidgets (current-buffer))");
    let items = list_to_vec(&listed).expect("proper xwidget list");
    assert_eq!(items.len(), 1);
    assert!(eq_value(&items[0], &xwidget));
}

#[test]
fn xwidget_plist_query_flag_resize_and_kill_follow_gnu_slots() {
    crate::test_utils::init_test_tracing();
    let mut ctx = xwidget_context();

    eval(
        &mut ctx,
        r#"(setq xw-test (make-xwidget 'webkit "Title" 10 20))"#,
    );
    let result = eval(
        &mut ctx,
        r#"
(progn
  (set-xwidget-plist xw-test '(a 1 b 2))
  (set-xwidget-query-on-exit-flag xw-test nil)
  (xwidget-resize xw-test 30 40)
  (list (xwidget-plist xw-test)
        (xwidget-query-on-exit-flag xw-test)
        (xwidget-size-request xw-test)))
"#,
    );
    let items = list_to_vec(&result).expect("proper result list");
    assert_eq!(
        list_to_vec(&items[0]).expect("plist"),
        vec![
            Value::symbol("a"),
            Value::fixnum(1),
            Value::symbol("b"),
            Value::fixnum(2),
        ]
    );
    assert!(items[1].is_nil());
    assert_eq!(
        list_to_vec(&items[2]).expect("size"),
        vec![Value::fixnum(30), Value::fixnum(40)]
    );

    let killed = eval(
        &mut ctx,
        r#"
(progn
  (kill-xwidget xw-test)
  (list (xwidget-live-p xw-test)
        (xwidget-buffer xw-test)
        (get-buffer-xwidgets (current-buffer))))
"#,
    );
    assert_eq!(
        list_to_vec(&killed).expect("kill result"),
        vec![Value::NIL, Value::NIL, Value::NIL]
    );
}

#[test]
fn make_xwidget_accepts_only_gnu_webkit_type() {
    crate::test_utils::init_test_tracing();
    let mut ctx = Context::new();

    let err = ctx
        .eval_str(r#"(make-xwidget 'video "Title" 10 20)"#)
        .expect_err("GNU make-xwidget accepts only webkit");
    let super::error::EvalError::Signal { symbol, data, .. } = err else {
        panic!("make-xwidget should signal error");
    };
    assert_eq!(resolve_sym(symbol), "error");
    assert_eq!(data, vec![Value::string("Bad xwidget type")]);
}
