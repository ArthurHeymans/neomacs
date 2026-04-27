use std::process::ExitCode;

use neovm_executor::{Engine, Executor, render_diagnostics};

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
    let Some((engine, args)) = parse_engine(args) else {
        usage();
        return ExitCode::from(2);
    };
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

    let executor = Executor::new(engine);
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
            println!("{value}");
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("execution produced no value");
            ExitCode::from(1)
        }
    }
}

fn parse_engine(args: Vec<String>) -> Option<(Engine, Vec<String>)> {
    let mut engine = Engine::Interpreter;
    let mut rest = Vec::new();
    for arg in args {
        if let Some(value) = arg.strip_prefix("--engine=") {
            engine = parse_engine_value(value)?;
        } else {
            rest.push(arg);
        }
    }
    Some((engine, rest))
}

fn parse_engine_value(value: &str) -> Option<Engine> {
    match value {
        "interp" | "interpreter" => Some(Engine::Interpreter),
        _ => {
            eprintln!("unknown engine `{value}`; supported engines: interp");
            None
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
    eprintln!("usage: neovm-executor run [--engine=interp] <file.el> [i64-arg ...]");
}
