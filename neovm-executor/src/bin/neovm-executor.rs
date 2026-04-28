use std::process::ExitCode;

use neovm_executor::{Executor, render_diagnostics};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };

    match command.as_str() {
        "run" => run(args.collect()),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> ExitCode {
    if args.iter().any(|arg| arg.starts_with("--engine=")) {
        eprintln!("--engine was removed; neovm-executor uses the LispValue object interpreter");
        return ExitCode::from(2);
    }
    let Some(path) = args.first() else {
        usage();
        return ExitCode::from(2);
    };
    let values = match parse_i64_args(&args[1..]) {
        Ok(values) => values,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let executor = Executor::default();
    let artifact = match executor.execute_file(path, &values) {
        Ok(artifact) => artifact,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            return ExitCode::from(1);
        }
    };

    if !artifact.result.diagnostics.is_empty() {
        eprint!(
            "{}",
            render_diagnostics(
                artifact.compile.source.name.as_deref().unwrap_or(path),
                &artifact.compile.source.text,
                &artifact.result.diagnostics,
            )
        );
    }

    if artifact
        .result
        .diagnostics
        .iter()
        .any(neovm_executor::Diagnostic::is_error)
    {
        return ExitCode::from(1);
    }

    match artifact.result.value {
        Some(value) => {
            println!("{}", artifact.runtime.format_value(value));
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("execution produced no value");
            ExitCode::from(1)
        }
    }
}

fn parse_i64_args(args: &[String]) -> Result<Vec<i64>, String> {
    args.iter()
        .map(|arg| {
            arg.parse::<i64>()
                .map_err(|error| format!("invalid integer argument `{arg}`: {error}"))
        })
        .collect()
}

fn usage() {
    eprintln!("usage: neovm-executor run <file.el> [i64-arg ...]");
}
