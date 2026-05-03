use std::path::Path;

pub mod jit_interp;
mod jit_rt;
mod object_interp;
pub mod runtime;
pub mod value;

pub use neovm_compiler::CompileArtifact;
pub use neovm_compiler::diagnostic::{Diagnostic, render_diagnostics};
pub use runtime::{Runtime, RuntimeError};
pub use value::LispValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecuteResult {
    pub value: Option<LispValue>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ExecuteArtifact {
    pub compile: CompileArtifact,
    pub result: ExecuteResult,
    pub runtime: Runtime,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Engine {
    #[default]
    Interpreter,
    Jit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Executor {
    engine: Engine,
}

impl Default for Executor {
    fn default() -> Self {
        Self {
            engine: Engine::default(),
        }
    }
}

impl Executor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_engine(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn execute_source(
        &self,
        name: impl Into<String>,
        text: impl Into<String>,
        args: &[i64],
    ) -> ExecuteArtifact {
        match self.engine {
            Engine::Interpreter => execute_with_object_interpreter(name, text, args),
            Engine::Jit => execute_with_jit_engine(name, text, args),
        }
    }

    pub fn execute_file(
        &self,
        path: impl AsRef<Path>,
        args: &[i64],
    ) -> std::io::Result<ExecuteArtifact> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)?;
        Ok(self.execute_source(path.display().to_string(), text, args))
    }
}

pub fn execute_source(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
) -> ExecuteArtifact {
    Executor::default().execute_source(name, text, args)
}

pub fn execute_file(path: impl AsRef<Path>, args: &[i64]) -> std::io::Result<ExecuteArtifact> {
    Executor::default().execute_file(path, args)
}

fn execute_with_jit_engine(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
) -> ExecuteArtifact {
    let artifact = jit_interp::execute_with_jit(name, text, args);
    ExecuteArtifact {
        compile: artifact.compile,
        result: artifact.result,
        runtime: artifact.runtime,
    }
}

fn execute_with_object_interpreter(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
) -> ExecuteArtifact {
    let compile = neovm_compiler::compile_source(name, text);
    let mut diagnostics = compile.diagnostics.clone();
    let mut value = None;
    let mut runtime = Runtime::new();

    if !diagnostics.iter().any(Diagnostic::is_error) {
        match &compile.regir {
            Some(regir) => {
                diagnostics.extend(neovm_compiler::verify::verify_regir_module(regir));
                if !diagnostics.iter().any(Diagnostic::is_error) {
                    let args = args
                        .iter()
                        .map(|value| LispValue::from_fixnum(*value))
                        .collect::<Option<Vec<_>>>();
                    match args {
                        Some(args) => {
                            let result =
                                object_interp::execute_module_with_args(regir, &args, &mut runtime);
                            value = result.value;
                            diagnostics.extend(result.diagnostics);
                        }
                        None => diagnostics.push(Diagnostic::error(
                            "object interpreter arguments must fit in LispValue fixnums",
                        )),
                    }
                }
            }
            None => diagnostics.push(Diagnostic::error(
                "execution requires a successfully lowered Register IR module",
            )),
        }
    }

    ExecuteArtifact {
        compile,
        result: ExecuteResult { value, diagnostics },
        runtime,
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, Executor, LispValue, execute_source};

    #[test]
    fn executes_runtime_free_source_with_default_object_interpreter() {
        let artifact = execute_source(
            "arith.el",
            ";;; -*- lexical-binding: t; -*-\n(+ 10 (* 2 3))",
            &[],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(16)));
    }

    #[test]
    fn executes_recursive_module_function() {
        let executor = Executor::default();
        let artifact = executor.execute_source(
            "fact.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))",
            &[5],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(120)));
    }

    #[test]
    fn executes_defsubst_as_module_function() {
        let executor = Executor::default();
        let artifact = executor.execute_source(
            "defsubst.el",
            ";;; -*- lexical-binding: t; -*-\n(defsubst add1 (x) (1+ x))",
            &[4],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn reports_unsupported_named_call() {
        let artifact = execute_source(
            "unknown.el",
            ";;; -*- lexical-binding: t; -*-\n(foo 1)",
            &[],
        );

        assert!(artifact.result.value.is_none());
        assert!(artifact.result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("named call `foo` requires runtime support")
        }));
    }

    #[test]
    fn object_interpreter_executes_pair_primitives() {
        let executor = Executor::new();
        let artifact = executor.execute_source(
            "pair.el",
            ";;; -*- lexical-binding: t; -*-\n(car (cons 1 2))",
            &[],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn object_interpreter_executes_list_primitives() {
        let executor = Executor::new();
        let artifact = executor.execute_source(
            "list.el",
            ";;; -*- lexical-binding: t; -*-\n(length (reverse (list 1 2 3)))",
            &[],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn returned_heap_values_remain_owned_by_artifact_runtime() {
        let artifact = execute_source(
            "list-result.el",
            ";;; -*- lexical-binding: t; -*-\n(list 1 2 3)",
            &[],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        let value = artifact.result.value.expect("list value");
        assert_eq!(artifact.runtime.format_value(value), "(1 2 3)");
    }

    #[test]
    fn closures_survive_list_primitives_with_large_fixnums() {
        let artifact = execute_source(
            "closure-list.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((f (lambda (x) (+ x 1)))
      (n (- 0 1152921504606840000)))
  (+ (funcall (car (list f)) 1)
     (funcall (nth 0 (list f)) 2)
     (funcall (cdr (cons 0 f)) 3)
     (nth 1 (list f n))))",
            &[],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(
            artifact.result.value,
            Some(LispValue::expect_fixnum(-1152921504606839991))
        );
    }

    #[test]
    fn reports_integer_constants_outside_lispvalue_fixnum_range() {
        let artifact = execute_source(
            "wide-int.el",
            ";;; -*- lexical-binding: t; -*-\n3819615433963601919",
            &[],
        );

        assert!(artifact.result.value.is_none());
        assert!(artifact.result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("integer constant 3819615433963601919 requires bignum support")
        }));
    }

    #[test]
    fn e2e_recursive_fibonacci() {
        let artifact = execute_source(
            "fib.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fib (n) (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))\n(fib 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(55)));
    }

    #[test]
    fn e2e_higher_order_composition() {
        let artifact = execute_source(
            "compose.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun compose (f g) (lambda (x) (funcall f (funcall g x))))
(funcall (compose (lambda (x) (* x 2)) (lambda (x) (+ x 1))) 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn e2e_mutable_closure_counter() {
        let artifact = execute_source(
            "counter.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun make-counter (start)
  (let ((count start))
    (lambda ()
      (setq count (+ count 1))
      count)))
(defvar c (make-counter 0))
(list (funcall c) (funcall c) (funcall c))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert!(artifact.result.value.is_some());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3)");
    }

    #[test]
    fn e2e_mutable_closure_across_multiple_closures() {
        let artifact = execute_source(
            "shared-counter.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun make-shared-counter (start)
  (let ((count start))
    (list (lambda () (setq count (+ count 1)) count)
          (lambda () count))))
(let ((pair (make-shared-counter 0)))
  (funcall (nth 0 pair))
  (funcall (nth 0 pair))
  (funcall (nth 1 pair)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn e2e_insertion_sort() {
        let artifact = execute_source(
            "isort.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun insert-sorted (x sorted)
  (cond ((null sorted) (list x))
        ((<= x (car sorted)) (cons x sorted))
        (t (cons (car sorted) (insert-sorted x (cdr sorted))))))
(defun insertion-sort (lst)
  (if (null lst) nil
    (insert-sorted (car lst) (insertion-sort (cdr lst)))))
(insertion-sort (list 5 3 1 4 2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4 5)");
    }

    #[test]
    fn e2e_filter_map_reduce() {
        let artifact = execute_source(
            "fmr.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-filter (pred lst)
  (if (null lst) nil
    (if (funcall pred (car lst))
        (cons (car lst) (my-filter pred (cdr lst)))
      (my-filter pred (cdr lst)))))
(defun my-reduce (fn init lst)
  (if (null lst) init
    (my-reduce fn (funcall fn init (car lst)) (cdr lst))))
(my-reduce '+ 0
  (mapcar (lambda (x) (* x x))
    (my-filter (lambda (x) (> x 2)) (list 1 2 3 4 5))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(50)));
    }

    #[test]
    fn e2e_dynamic_variables() {
        let artifact = execute_source(
            "dyn.el",
            "\
;;; -*- lexical-binding: t; -*-
(defvar total 0)
(defun add-total (n) (setq total (+ total n)))
(add-total 5)
(add-total 10)
total",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn e2e_condition_case_catches_arith_error() {
        let artifact = execute_source(
            "safe-div.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun safe-div (a b)
  (condition-case err
      (/ a b)
    (arith-error 0)))
(list (safe-div 10 3) (safe-div 10 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(3 0)");
    }

    #[test]
    fn e2e_condition_case_catches_error_signal() {
        let artifact = execute_source(
            "catch-error.el",
            "\
;;; -*- lexical-binding: t; -*-
(condition-case err
    (error \"something went wrong\")
  (error (cadr err)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "\"something went wrong\""
        );
    }

    #[test]
    fn e2e_unwind_protect_cleanup_runs_on_signal() {
        let artifact = execute_source(
            "unwind.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((x 0))
  (condition-case nil
      (unwind-protect
          (progn (setq x 1) (signal 'test-signal nil))
        (setq x 2))
    (test-signal (setq x (+ x 10))))
  x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn e2e_throw_catch() {
        let artifact = execute_source(
            "throw.el",
            ";;; -*- lexical-binding: t; -*-\n(catch 'done (throw 'done 42))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn e2e_string_operations() {
        let artifact = execute_source(
            "strings.el",
            "\
;;; -*- lexical-binding: t; -*-
(concat (substring \"hello world\" 0 5) \"!\")",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "\"hello!\"");
    }

    #[test]
    fn e2e_vector_binary_search() {
        let artifact = execute_source(
            "bsearch.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun binary-search (vec target lo hi)
  (if (> lo hi) -1
    (let ((mid (/ (+ lo hi) 2)))
      (cond ((= (aref vec mid) target) mid)
            ((< (aref vec mid) target) (binary-search vec target (+ mid 1) hi))
            (t (binary-search vec target lo (- mid 1)))))))
(let ((v (vector 1 3 5 7 9 11 13 15 17 19)))
  (list (binary-search v 7 0 9)
        (binary-search v 1 0 9)
        (binary-search v 19 0 9)
        (binary-search v 4 0 9)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(3 0 9 -1)");
    }

    #[test]
    fn e2e_mutual_recursion() {
        let artifact = execute_source(
            "mutual.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-even (n) (if (= n 0) t (my-odd (- n 1))))
(defun my-odd (n) (if (= n 0) nil (my-even (- n 1))))
(list (my-even 4) (my-odd 3))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t t)");
    }

    #[test]
    fn e2e_rest_and_optional_params() {
        let artifact = execute_source(
            "params.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-list (&rest args) args)
(defun greet (greeting &optional name)
  (concat greeting \" \" (if name name \"world\")))
(list (my-list 1 2 3) (greet \"hi\") (greet \"hello\" \"emacs\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "((1 2 3) \"hi world\" \"hello emacs\")"
        );
    }

    #[test]
    fn e2e_hash_table_operations() {
        let artifact = execute_source(
            "hash.el",
            "\
;;; -*- lexical-binding: t; -*-
(defvar h (make-hash-table :test 'equal))
(puthash \"a\" 1 h)
(puthash \"b\" 2 h)
(list (gethash \"a\" h) (gethash \"b\" h) (gethash \"c\" h 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 0)");
    }

    #[test]
    fn e2e_new_math_primitives() {
        let artifact = execute_source(
            "math.el",
            "\
;;; -*- lexical-binding: t; -*-
(list (abs -5) (mod 10 3) (mod -10 3) (max 1 3 2) (min 1 3 2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(5 1 2 3 1)");
    }

    #[test]
    fn e2e_type_predicates() {
        let artifact = execute_source(
            "types.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-fn (x) x)
(list (type-of 42) (type-of \"hello\") (type-of nil) (type-of (list 1 2))
      (functionp 'my-fn) (functionp 42) (booleanp t) (booleanp nil))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "(integer string symbol cons t nil t t)"
        );
    }

    #[test]
    fn e2e_cadr_and_friends() {
        let artifact = execute_source(
            "cadr.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((xs (list (list 1 2) (list 3 4) (list 5 6))))
  (list (caar xs) (cadr xs) (caddr xs) (cdar xs) (cddr xs)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "(1 (3 4) (5 6) (2) ((5 6)))"
        );
    }

    #[test]
    fn e2e_factorial_accumulator() {
        let artifact = execute_source(
            "fact.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun fact-acc (n acc)
  (if (= n 0) acc (fact-acc (- n 1) (* n acc))))
(fact-acc 10 1)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(
            artifact.result.value,
            Some(LispValue::expect_fixnum(3628800))
        );
    }

    #[test]
    fn e2e_while_loop() {
        let artifact = execute_source(
            "while.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun countdown (n)
  (let ((result nil))
    (while (> n 0)
      (setq result (cons n result))
      (setq n (- n 1)))
    result))
(countdown 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4 5)");
    }

    #[test]
    fn e2e_dolist_and_dotimes() {
        let artifact = execute_source(
            "loops.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun sum-list (xs)
  (let ((s 0))
    (dolist (x xs s)
      (setq s (+ s x)))))
(defun make-range (n)
  (let ((result nil))
    (dotimes (i n)
      (setq result (cons i result)))
    (reverse result)))
(list (sum-list (list 1 2 3 4 5)) (make-range 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(15 (0 1 2 3 4))");
    }

    #[test]
    fn e2e_cond_form() {
        let artifact = execute_source(
            "cond.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun classify (n)
  (cond ((< n 0) -1) ((= n 0) 0) (t 1)))
(list (classify (- 0 5)) (classify 0) (classify 42))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(-1 0 1)");
    }

    // --- JIT execution tests ---

    #[test]
    fn jit_simple_arithmetic() {
        let artifact = crate::jit_interp::execute_with_jit(
            "arith.el",
            ";;; -*- lexical-binding: t; -*-\n(+ 10 (* 2 3))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(16)));
    }

    #[test]
    fn jit_recursive_factorial() {
        let artifact = crate::jit_interp::execute_with_jit(
            "fact.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))\n(fact 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(120)));
    }

    #[test]
    fn jit_recursive_fibonacci() {
        let artifact = crate::jit_interp::execute_with_jit(
            "fib.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fib (n) (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))\n(fib 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(55)));
    }

    #[test]
    fn jit_float_constant() {
        let artifact = crate::jit_interp::execute_with_jit(
            "float-const.el",
            ";;; -*- lexical-binding: t; -*-\n3.14",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let value = artifact.result.value.unwrap();
        assert!(artifact.runtime.is_float(value));
        assert!((artifact.runtime.float_data(value).unwrap() - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn jit_float_arithmetic() {
        let artifact = crate::jit_interp::execute_with_jit(
            "float-arith.el",
            ";;; -*- lexical-binding: t; -*-\n(+ 1.5 2.5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let value = artifact.result.value.unwrap();
        assert!(artifact.runtime.is_float(value));
        assert_eq!(artifact.runtime.float_data(value).unwrap(), 4.0);
    }

    #[test]
    fn jit_float_fixnum_promotion() {
        let artifact = crate::jit_interp::execute_with_jit(
            "float-promote.el",
            ";;; -*- lexical-binding: t; -*-\n(+ 1 2.5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let value = artifact.result.value.unwrap();
        assert!(artifact.runtime.is_float(value));
        assert_eq!(artifact.runtime.float_data(value).unwrap(), 3.5);
    }

    #[test]
    fn jit_float_comparison() {
        let artifact = crate::jit_interp::execute_with_jit(
            "float-cmp.el",
            ";;; -*- lexical-binding: t; -*-\n(if (< 1.0 2.0) 42 99)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_let_bindings() {
        let artifact = crate::jit_interp::execute_with_jit(
            "let.el",
            ";;; -*- lexical-binding: t; -*-\n(let ((x 10) (y 20)) (+ x y))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn jit_cons_and_car() {
        let artifact = crate::jit_interp::execute_with_jit(
            "pair.el",
            ";;; -*- lexical-binding: t; -*-\n(car (cons 1 2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn jit_if_branching() {
        let artifact = crate::jit_interp::execute_with_jit(
            "if.el",
            ";;; -*- lexical-binding: t; -*-\n(if (> 5 3) 42 99)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_higher_order_composition() {
        let artifact = crate::jit_interp::execute_with_jit(
            "compose.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun compose (f g) (lambda (x) (funcall f (funcall g x))))
(funcall (compose (lambda (x) (* x 2)) (lambda (x) (+ x 1))) 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn jit_mutable_closure_counter() {
        let artifact = crate::jit_interp::execute_with_jit(
            "counter.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun make-counter (start)
  (let ((count start))
    (lambda ()
      (setq count (+ count 1))
      count)))
(defvar c (make-counter 0))
(list (funcall c) (funcall c) (funcall c))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3)");
    }

    #[test]
    fn jit_mutable_closure_shared() {
        let artifact = crate::jit_interp::execute_with_jit(
            "shared-counter.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun make-shared-counter (start)
  (let ((count start))
    (list (lambda () (setq count (+ count 1)) count)
          (lambda () count))))
(let ((pair (make-shared-counter 0)))
  (funcall (nth 0 pair))
  (funcall (nth 0 pair))
  (funcall (nth 1 pair)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn jit_insertion_sort() {
        let artifact = crate::jit_interp::execute_with_jit(
            "isort.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun insert-sorted (x sorted)
  (cond ((null sorted) (list x))
        ((<= x (car sorted)) (cons x sorted))
        (t (cons (car sorted) (insert-sorted x (cdr sorted))))))
(defun insertion-sort (lst)
  (if (null lst) nil
    (insert-sorted (car lst) (insertion-sort (cdr lst)))))
(insertion-sort (list 5 3 1 4 2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4 5)");
    }

    #[test]
    fn jit_filter_map_reduce() {
        let artifact = crate::jit_interp::execute_with_jit(
            "fmr.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-filter (pred lst)
  (if (null lst) nil
    (if (funcall pred (car lst))
        (cons (car lst) (my-filter pred (cdr lst)))
      (my-filter pred (cdr lst)))))
(defun my-reduce (fn init lst)
  (if (null lst) init
    (my-reduce fn (funcall fn init (car lst)) (cdr lst))))
(my-reduce '+ 0
  (mapcar (lambda (x) (* x x))
    (my-filter (lambda (x) (> x 2)) (list 1 2 3 4 5))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(50)));
    }

    #[test]
    fn jit_dynamic_variables() {
        let artifact = crate::jit_interp::execute_with_jit(
            "dyn.el",
            "\
;;; -*- lexical-binding: t; -*-
(defvar total 0)
(defun add-total (n) (setq total (+ total n)))
(add-total 5)
(add-total 10)
total",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_string_operations() {
        let artifact = crate::jit_interp::execute_with_jit(
            "strings.el",
            "\
;;; -*- lexical-binding: t; -*-
(concat (substring \"hello world\" 0 5) \"!\")",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "\"hello!\"");
    }

    #[test]
    fn jit_vector_binary_search() {
        let artifact = crate::jit_interp::execute_with_jit(
            "bsearch.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun binary-search (vec target lo hi)
  (if (> lo hi) -1
    (let ((mid (/ (+ lo hi) 2)))
      (cond ((= (aref vec mid) target) mid)
            ((< (aref vec mid) target) (binary-search vec target (+ mid 1) hi))
            (t (binary-search vec target lo (- mid 1)))))))
(let ((v (vector 1 3 5 7 9 11 13 15 17 19)))
  (list (binary-search v 7 0 9)
        (binary-search v 1 0 9)
        (binary-search v 19 0 9)
        (binary-search v 4 0 9)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(3 0 9 -1)");
    }

    #[test]
    fn jit_mutual_recursion() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mutual.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-even (n) (if (= n 0) t (my-odd (- n 1))))
(defun my-odd (n) (if (= n 0) nil (my-even (- n 1))))
(list (my-even 4) (my-odd 3))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t t)");
    }

    #[test]
    fn jit_rest_and_optional_params() {
        let artifact = crate::jit_interp::execute_with_jit(
            "params.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-list (&rest args) args)
(defun greet (greeting &optional name)
  (concat greeting \" \" (if name name \"world\")))
(list (my-list 1 2 3) (greet \"hi\") (greet \"hello\" \"emacs\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "((1 2 3) \"hi world\" \"hello emacs\")"
        );
    }

    #[test]
    fn jit_hash_table_operations() {
        let artifact = crate::jit_interp::execute_with_jit(
            "hash.el",
            "\
;;; -*- lexical-binding: t; -*-
(defvar h (make-hash-table :test 'equal))
(puthash \"a\" 1 h)
(puthash \"b\" 2 h)
(list (gethash \"a\" h) (gethash \"b\" h) (gethash \"c\" h 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 0)");
    }

    #[test]
    fn jit_type_predicates() {
        let artifact = crate::jit_interp::execute_with_jit(
            "types.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun my-fn (x) x)
(list (type-of 42) (type-of \"hello\") (type-of nil) (type-of (list 1 2))
      (functionp 'my-fn) (functionp 42) (booleanp t) (booleanp nil))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "(integer string symbol cons t nil t t)"
        );
    }

    #[test]
    fn jit_cadr_and_friends() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cadr.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((xs (list (list 1 2) (list 3 4) (list 5 6))))
  (list (caar xs) (cadr xs) (caddr xs) (cdar xs) (cddr xs)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "(1 (3 4) (5 6) (2) ((5 6)))"
        );
    }

    #[test]
    fn jit_factorial_accumulator() {
        let artifact = crate::jit_interp::execute_with_jit(
            "fact-acc.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun fact-acc (n acc)
  (if (= n 0) acc (fact-acc (- n 1) (* n acc))))
(fact-acc 10 1)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(
            artifact.result.value,
            Some(LispValue::expect_fixnum(3628800))
        );
    }

    #[test]
    fn jit_list_primitives() {
        let artifact = crate::jit_interp::execute_with_jit(
            "list.el",
            ";;; -*- lexical-binding: t; -*-\n(length (reverse (list 1 2 3)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_eq_eql_equal() {
        let artifact = crate::jit_interp::execute_with_jit(
            "eq.el",
            ";;; -*- lexical-binding: t; -*-\n(list (eq 42 42) (eql 42 42) (equal (list 1 2) (list 1 2)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t t t)");
    }

    #[test]
    fn jit_symbol_operations() {
        let artifact = crate::jit_interp::execute_with_jit(
            "sym.el",
            "\
;;; -*- lexical-binding: t; -*-
(defvar sym-x 42)
(list (boundp 'sym-x) (symbol-value 'sym-x) (set 'sym-x 99) sym-x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t 42 99 99)");
    }

    #[test]
    fn jit_string_comparison() {
        let artifact = crate::jit_interp::execute_with_jit(
            "strcmp.el",
            ";;; -*- lexical-binding: t; -*-\n(list (string= \"abc\" \"abc\") (string< \"abc\" \"abd\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t t)");
    }

    #[test]
    fn jit_math_primitives() {
        let artifact = crate::jit_interp::execute_with_jit(
            "math.el",
            "\
;;; -*- lexical-binding: t; -*-
(list (abs -5) (mod 10 3) (mod -10 3) (max 1 3 2) (min 1 3 2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(5 1 2 3 1)");
    }

    #[test]
    fn jit_memq_member_assq() {
        let artifact = crate::jit_interp::execute_with_jit(
            "search.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((xs (list 1 2 3 4)))
  (list (memq 3 xs) (member 3 xs) (assq 'b (list (cons 'a 1) (cons 'b 2)))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "((3 4) (3 4) (b . 2))");
    }

    // --- Tests for compiler-level constructs (while, cond, dolist, dotimes) ---

    #[test]
    fn jit_while_loop() {
        let artifact = crate::jit_interp::execute_with_jit(
            "while.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun countdown (n)
  (let ((result nil))
    (while (> n 0)
      (setq result (cons n result))
      (setq n (- n 1)))
    result))
(countdown 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4 5)");
    }

    #[test]
    fn jit_dolist_and_dotimes() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loops.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun sum-list (xs)
  (let ((s 0))
    (dolist (x xs s)
      (setq s (+ s x)))))
(defun make-range (n)
  (let ((result nil))
    (dotimes (i n)
      (setq result (cons i result)))
    (reverse result)))
(list (sum-list (list 1 2 3 4 5)) (make-range 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(15 (0 1 2 3 4))");
    }

    #[test]
    fn jit_cond_form() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun classify (n)
  (cond ((< n 0) -1) ((= n 0) 0) (t 1)))
(list (classify (- 0 5)) (classify 0) (classify 42))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(-1 0 1)");
    }

    #[test]
    fn jit_closures_with_large_fixnums() {
        let artifact = crate::jit_interp::execute_with_jit(
            "closure-list.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((f (lambda (x) (+ x 1)))
      (n (- 0 1152921504606840000)))
  (+ (funcall (car (list f)) 1)
     (funcall (nth 0 (list f)) 2)
     (funcall (cdr (cons 0 f)) 3)
     (nth 1 (list f n))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(
            artifact.result.value,
            Some(LispValue::expect_fixnum(-1152921504606839991))
        );
    }

    // --- Nonlocal control flow tests (require catch/throw/condition-case support) ---

    #[test]
    fn jit_throw_catch() {
        let artifact = crate::jit_interp::execute_with_jit(
            "throw.el",
            ";;; -*- lexical-binding: t; -*-\n(catch 'done (throw 'done 42))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_condition_case_catches_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "catch-error.el",
            "\
;;; -*- lexical-binding: t; -*-
(condition-case err
    (error \"something went wrong\")
  (error (cadr err)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "\"something went wrong\""
        );
    }

    #[test]
    fn jit_condition_case_catches_arith_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "safe-div.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun safe-div (a b)
  (condition-case err
      (/ a b)
    (arith-error 0)))
(list (safe-div 10 3) (safe-div 10 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(3 0)");
    }

    #[test]
    fn jit_unwind_protect_cleanup() {
        let artifact = crate::jit_interp::execute_with_jit(
            "unwind.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((x 0))
  (condition-case nil
      (unwind-protect
          (progn (setq x 1) (signal 'test-signal nil))
        (setq x 2))
    (test-signal (setq x (+ x 10))))
  x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn jit_inline_fixnum_add_overflow_fallback() {
        // Inline fixnum + should fallback to runtime for large results
        let artifact = crate::jit_interp::execute_with_jit(
            "overflow.el",
            ";;; -*- lexical-binding: t; -*-\n(+ 1 2)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_inline_fixnum_mul() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mul.el",
            ";;; -*- lexical-binding: t; -*-\n(* 6 7)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_inline_comparison_chain() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cmp.el",
            ";;; -*- lexical-binding: t; -*-\n(if (and (< 1 2) (> 3 2) (<= 5 5) (>= 4 3)) 1 0)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn jit_inline_eq_symbols() {
        let artifact = crate::jit_interp::execute_with_jit(
            "eq-sym.el",
            ";;; -*- lexical-binding: t; -*-\n(if (eq 'foo 'foo) 1 0)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn jit_inline_eq_fixnums() {
        let artifact = crate::jit_interp::execute_with_jit(
            "eq-int.el",
            ";;; -*- lexical-binding: t; -*-\n(if (eq 42 42) 1 0)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn jit_inline_integerp() {
        let artifact = crate::jit_interp::execute_with_jit(
            "intp.el",
            ";;; -*- lexical-binding: t; -*-\n(list (integerp 42) (integerp \"hello\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t nil)");
    }

    #[test]
    fn jit_inline_add1_sub1() {
        let artifact = crate::jit_interp::execute_with_jit(
            "add1-sub1.el",
            ";;; -*- lexical-binding: t; -*-\n(list (1+ 9) (1- 11))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(10 10)");
    }

    #[test]
    fn jit_inline_bitwise_ops() {
        let artifact = crate::jit_interp::execute_with_jit(
            "bitwise.el",
            ";;; -*- lexical-binding: t; -*-\n(list (logand 15 6) (logior 8 4) (logxor 12 10))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(6 12 6)");
    }

    #[test]
    fn jit_catch_throw_nested() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nested-catch.el",
            ";;; -*- lexical-binding: t; -*-\n(catch 'a (catch 'b (throw 'a 99)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn jit_catch_throw_no_match_returns_nil() {
        let artifact = crate::jit_interp::execute_with_jit(
            "no-match.el",
            ";;; -*- lexical-binding: t; -*-\n(catch 'x (+ 1 2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_condition_case_no_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "no-err.el",
            ";;; -*- lexical-binding: t; -*-\n(condition-case err (+ 1 2) (error 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_unwind_protect_normal_exit() {
        let artifact = crate::jit_interp::execute_with_jit(
            "unwind-normal.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((x 0))
  (unwind-protect
      (setq x 10)
    (setq x (+ x 1)))
  x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(11)));
    }

    // ── cl-loop end-to-end JIT tests ───────────────────────────────

    #[test]
    fn jit_cl_loop_for_from_collect() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-1.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 5 collect (* x x))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 4 9 16 25)");
    }

    #[test]
    fn jit_cl_loop_for_in_collect() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-2.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x in (list 10 20 30) collect (+ x 1))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(11 21 31)");
    }

    #[test]
    fn jit_cl_loop_sum() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-3.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 10 sum x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(55)));
    }

    #[test]
    fn jit_cl_loop_count() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-4.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 10 count (> x 7))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_cl_loop_do_body() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-5.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((acc 0))
  (cl-loop for x from 1 to 5 do (setq acc (+ acc x)))
  acc)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_cl_loop_with_binding() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-6.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop with base = 100 for x from 1 to 3 collect (+ x base))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(101 102 103)");
    }

    #[test]
    fn jit_cl_loop_while_termination() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-7.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 100 while (< x 5) collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4)");
    }

    #[test]
    fn jit_cl_loop_by_step() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-8.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 0 to 10 by 2 collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(0 2 4 6 8 10)");
    }

    #[test]
    fn jit_cl_loop_for_on() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-9.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x on (list 1 2 3) collect (car x))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3)");
    }

    #[test]
    fn jit_cl_loop_append() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-10.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x in (list (list 1 2) (list 3 4)) append x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4)");
    }

    #[test]
    fn jit_cl_loop_nconc() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-11.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x in (list (list 5 6) (list 7 8)) nconc x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(5 6 7 8)");
    }

    #[test]
    fn jit_cl_loop_empty() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-empty.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::NIL));
    }

    #[test]
    fn jit_cl_loop_repeat() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-repeat.el",
            "\
;;; -*- lexical-binding: t; -*-
(let ((acc 0))
  (cl-loop repeat 5 do (setq acc (+ acc 10)))
  acc)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(50)));
    }

    #[test]
    fn jit_cl_loop_always() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-always.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 5 always (> x 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // All x > 0, so always returns t
        assert_eq!(artifact.result.value, Some(LispValue::TRUE));
    }

    #[test]
    fn jit_cl_loop_always_fails() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-always-fail.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 5 always (< x 3))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // x=3 fails (< 3), so always returns nil
        assert_eq!(artifact.result.value, Some(LispValue::NIL));
    }

    #[test]
    fn jit_cl_loop_never() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-never.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 5 never (> x 10))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // No x > 10, so never returns t
        assert_eq!(artifact.result.value, Some(LispValue::TRUE));
    }

    #[test]
    fn jit_cl_loop_thereis() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-thereis.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 10 thereis (> x 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // First x > 5 is x=6, returns t (truthy)
        assert!(!artifact.result.value.unwrap().is_nil());
    }

    #[test]
    fn jit_cl_loop_minimize() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-min.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x in (list 5 3 8 1 9) minimize x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn jit_cl_loop_maximize() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-max.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x in (list 5 3 8 1 9) maximize x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(9)));
    }

    // --- JIT/interpreter boundary tests ---

    #[test]
    fn jit_calls_interpreter_function() {
        // Entry function is JIT-safe, callee has catch/throw (interpreter-only)
        let artifact = crate::jit_interp::execute_with_jit(
            "jit-interp-boundary.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun safe-caller (x)
  (+ x (catchy 10)))
(defun catchy (y)
  (catch 'tag (throw 'tag y)))
(safe-caller 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_entry_with_catch_falls_back_to_interpreter() {
        // Entry function itself has catch — entire module falls back
        let artifact = crate::jit_interp::execute_with_jit(
            "jit-catch-entry.el",
            "\
;;; -*- lexical-binding: t; -*-
(catch 'tag (throw 'tag 42))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_condition_case_boundary() {
        // JIT function calls a function that uses condition-case
        let artifact = crate::jit_interp::execute_with_jit(
            "jit-cond-boundary.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun safe-add (a b)
  (+ a (safe-div b)))
(defun safe-div (x)
  (condition-case err
    (/ x 1)
    (arith-error 0)))
(safe-add 10 3)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(13)));
    }

    #[test]
    fn jit_unwind_protect_boundary() {
        let artifact = crate::jit_interp::execute_with_jit(
            "jit-unwind-boundary.el",
            "\
;;; -*- lexical-binding: t; -*-
(defun with-cleanup (x)
  (unwind-protect
    (+ x 1)
    (message \"cleaned\")))
(with-cleanup 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(11)));
    }

    // --- Integer overflow / error tests ---

    #[test]
    fn division_by_zero_signals_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "div-zero.el",
            "(condition-case err (/ 10 0) (arith-error -1))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(-1)));
    }

    #[test]
    fn mod_by_zero_signals_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mod-zero.el",
            "(condition-case err (mod 10 0) (arith-error -1))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(-1)));
    }

    #[test]
    fn error_signal_is_catchable() {
        // (signal 'test-error '(42)) → err = (test-error 42), cadr = 42
        let artifact = crate::jit_interp::execute_with_jit(
            "error-catch.el",
            "(condition-case err (signal 'test-error '(42)) (test-error (cadr err)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    // --- JIT catch/throw with interpreter fallback bridge ---

    #[test]
    fn jit_catch_throw_from_interpreter_function() {
        // JIT catch + call to a function that throws (runs in interpreter via bridge)
        let artifact = crate::jit_interp::execute_with_jit(
            "catch-call.el",
            "(defun do-throw () (throw 'tag 77))
             (catch 'tag (do-throw))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(77)));
    }

    #[test]
    fn jit_catch_throw_with_computation() {
        // Catch that catches a throw from within a computation
        let artifact = crate::jit_interp::execute_with_jit(
            "catch-comp.el",
            "(+ 10 (catch 'tag (+ 1 (throw 'tag 20))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn jit_format_multiple_args() {
        // Tests the fixed format_string that advances arg index
        let artifact = crate::jit_interp::execute_with_jit(
            "format-args.el",
            "(format \"%s %d %s\" \"hello\" 42 \"world\")",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.string_contents(val).unwrap();
        assert_eq!(s, "hello 42 world");
    }

    // --- Complex end-to-end pipeline tests ---

    #[test]
    fn jit_closure_counter_with_funcall() {
        let artifact = crate::jit_interp::execute_with_jit(
            "closure-funcall.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((counter (let ((n 0))
               (lambda () (setq n (+ n 1)) n))))
               (funcall counter)
               (funcall counter)
               (funcall counter))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_mapcar_with_lambda() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mapcar-lambda.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((result (mapcar (lambda (x) (* x x)) '(1 2 3 4 5))))
               (car (cdr (cdr (cdr (cdr result))))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(25)));
    }

    #[test]
    fn jit_recursive_fibonacci_with_catch() {
        let artifact = crate::jit_interp::execute_with_jit(
            "fib-catch.el",
            ";;; -*- lexical-binding: t; -*-
             (defun fib (n)
               (if (< n 2) n
                 (+ (fib (- n 1)) (fib (- n 2)))))
             (+ 100 (catch 'done (throw 'done (fib 10))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(155)));
    }

    #[test]
    fn jit_nested_let_with_vector_mutation() {
        let artifact = crate::jit_interp::execute_with_jit(
            "vector-mut.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((v (vector 10 20 30 40 50)))
               (aset v 2 99)
               (+ (aref v 0) (aref v 1) (aref v 2) (aref v 3) (aref v 4)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(219)));
    }

    #[test]
    fn jit_multiple_closures_sharing_mutable_cell() {
        let artifact = crate::jit_interp::execute_with_jit(
            "shared-cell.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((cell (list 0)))
               (let ((inc (lambda () (setcar cell (+ 1 (car cell))))))
                 (funcall inc)
                 (funcall inc)
                 (funcall inc)
                 (car cell)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_cond_multi_clause() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond-multi.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((x 42))
               (cond ((< x 10) 1)
                     ((< x 20) 2)
                     ((< x 30) 3)
                     ((< x 50) 4)
                     (t 5)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn jit_condition_case_div_zero() {
        // Division by zero caught by condition-case with two handlers
        let artifact = crate::jit_interp::execute_with_jit(
            "arith-err-div.el",
            ";;; -*- lexical-binding: t; -*-
             (condition-case err
               (/ 10 0)
             (arith-error -1)
             (error -2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(-1)));
    }

    #[test]
    fn jit_nested_condition_case_inner_signals_to_outer() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nested-resignal.el",
            ";;; -*- lexical-binding: t; -*-
             (condition-case err
               (condition-case inner
                 (/ 1 0)
               (arith-error (signal 'error (list 99))))
             (error 42))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    // --- Pipeline deep audit: optimizer interaction tests ---

    #[test]
    fn jit_constant_folding_arithmetic_chain() {
        let artifact = crate::jit_interp::execute_with_jit(
            "const-fold.el",
            ";;; -*- lexical-binding: t; -*-
             (+ (+ 1 2) (+ 3 (+ 4 5)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_dead_code_after_return() {
        let artifact = crate::jit_interp::execute_with_jit(
            "dce.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((a (+ 1 2)))
               (let ((b (* a 3)))
                 (+ a b)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn jit_nested_if_else_chain() {
        let artifact = crate::jit_interp::execute_with_jit(
            "if-chain.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((x 3))
               (if (= x 1) 10
                 (if (= x 2) 20
                   (if (= x 3) 30 40))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn jit_multiple_catch_throw_nested() {
        let artifact = crate::jit_interp::execute_with_jit(
            "catch-nested.el",
            ";;; -*- lexical-binding: t; -*-
             (+ 100
                (catch 'outer
                  (catch 'inner
                    (throw 'outer 42))
                  999))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(142)));
    }

    #[test]
    fn jit_catch_return_from_inner_progn() {
        let artifact = crate::jit_interp::execute_with_jit(
            "catch-progn.el",
            ";;; -*- lexical-binding: t; -*-
             (catch 'done
               (progn
                 (+ 1 2)
                 (throw 'done 99)
                 (+ 3 4)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn jit_catch_throw_inside_cl_loop() {
        let artifact = crate::jit_interp::execute_with_jit(
            "catch-loop.el",
            ";;; -*- lexical-binding: t; -*-
             (catch 'done
               (cl-loop for i from 0 to 10
                        do (if (> i 5) (throw 'done (* i 10)))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(60)));
    }

    #[test]
    fn jit_conditional_let_binding() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond-let.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((x (if t 10 20)))
               (+ x 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_closure_over_loop_variable() {
        let artifact = crate::jit_interp::execute_with_jit(
            "closure-loop.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((fns nil))
               (let ((i 0))
                 (while (< i 3)
                   (push (lambda () i) fns)
                   (setq i (+ i 1))))
               (+ (funcall (car fns))
                  (funcall (cadr fns))
                  (funcall (caddr fns))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // All closures share the mutable cell for i, which ends at 3.
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn jit_recursive_with_base_case() {
        let artifact = crate::jit_interp::execute_with_jit(
            "recurse.el",
            ";;; -*- lexical-binding: t; -*-
             (defun fact (n)
               (if (<= n 1) 1
                 (* n (fact (- n 1)))))
             (fact 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(
            artifact.result.value,
            Some(LispValue::expect_fixnum(3628800))
        );
    }

    #[test]
    fn jit_tail_recursive_sum() {
        let artifact = crate::jit_interp::execute_with_jit(
            "tail-rec.el",
            ";;; -*- lexical-binding: t; -*-
             (defun sum-to (n acc)
               (if (= n 0) acc
                 (sum-to (- n 1) (+ acc n))))
             (sum-to 100 0)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(5050)));
    }

    #[test]
    fn jit_string_length_and_compare() {
        let artifact = crate::jit_interp::execute_with_jit(
            "strings.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((s \"hello\"))
               (+ (length s) (if (string= s \"hello\") 100 0)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(105)));
    }

    #[test]
    fn jit_vector_operations() {
        let artifact = crate::jit_interp::execute_with_jit(
            "vectors.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((v (vector 10 20 30)))
               (+ (aref v 0) (aref v 1) (aref v 2)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(60)));
    }

    #[test]
    fn jit_cl_loop_nested_collect() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-nested.el",
            ";;; -*- lexical-binding: t; -*-
             (length
               (cl-loop for i from 1 to 5
                        collect (* i i)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn jit_cl_loop_sum_and_count() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-sum-count.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((nums (list 1 2 3 4 5)))
               (+ (cl-loop for x in nums sum x)
                  (cl-loop for x in nums count (> x 3))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(17)));
    }

    #[test]
    fn jit_condition_case_with_binding() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond-bind.el",
            ";;; -*- lexical-binding: t; -*-
             (condition-case err
               (/ 1 0)
             (arith-error
              (car (cdr err))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // The error data for arith-error should be non-nil
        assert!(artifact.result.value.is_some());
    }

    #[test]
    fn jit_switch_like_cond() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond-switch.el",
            ";;; -*- lexical-binding: t; -*-
             (let ((x 2))
               (cond ((= x 1) 10)
                     ((= x 2) 20)
                     ((= x 3) 30)
                     (t 0)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(20)));
    }

    #[test]
    fn jit_and_or_short_circuit() {
        let artifact = crate::jit_interp::execute_with_jit(
            "and-or.el",
            ";;; -*- lexical-binding: t; -*-
             (+ (and 1 2 3)
                (or nil nil 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        // (and 1 2 3) = 3, (or nil nil 5) = 5, (+ 3 5) = 8
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(8)));
    }

    #[test]
    fn jit_let_star_sequential_binding() {
        let artifact = crate::jit_interp::execute_with_jit(
            "let-star.el",
            ";;; -*- lexical-binding: t; -*-
             (let* ((a 1)
                    (b (+ a 1))
                    (c (+ a b)))
               c)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn jit_string_length() {
        let artifact = crate::jit_interp::execute_with_jit(
            "strlen.el",
            ";;; -*- lexical-binding: t; -*-
             (length \"hello world\")",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(11)));
    }

    #[test]
    fn jit_multiple_return_paths_via_if() {
        let artifact = crate::jit_interp::execute_with_jit(
            "multi-return.el",
            ";;; -*- lexical-binding: t; -*-
             (defun classify (n)
               (cond ((< n 0) 'negative)
                     ((= n 0) 'zero)
                     (t 'positive)))
             (+ (if (eq (classify -5) 'negative) 100 0)
                (if (eq (classify 0) 'zero) 200 0)
                (if (eq (classify 7) 'positive) 300 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(600)));
    }

    // ── cl-loop `into` variable tests ─────────────────────────────

    #[test]
    fn jit_cl_loop_sum_into() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-sum-into.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for i from 1 to 5 sum i into total finally return total)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_cl_loop_count_into() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-count-into.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for i from 1 to 10
         count (= (mod i 2) 0) into evens
         finally return evens)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn jit_cl_loop_collect_into() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-collect-into.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x from 1 to 3
         collect (* x x) into squares
         finally return squares)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 4 9)");
    }

    #[test]
    fn jit_cl_loop_multiple_into() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-multi-into.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for i from 1 to 5
         sum i into total
         collect i into items
         finally return (list total items))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(15 (1 2 3 4 5))");
    }

    // ── JIT primitive parity tests ────────────────────────────────

    #[test]
    fn jit_bitwise_logand() {
        let artifact = crate::jit_interp::execute_with_jit(
            "bitwise.el",
            ";;; -*- lexical-binding: t; -*-\n(logand 15 6)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn jit_bitwise_logior_logxor_lognot() {
        let artifact = crate::jit_interp::execute_with_jit(
            "bitwise2.el",
            ";;; -*- lexical-binding: t; -*-\n(list (logior 4 2) (logxor 5 3) (lognot 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(6 6 -1)");
    }

    #[test]
    fn jit_bitwise_ash_lsh() {
        let artifact = crate::jit_interp::execute_with_jit(
            "shift.el",
            ";;; -*- lexical-binding: t; -*-\n(list (ash 1 4) (ash 16 -2))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(16 4)");
    }

    #[test]
    fn jit_string_number_conversion() {
        let artifact = crate::jit_interp::execute_with_jit(
            "strnum.el",
            ";;; -*- lexical-binding: t; -*-\n(list (number-to-string 42) (string-to-number \"99\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(\"42\" 99)");
    }

    #[test]
    fn jit_string_case_ops() {
        let artifact = crate::jit_interp::execute_with_jit(
            "case.el",
            ";;; -*- lexical-binding: t; -*-\n(list (downcase \"HELLO\") (upcase \"world\") (capitalize \"foo bar\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(
            artifact.runtime.format_value(val),
            "(\"hello\" \"WORLD\" \"Foo bar\")"
        );
    }

    #[test]
    fn jit_string_trim_split_join() {
        let artifact = crate::jit_interp::execute_with_jit(
            "trim.el",
            ";;; -*- lexical-binding: t; -*-\n(list (string-trim \"  hello  \") (string-join (list \"a\" \"b\" \"c\") \"-\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(\"hello\" \"a-b-c\")");
    }

    #[test]
    fn jit_delq_remove_elt() {
        let artifact = crate::jit_interp::execute_with_jit(
            "listops.el",
            ";;; -*- lexical-binding: t; -*-\n(list (delq 2 (list 1 2 3 2)) (elt (list 10 20 30) 1))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "((1 3) 20)");
    }

    #[test]
    fn jit_symbol_ops() {
        let artifact = crate::jit_interp::execute_with_jit(
            "symops.el",
            ";;; -*- lexical-binding: t; -*-\n(list (keywordp :foo) (keywordp 'bar))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t nil)");
    }

    #[test]
    fn jit_evenp_expt() {
        let artifact = crate::jit_interp::execute_with_jit(
            "math.el",
            ";;; -*- lexical-binding: t; -*-\n(list (evenp 4) (evenp 3) (expt 2 10))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t nil 1024)");
    }

    #[test]
    fn jit_cl_loop_for_equals_then() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-then.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for x = 0 then (+ x 2) repeat 5 collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(0 2 4 6 8)");
    }

    #[test]
    fn jit_cl_loop_for_equals_no_then() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cl-loop-eq.el",
            "\
;;; -*- lexical-binding: t; -*-
(cl-loop for i from 1 to 3
         for x = (* i i)
         collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 4 9)");
    }

    // ── setf expansion tests ───────────────────────────────────────

    #[test]
    fn jit_setf_symbol() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-sym.el",
            ";;; -*- lexical-binding: t; -*-\n(let ((x 1)) (setf x 42) x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_setf_car_cdr() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-car.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x (list 1 2 3)))
               (setf (car x) 10)
               (setf (cdr x) (list 20))
               x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(10 20)");
    }

    #[test]
    fn jit_setf_aref() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-aref.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v (vector 1 2 3)))
               (setf (aref v 1) 99)
               v)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "[1 99 3]");
    }

    #[test]
    fn jit_setf_nth() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-nth.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x (list 10 20 30)))
               (setf (nth 1 x) 99)
               x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(10 99 30)");
    }

    #[test]
    fn jit_setf_gethash() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-hash.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((h (make-hash-table)))
               (setf (gethash 'a h) 42)
               (gethash 'a h))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn jit_ash_shift_safety() {
        let artifact = crate::jit_interp::execute_with_jit(
            "ash.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (list (ash 1 100) (ash 1 -100) (ash -1 0))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        // Should not panic; results may wrap but must be valid fixnums
        let s = artifact.runtime.format_value(val);
        assert!(s.starts_with('('));
    }

    #[test]
    fn jit_logand_identity() {
        let artifact = crate::jit_interp::execute_with_jit(
            "logand.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (list (logand) (logand 15) (logand 15 6))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(-1 15 6)");
    }

    #[test]
    fn jit_expt_float_args() {
        // Integer expt returns fixnum
        let artifact = crate::jit_interp::execute_with_jit(
            "expt-int.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (expt 2 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(1024));

        // Float expt returns float
        let artifact = crate::jit_interp::execute_with_jit(
            "expt-float.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (expt 2.0 3)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert!(
            artifact.runtime.is_float(val),
            "expt 2.0 3 should return float"
        );
        assert_eq!(artifact.runtime.float_data(val).unwrap(), 8.0);
    }

    #[test]
    fn jit_string_to_number_float() {
        let artifact = crate::jit_interp::execute_with_jit(
            "s2n.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (list (string-to-number \"42\") (string-to-number \"3.14\"))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(42 3.14)");
    }

    #[test]
    fn jit_number_to_string_float() {
        let artifact = crate::jit_interp::execute_with_jit(
            "n2s.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (list (number-to-string 42) (number-to-string 3.14))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        // 3.14 becomes a float, number-to-string should produce "3.14"
        let s = artifact.runtime.format_value(val);
        assert!(s.contains("\"42\""));
        assert!(s.contains("3.14"));
    }

    #[test]
    fn jit_cl_loop_downto() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-downto.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i from 5 downto 1 collect i)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(5 4 3 2 1)");
    }

    #[test]
    fn jit_cl_loop_below() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-below.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i below 5 collect i)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(0 1 2 3 4)");
    }

    #[test]
    fn jit_cl_loop_above() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-above.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i from 10 above 7 collect i)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(10 9 8)");
    }

    #[test]
    fn jit_cl_loop_implicit_from() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-implicit.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i to 4 collect (* i i))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(0 1 4 9 16)");
    }

    #[test]
    fn jit_cl_symbol_macrolet() {
        let artifact = crate::jit_interp::execute_with_jit(
            "symacro.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-symbol-macrolet ((x (+ 1 2)))
               (* x 10))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(30));
    }

    #[test]
    fn jit_cl_symbol_macrolet_shadowing() {
        let artifact = crate::jit_interp::execute_with_jit(
            "symacro-shadow.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 5))
               (cl-symbol-macrolet ((x (+ 1 2)))
                 (+ x x)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        // x inside symbol-macrolet body expands to (+ 1 2) = 3
        // (+ 3 3) = 6
        assert_eq!(val, LispValue::expect_fixnum(6));
    }

    // --- Pipeline stress tests: surface real bugs ---

    #[test]
    fn stress_cl_loop_if_filter() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-if.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(1 2 3 4 5 6)
                      if (cl-oddp x)
                      collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 3 5)");
    }

    #[test]
    fn jit_cl_loop_when_filter() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-when.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(1 2 3 4 5)
                      when (> x 3)
                      collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(4 5)");
    }

    #[test]
    fn stress_cl_loop_repeat() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-repeat.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop repeat 5 collect 42)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(42 42 42 42 42)");
    }

    #[test]
    fn jit_cl_loop_sum_count() {
        // Test sum separately
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-sum.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(1 2 3 4 5) sum x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(15));

        // Test count separately
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-count.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(1 2 3 4 5)
                      count (cl-oddp x))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(3));
    }

    #[test]
    fn jit_cl_loop_maximize_minimize() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-maxmin.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(3 1 4 1 5 9 2 6)
                      maximize x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(9));
    }

    #[test]
    fn stress_cl_loop_append() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-append.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '((1 2) (3 4) (5 6))
                      append x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4 5 6)");
    }

    #[test]
    fn stress_cl_loop_for_on() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-for-on.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x on '(1 2 3)
                      collect x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "((1 2 3) (2 3) (3))");
    }

    #[test]
    fn jit_cl_loop_for_in_vector() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-vec.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x across [10 20 30]
                      sum x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(60));
    }

    #[test]
    fn jit_defun_optional_rest() {
        let artifact = crate::jit_interp::execute_with_jit(
            "defun-opt.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (defun my-add (&optional a b)
               (+ (or a 0) (or b 0)))
             (list (my-add) (my-add 5) (my-add 5 7))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(0 5 12)");
    }

    #[test]
    fn jit_defun_rest_args() {
        let artifact = crate::jit_interp::execute_with_jit(
            "defun-rest.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (defun my-sum (&rest args)
               (cl-loop for x in args sum x))
             (my-sum 1 2 3 4 5)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(15));
    }

    #[test]
    fn jit_closure_capture() {
        let artifact = crate::jit_interp::execute_with_jit(
            "closure.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 10))
               (funcall (lambda (y) (+ x y)) 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(15));
    }

    #[test]
    fn jit_nested_closures() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nested-closure.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (funcall
               (let ((x 3))
                 (lambda (y)
                   (funcall
                     (let ((z 7))
                       (lambda (a) (+ a x y z)))
                     100))) 200)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        // 100 + 3 + 200 + 7 = 310
        assert_eq!(val, LispValue::expect_fixnum(310));
    }

    #[test]
    fn jit_recursive_closure() {
        let artifact = crate::jit_interp::execute_with_jit(
            "recur-closure.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (letrec ((fact (lambda (n)
                              (if (<= n 1) 1 (* n (funcall fact (- n 1)))))))
               (funcall fact 10))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(3628800));
    }

    #[test]
    fn jit_condition_case_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond-case.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err
                 (/ 1 0)
               (arith-error (car err)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let sym = artifact.runtime.format_value(val);
        assert_eq!(sym, "arith-error");
    }

    #[test]
    fn stress_condition_case_no_error() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond-case-ok.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err
                 (+ 1 2)
               (error -1))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(3));
    }

    #[test]
    fn jit_unwind_protect() {
        let artifact = crate::jit_interp::execute_with_jit(
            "unwind.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 0))
               (unwind-protect
                    (setq x (+ x 1))
                 (setq x (+ x 10)))
               x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(11));
    }

    #[test]
    fn jit_throw_catch_value() {
        let artifact = crate::jit_interp::execute_with_jit(
            "throw-catch.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (catch 'my-tag
               (throw 'my-tag 42)
               99)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(42));
    }

    #[test]
    fn jit_setf_symbol_value() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-symval.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 10))
               (setf x 20)
               x)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(20));
    }

    #[test]
    fn jit_setf_push_pop() {
        let artifact = crate::jit_interp::execute_with_jit(
            "push-pop.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((lst nil))
               (push 1 lst)
               (push 2 lst)
               (push 3 lst)
               (let ((v (pop lst)))
                 (list v lst)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(3 (2 1))");
    }

    #[test]
    fn jit_mapcar_lambda() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mapcar.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (mapcar (lambda (x) (* x x)) '(1 2 3 4 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 4 9 16 25)");
    }

    #[test]
    fn jit_quasiquote() {
        let artifact = crate::jit_interp::execute_with_jit(
            "qq.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 42))
               `(a ,x b))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(a 42 b)");
    }

    #[test]
    fn jit_quasiquote_splice() {
        let artifact = crate::jit_interp::execute_with_jit(
            "qq-splice.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((xs '(2 3 4)))
               `(1 ,@xs 5))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2 3 4 5)");
    }

    #[test]
    fn jit_nthcdr_ops() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nthcdr.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((lst '(a b c d e)))
               (list (nth 0 lst) (nth 2 lst) (nthcdr 3 lst)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(a c (d e))");
    }

    #[test]
    fn jit_dotted_pair_basic() {
        let artifact = crate::jit_interp::execute_with_jit(
            "dotpair.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p '(a . 42)))
               (list (car p) (cdr p)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(a 42)");
    }

    #[test]
    fn jit_assq_simple() {
        let artifact = crate::jit_interp::execute_with_jit(
            "assq-simple.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((al (list (cons 'a 1) (cons 'b 2))))
               (assq 'b al))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(b . 2)");
    }

    #[test]
    fn jit_quoted_alist_assq() {
        let artifact = crate::jit_interp::execute_with_jit(
            "alist-assq.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((al '((a . 1) (b . 2))))
               (list (assq 'a al) (assq 'b al)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "((a . 1) (b . 2))");
    }

    #[test]
    fn jit_cl_incf_decf() {
        let artifact = crate::jit_interp::execute_with_jit(
            "incf-decf.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 10) (y 20))
               (cl-incf x)
               (cl-decf y 5)
               (cl-incf x 3)
               (list x y))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(14 15)");
    }

    // --- More pipeline stress tests: round 2 ---

    #[test]
    fn jit_defun_recursive() {
        let artifact = crate::jit_interp::execute_with_jit(
            "defun-recur.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (defun fib (n)
               (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
             (fib 10)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(55));
    }

    #[test]
    fn jit_defun_mutual_recursive() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mutual-recur.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (defun my-even (n) (if (= n 0) t (my-odd (- n 1))))
             (defun my-odd (n) (if (= n 0) nil (my-even (- n 1))))
             (list (my-even 4) (my-odd 5) (my-even 3))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(t t nil)");
    }

    #[test]
    fn jit_multiple_setf() {
        let artifact = crate::jit_interp::execute_with_jit(
            "multi-setf.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a 0) (b 0))
               (setf a 10 b 20)
               (list a b))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(10 20)");
    }

    #[test]
    fn jit_let_star() {
        let artifact = crate::jit_interp::execute_with_jit(
            "let-star.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let* ((x 1) (y (+ x 10)) (z (* y 2)))
               (list x y z))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 11 22)");
    }

    #[test]
    fn jit_when_unless() {
        let artifact = crate::jit_interp::execute_with_jit(
            "when-unless.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (list (when t 1 2 3)
                   (unless nil 4 5)
                   (when nil 99)
                   (unless t 99))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(3 5 nil nil)");
    }

    #[test]
    fn stress_cond_form() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond2.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 42))
               (cond ((= x 1) 'one)
                     ((= x 42) 'forty-two)
                     (t 'other)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "forty-two");
    }

    #[test]
    fn jit_nested_let() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nested-let.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 1))
               (let ((x 10) (y 20))
                 (+ x y)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(30));
    }

    #[test]
    fn jit_cl_loop_always_never() {
        // always: returns t if all satisfy, nil otherwise
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-always.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(2 4 6 8) always (cl-evenp x))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::TRUE);

        // never: returns t if none satisfy
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-never.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(2 4 6 8) never (cl-oddp x))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::TRUE);
    }

    #[test]
    fn stress_cl_loop_thereis() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-thereis2.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for x in '(1 2 3 4 5) thereis (and (> x 3) x))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(4));
    }

    #[test]
    fn jit_quoted_vector() {
        let artifact = crate::jit_interp::execute_with_jit(
            "quoted-vec.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v '[1 2 3]))
               (aref v 1))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(2));
    }

    #[test]
    fn jit_apply_primitive() {
        let artifact = crate::jit_interp::execute_with_jit(
            "apply.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (apply '+ '(1 2 3 4))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(10));
    }

    #[test]
    fn jit_funcall_with_lambda() {
        let artifact = crate::jit_interp::execute_with_jit(
            "funcall-lambda.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (funcall (function (lambda (x y) (+ x y))) 3 4)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(val, LispValue::expect_fixnum(7));
    }

    #[test]
    fn jit_hash_table_ops() {
        let artifact = crate::jit_interp::execute_with_jit(
            "hash.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((h (make-hash-table)))
               (puthash 'x 10 h)
               (puthash 'y 20 h)
               (list (gethash 'x h) (gethash 'y h) (gethash 'z h)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(10 20 nil)");
    }

    #[test]
    fn jit_progn_prog1() {
        let artifact = crate::jit_interp::execute_with_jit(
            "progn.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a 0))
               (list (progn (setq a 1) (setq a 2) a)
                     (prog1 (setq a 10) (setq a 20))))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(2 10)");
    }

    #[test]
    fn jit_setf_symbol_plist() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-plist.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((sym (intern \"test-sym\")))\n\
               (setf (symbol-plist sym) '(a 1 b 2))\n\
               (list (get sym 'a) (get sym 'b)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 2)");
    }

    #[test]
    fn jit_setf_plist_get() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-plist-get.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((plist '(a 1 b 2)))\n\
               (setf (plist-get plist 'b) 99)\n\
               (list (plist-get plist 'a) (plist-get plist 'b)))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "(1 99)");
    }

    // --- Real-world integration test ---

    #[test]
    fn jit_real_world_integration() {
        // Simulates a typical Elisp utility library with:
        // - defun with &optional/&rest
        // - recursive calls
        // - cl-loop with various clauses
        // - setf on multiple places
        // - closures and higher-order functions
        // - hash tables
        // - condition-case error handling
        // - pcase pattern matching
        let artifact = crate::jit_interp::execute_with_jit(
            "integration.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (defun my-filter (pred lst)\n\
               \"Filter elements of LST by PRED.\"\n\
               (cl-loop for x in lst\n\
                        if (funcall pred x) collect x))\n\
\n\
             (defun my-reduce (fn init lst)\n\
               \"Reduce LST using FN starting from INIT.\"\n\
               (if (null lst) init\n\
                 (my-reduce fn (funcall fn init (car lst)) (cdr lst))))\n\
\n\
             (defun my-partition (pred lst)\n\
               \"Partition LST into (matching . non-matching).\"\n\
               (let ((yes nil) (no nil))\n\
                 (dolist (x lst)\n\
                   (if (funcall pred x)\n\
                       (push x yes)\n\
                     (push x no)))\n\
                 (cons (nreverse yes) (nreverse no))))\n\
\n\
             (defun safe-div (a b)\n\
               \"Divide A by B, return nil on division by zero.\"\n\
               (condition-case nil\n\
                   (/ a b)\n\
                 (arith-error nil)))\n\
\n\
             (defun my-compose (f g)\n\
               \"Return a function that is F composed with G.\"\n\
               (lambda (x) (funcall f (funcall g x))))\n\
\n\
             (let* ((nums '(1 2 3 4 5 6 7 8 9 10))\n\
                    (evens (my-filter (lambda (x) (cl-evenp x)) nums))\n\
                    (odds (my-filter (lambda (x) (cl-oddp x)) nums))\n\
                    (sum (my-reduce (lambda (a b) (+ a b)) 0 nums))\n\
                    (part (my-partition (lambda (x) (> x 5)) nums))\n\
                    (double-then-inc (my-compose (lambda (x) (+ x 1))\n\
                                                 (lambda (x) (* x 2))))\n\
                    (div-result (list (safe-div 10 3) (safe-div 10 0))))\n\
               (list evens\n\
                     odds\n\
                     sum\n\
                     (car part)  ; matching (> 5)\n\
                     (cdr part)  ; non-matching (<= 5)\n\
                     (funcall double-then-inc 5)  ; 5*2+1=11\n\
                     div-result))",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        // Verify key results
        assert!(s.contains("(2 4 6 8 10)"), "evens: {s}");
        assert!(s.contains("(1 3 5 7 9)"), "odds: {s}");
        assert!(s.contains("55"), "sum: {s}"); // 1+2+...+10 = 55
        assert!(s.contains("11"), "compose: {s}"); // 5*2+1 = 11
    }

    // ── cl-loop `into` variable tests ────────────────────────────────────

    #[test]
    fn jit_cl_loop_into_sum_finally() {
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for i from 1 to 5 sum i into total
           finally return total))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn jit_cl_loop_into_all_no_finally() {
        // When all accumulations use `into`, the loop result should be nil
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for i from 1 to 5 sum i into total))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert!(
            artifact.result.value.unwrap().is_nil(),
            "expected nil when all accums use into"
        );
    }

    #[test]
    fn jit_cl_loop_into_collect_finally() {
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for i from 1 to 4 collect (* i i) into squares
           finally return squares))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        assert_eq!(s, "(1 4 9 16)");
    }

    #[test]
    fn jit_cl_loop_into_count_finally() {
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for i from 1 to 10
           count (cl-oddp i) into odd-count
           finally return odd-count))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn jit_cl_loop_into_mixed_default() {
        // sum with into + count without into → count is the loop result
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for i from 1 to 10
           sum i into total
           count (cl-oddp i)))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn jit_cl_loop_into_maximize_finally() {
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for x in (list 3 7 2 9 5)
           maximize x into biggest
           finally return biggest))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn jit_cl_loop_into_multiple_finally() {
        // Multiple into variables used in finally
        let artifact = crate::jit_interp::execute_with_jit(
            "test",
            ";;; -*- lexical-binding: t; -*-
(defun f ()
  (cl-loop for i from 1 to 5
           sum i into total
           collect (* i i) into squares
           finally return (list total squares)))
(f)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        assert!(s.contains("15"), "total should be 15: {s}");
        assert!(
            s.contains("1") && s.contains("25"),
            "squares should have 1..25: {s}"
        );
    }

    #[test]
    fn jit_comprehensive_integration() {
        let artifact = crate::jit_interp::execute_with_jit(
            "integration.el",
            r#"
;;; -*- lexical-binding: t; -*-

;; Recursive Fibonacci with memoization using hash table
(defun make-memoized-fib ()
  (let ((cache (make-hash-table :test 'equal)))
    (lambda (n)
      (condition-case err
          (if (<= n 1)
              n
            (let ((cached (gethash n cache)))
              (if cached
                  cached
                (let ((result (+ (funcall (make-memoized-fib) (- n 1))
                                 (funcall (make-memoized-fib) (- n 2)))))
                  (puthash n result cache)
                  result))))
        (error 0)))))

;; Higher-order: map with closure
(defun make-adder (x)
  (lambda (y) (+ x y)))

(defun map-adder (lst x)
  (let ((adder (make-adder x)))
    (cl-loop for item in lst
             collect (funcall adder item))))

;; String processing
(defun join-with-commas (lst)
  (if (null lst)
      ""
    (let ((result (car lst)))
      (dolist (item (cdr lst))
        (setq result (concat result ", " item)))
      result)))

;; Main test
(let* ((fib10 0)
       (adder-result (map-adder '(1 2 3 4 5) 10))
       (str-result (join-with-commas '("hello" "world" "test"))))
  ;; Compute fib safely
  (condition-case err
      (setq fib10 (funcall (make-memoized-fib) 10))
    (error (setq fib10 -1)))
  (list fib10 adder-result str-result))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        // fib(10) = 55, adder-result = (11 12 13 14 15), str-result = "hello, world, test"
        assert!(s.contains("55"), "fib(10) should be 55, got: {s}");
        assert!(s.contains("11"), "adder-result should contain 11, got: {s}");
        assert!(
            s.contains("hello"),
            "str-result should contain hello, got: {s}"
        );
    }

    #[test]
    fn jit_nested_condition_case_propagation() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nested-cc.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defun safe-compute (x)
  (condition-case outer-err
      (condition-case inner-err
          (if (< x 0)
              (error "negative: %s" x)
            (/ 100 x))
        (arith-error 'div-error))
    (error (cons 'caught outer-err))))
(list (safe-compute 10) (safe-compute 0) (safe-compute -5))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        // (safe-compute 10) = 10, (safe-compute 0) = div-error (arith caught by inner),
        // (safe-compute -5) = (caught . (error "negative: -5"))
        assert!(s.contains("10"), "should contain 10, got: {s}");
    }

    #[test]
    fn jit_closure_over_let_loop() {
        let artifact = crate::jit_interp::execute_with_jit(
            "closure-loop.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defun make-counter ()
  (let ((count 0))
    (lambda ()
      (setq count (+ count 1))
      count)))
(let ((c (make-counter)))
  (list (funcall c) (funcall c) (funcall c)))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        assert!(
            s.contains("1") && s.contains("2") && s.contains("3"),
            "counter should be (1 2 3), got: {s}"
        );
    }

    #[test]
    fn jit_cl_loop_with_hash_table() {
        let artifact = crate::jit_interp::execute_with_jit(
            "ht-loop.el",
            r#"
;;; -*- lexical-binding: t; -*-
(let ((ht (make-hash-table :test 'equal)))
  (puthash "a" 1 ht)
  (puthash "b" 2 ht)
  (puthash "c" 3 ht)
  (let ((sum 0))
    (maphash (lambda (k v) (setq sum (+ sum v))) ht)
    sum))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "6");
    }

    #[test]
    fn jit_cond_multi_branch() {
        let artifact = crate::jit_interp::execute_with_jit(
            "cond.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defun classify (n)
  (cond
   ((< n 0) 'negative)
   ((= n 0) 'zero)
   ((< n 10) 'small)
   (t 'large)))
(list (classify -5) (classify 0) (classify 7) (classify 100))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        assert!(s.contains("negative"), "should contain negative, got: {s}");
        assert!(s.contains("zero"), "should contain zero, got: {s}");
        assert!(s.contains("small"), "should contain small, got: {s}");
        assert!(s.contains("large"), "should contain large, got: {s}");
    }

    #[test]
    fn jit_recursive_accumulator() {
        let artifact = crate::jit_interp::execute_with_jit(
            "recurse-accum.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defun sum-to (n acc)
  (if (= n 0)
      acc
    (sum-to (- n 1) (+ acc n))))
(sum-to 100 0)
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "5050");
    }

    #[test]
    fn jit_cl_labels_mutual_recursion() {
        let artifact = crate::jit_interp::execute_with_jit(
            "mutual-recurse.el",
            r#"
;;; -*- lexical-binding: t; -*-
(cl-labels ((even-p (n) (if (= n 0) t (odd-p (- n 1))))
            (odd-p (n) (if (= n 0) nil (even-p (- n 1)))))
  (list (even-p 4) (odd-p 5) (even-p 3) (odd-p 2)))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        assert!(s.contains("t"), "even-p(4) should be t, got: {s}");
    }

    #[test]
    fn jit_defvar_dynamic_scope() {
        let artifact = crate::jit_interp::execute_with_jit(
            "defvar.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defvar my-counter 0)
(defun inc-counter ()
  (setq my-counter (+ my-counter 1)))
(inc-counter)
(inc-counter)
(inc-counter)
my-counter
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "3");
    }

    #[test]
    fn jit_setf_on_aref() {
        let artifact = crate::jit_interp::execute_with_jit(
            "setf-aref.el",
            r#"
;;; -*- lexical-binding: t; -*-
(let ((v (vector 1 2 3 4 5)))
  (setf (aref v 2) 99)
  (aref v 2))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "99");
    }

    #[test]
    fn jit_cl_loop_for_in_do() {
        let artifact = crate::jit_interp::execute_with_jit(
            "loop-do.el",
            r#"
;;; -*- lexical-binding: t; -*-
(let ((sum 0))
  (cl-loop for x in '(1 2 3 4 5) do (setq sum (+ sum x)))
  sum)
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "15");
    }

    #[test]
    fn jit_nested_let_closures() {
        let artifact = crate::jit_interp::execute_with_jit(
            "nested-closures.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defun make-pair-adder (a b)
  (let ((sum (+ a b)))
    (lambda (x) (+ x sum))))
(let ((f (make-pair-adder 10 20)))
  (funcall f 5))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        assert_eq!(artifact.runtime.format_value(val), "35");
    }

    #[test]
    fn jit_real_world_functional_lib() {
        let artifact = crate::jit_interp::execute_with_jit(
            "dash-like.el",
            r#"
;;; -*- lexical-binding: t; -*-
(defun my-map (fn list)
  (let ((result nil))
    (dolist (item list)
      (setq result (cons (funcall fn item) result)))
    (nreverse result)))

(defun my-filter (pred list)
  (let ((result nil))
    (dolist (item list)
      (when (funcall pred item)
        (setq result (cons item result))))
    (nreverse result)))

(defun my-reduce (fn init list)
  (let ((acc init))
    (dolist (item list)
      (setq acc (funcall fn acc item)))
    acc))

(let* ((nums '(1 2 3 4 5 6 7 8 9 10))
       (doubled (my-map (lambda (x) (* x 2)) nums))
       (evens (my-filter (lambda (x) (= (mod x 2) 0)) nums))
       (sum (my-reduce (lambda (acc x) (+ acc x)) 0 nums)))
  (list (nth 0 doubled) (nth 4 doubled)
        (length evens)
        sum))
"#,
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        let val = artifact.result.value.unwrap();
        let s = artifact.runtime.format_value(val);
        assert!(s.contains("2"), "first doubled should be 2, got: {s}");
        assert!(s.contains("10"), "fifth doubled should be 10, got: {s}");
        assert!(s.contains("5"), "should have 5 evens, got: {s}");
        assert!(s.contains("55"), "sum should be 55, got: {s}");
    }
}
