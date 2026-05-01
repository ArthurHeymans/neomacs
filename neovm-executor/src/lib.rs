use std::path::Path;

mod object_interp;
pub mod runtime;
pub mod value;
mod jit_rt;
mod jit_interp;

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
        Self { engine: Engine::default() }
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
    use super::{Executor, LispValue, execute_source};

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
        assert_eq!(artifact.runtime.format_value(val), "\"something went wrong\"");
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
        assert_eq!(artifact.runtime.format_value(val), "((1 2 3) \"hi world\" \"hello emacs\")");
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
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3628800)));
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
        assert_eq!(artifact.runtime.format_value(val), "((1 2 3) \"hi world\" \"hello emacs\")");
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
        assert_eq!(artifact.result.value, Some(LispValue::expect_fixnum(3628800)));
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
        assert_eq!(artifact.runtime.format_value(val), "\"something went wrong\"");
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
}
