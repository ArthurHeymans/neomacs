use std::path::Path;

pub mod object_interp;
pub mod runtime;
pub mod value;

pub use neovm_compiler::CompileArtifact;
pub use neovm_compiler::diagnostic::{Diagnostic, render_diagnostics};
pub use object_interp::ObjectInterpResult;
pub use runtime::{Runtime, RuntimeError};
pub use value::LispValue;

pub struct ExecuteArtifact {
    pub compile: CompileArtifact,
    pub result: ObjectInterpResult,
    pub runtime: Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Executor;

impl Default for Executor {
    fn default() -> Self {
        Self
    }
}

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_source(
        &self,
        name: impl Into<String>,
        text: impl Into<String>,
        args: &[i64],
    ) -> ExecuteArtifact {
        execute_with_object_interpreter(name, text, args)
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
        result: ObjectInterpResult { value, diagnostics },
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
}
