use std::process::ExitCode;

use neovm_compiler::compile_source;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        usage();
        return ExitCode::from(2);
    };

    match command.as_str() {
        "scan" => scan(args.collect()),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn usage() {
    eprintln!("usage: neovm-compiler <command> [args...]");
    eprintln!("  scan <file.el> ...             compile and report macro evaluation gaps");
}

fn scan(args: Vec<String>) -> ExitCode {
    if args.is_empty() {
        usage();
        return ExitCode::from(2);
    }

    let mut total_files = 0usize;
    let mut ok_files = 0usize;
    let mut reader_errors = 0usize;
    let mut hir_errors = 0usize;
    let mut macro_gaps: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for path in &args {
        total_files += 1;
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}: read error: {e}", path);
                continue;
            }
        };

        // Phase 1: reader only
        let src = neovm_compiler::source::SourceFile::new(
            neovm_compiler::source::SourceId::new(0),
            Some(path.clone()),
            text.clone(),
        );
        let reader_output = neovm_compiler::reader::read_source(&src);
        if !reader_output.diagnostics.is_empty() {
            reader_errors += 1;
            eprintln!(
                "{}: {} reader diagnostics (forms: {})",
                path,
                reader_output.diagnostics.len(),
                reader_output.forms.len()
            );
            // Don't continue — try expansion too
        } else {
            eprintln!("{}: reader OK (forms: {})", path, reader_output.forms.len());
        }

        // Phase 2: expansion + lowering
        let artifact = compile_source(path, &text);
        let has_reader_errors = artifact
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unexpected") || d.message.contains("expected"));
        let has_hir_errors = artifact
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unsupported HIR") || d.message.contains("not supported"));

        if !artifact.has_errors() {
            ok_files += 1;
            continue;
        }

        if has_reader_errors {
            reader_errors += 1;
        }
        if has_hir_errors {
            hir_errors += 1;
        }

        for diag in &artifact.diagnostics {
            if let Some(op) = diag.message.strip_prefix("cannot evaluate '") {
                if let Some(end) = op.find("' at macro expansion time") {
                    let op_name = op[..end].to_string();
                    macro_gaps.entry(op_name).or_default().push(path.clone());
                }
            } else if !has_reader_errors && !has_hir_errors {
                // Track other errors for non-reader/non-HIR failures
                let key = diag
                    .message
                    .split_whitespace()
                    .take(8)
                    .collect::<Vec<_>>()
                    .join(" ");
                macro_gaps.entry(key).or_default().push(path.clone());
            }
        }
    }

    println!("=== Scan Results ===");
    println!(
        "Files: {total_files} total, {ok_files} clean, {reader_errors} reader errors, {hir_errors} HIR errors"
    );
    println!();

    if macro_gaps.is_empty() {
        println!("No macro evaluation gaps found.");
    } else {
        let mut sorted: Vec<_> = macro_gaps.iter().collect();
        sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        println!("Macro evaluation gaps (operation: count, files):");
        for (op, files) in &sorted {
            println!("  {:30} {:3}x  {}", op, files.len(), files.first().unwrap());
        }
    }

    ExitCode::SUCCESS
}
