use std::path::Path;

pub use neovm_compiler::CompileArtifact;
pub use neovm_compiler::diagnostic::{Diagnostic, render_diagnostics};
pub use neovm_compiler::interp::InterpResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Interpreter,
}

impl Engine {
    pub fn name(self) -> &'static str {
        match self {
            Self::Interpreter => "interp",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteArtifact {
    pub compile: CompileArtifact,
    pub result: InterpResult,
    pub engine: Engine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Executor {
    engine: Engine,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new(Engine::Interpreter)
    }
}

impl Executor {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub fn engine(&self) -> Engine {
        self.engine
    }

    pub fn execute_source(
        &self,
        name: impl Into<String>,
        text: impl Into<String>,
        args: &[i64],
    ) -> ExecuteArtifact {
        match self.engine {
            Engine::Interpreter => execute_with_interpreter(name, text, args),
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

fn execute_with_interpreter(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
) -> ExecuteArtifact {
    let compile = neovm_compiler::compile_source(name, text);
    let mut diagnostics = compile.diagnostics.clone();
    let mut value = None;

    if !diagnostics.iter().any(Diagnostic::is_error) {
        match &compile.regir {
            Some(regir) => {
                diagnostics.extend(neovm_compiler::verify::verify_regir_module(regir));
                if !diagnostics.iter().any(Diagnostic::is_error) {
                    let result = neovm_compiler::interp::execute_module_with_args(regir, args);
                    value = result.value;
                    diagnostics.extend(result.diagnostics);
                }
            }
            None => diagnostics.push(Diagnostic::error(
                "execution requires a successfully lowered Register IR module",
            )),
        }
    }

    ExecuteArtifact {
        compile,
        result: InterpResult { value, diagnostics },
        engine: Engine::Interpreter,
    }
}

#[cfg(test)]
mod tests {
    use super::{Engine, Executor, execute_source};

    #[test]
    fn executes_runtime_free_source_with_default_interpreter() {
        let artifact = execute_source(
            "arith.el",
            ";;; -*- lexical-binding: t; -*-\n(+ 10 (* 2 3))",
            &[],
        );

        assert_eq!(artifact.engine, Engine::Interpreter);
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(16));
    }

    #[test]
    fn executes_recursive_module_function() {
        let executor = Executor::new(Engine::Interpreter);
        let artifact = executor.execute_source(
            "fact.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))",
            &[5],
        );

        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(120));
    }

    #[test]
    fn reports_runtime_dependent_operation() {
        let artifact = execute_source(
            "unknown.el",
            ";;; -*- lexical-binding: t; -*-\n(foo 1)",
            &[],
        );

        assert!(artifact.result.value.is_none());
        assert!(artifact.result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("unsupported Register IR interpreter operation")
        }));
    }
}
