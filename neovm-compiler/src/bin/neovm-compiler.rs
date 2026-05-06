use std::process::ExitCode;

use neovm_compiler::ExpandMode;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        return ExitCode::from(2);
    }

    let command = args[0].clone();
    let rest = args.split_off(1);

    match command.as_str() {
        "scan" => scan(rest),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn usage() {
    eprintln!("usage: neovm-compiler <command> [args...]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  scan [--expand=minieval|emacs] [--emacs=PATH] <file.el> ...");
    eprintln!("        compile and report macro evaluation gaps");
}

fn parse_expand_args(args: Vec<String>) -> (ExpandMode, Vec<String>, Vec<String>) {
    let mut expand_mode = ExpandMode::MiniEval;
    let mut load_paths = vec!["lisp".to_string(), "lisp/emacs-lisp".to_string()];
    let mut positional = Vec::new();

    let mut i = 0;
    while i < args.len() {
        if let Some(value) = args[i].strip_prefix("--expand=") {
            expand_mode = match value {
                "minieval" => ExpandMode::MiniEval,
                "emacs" => ExpandMode::Emacs {
                    emacs_path: "emacs".to_string(),
                },
                _ => {
                    eprintln!("unknown expand mode '{value}' (options: minieval, emacs)");
                    std::process::exit(2);
                }
            };
        } else if let Some(value) = args[i].strip_prefix("--emacs=") {
            expand_mode = ExpandMode::Emacs {
                emacs_path: value.to_string(),
            };
        } else if args[i] == "--load-path" && i + 1 < args.len() {
            load_paths.push(args[i + 1].clone());
            i += 1;
        } else if let Some(value) = args[i].strip_prefix("--load-path=") {
            load_paths.push(value.to_string());
        } else {
            positional.push(args[i].clone());
        }
        i += 1;
    }

    (expand_mode, positional, load_paths)
}

fn scan(args: Vec<String>) -> ExitCode {
    let (expand_mode, files, load_paths) = parse_expand_args(args);
    if files.is_empty() {
        usage();
        return ExitCode::from(2);
    }

    // Process large batches via separate processes to avoid
    // cumulative memory pressure from jemalloc retaining freed
    // allocations across many compilations.
    const MAX_PER_PROCESS: usize = 5;
    if files.len() > MAX_PER_PROCESS {
        let expand_str = match &expand_mode {
            ExpandMode::MiniEval => "minieval".to_string(),
            ExpandMode::Emacs { emacs_path } => format!("emacs:{emacs_path}"),
        };
        return scan_in_subprocesses(&expand_str, files, load_paths);
    }

    scan_single_process(expand_mode, files, load_paths)
}

fn scan_in_subprocesses(expand_str: &str, files: Vec<String>, load_paths: Vec<String>) -> ExitCode {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cannot find compiler binary: {e}");
            return ExitCode::from(1);
        }
    };
    for chunk in files.chunks(30) {
        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("scan").arg(format!("--expand={expand_str}"));
        for lp in &load_paths {
            cmd.arg("--load-path").arg(lp);
        }
        cmd.args(chunk);
        match cmd.status() {
            Ok(status) => {
                if !status.success() {
                    return ExitCode::from(status.code().unwrap_or(1) as u8);
                }
            }
            Err(e) => {
                eprintln!("failed to spawn subprocess: {e}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

fn scan_single_process(
    expand_mode: ExpandMode,
    files: Vec<String>,
    load_paths: Vec<String>,
) -> ExitCode {
    let mut total_files = 0usize;
    let mut ok_files = 0usize;
    let mut reader_errors = 0usize;
    let mut hir_errors = 0usize;
    let mut macro_gaps: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for path in &files {
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
        } else {
            eprintln!("{}: reader OK (forms: {})", path, reader_output.forms.len());
        }

        // Phase 2: expansion + lowering (per-file fresh session to prevent
        // cumulative state exhaustion from large transitive require chains)
        let mut file_session = neovm_compiler::expand::CompilerSession::new();
        for lp in &load_paths {
            file_session.add_load_path((*lp).clone());
        }
        let artifact = neovm_compiler::compile_source_with_expand_and_session(
            path,
            &text,
            expand_mode.clone(),
            &mut file_session,
        );
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
