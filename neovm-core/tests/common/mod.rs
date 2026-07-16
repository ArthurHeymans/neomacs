use std::cell::RefCell;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use neovm_core::emacs_core::format_eval_result_with_eval;
use neovm_core::emacs_core::load::{
    apply_runtime_startup_state, create_bootstrap_evaluator_cached,
};
use neovm_core::emacs_core::pdump;

thread_local! {
    static RUNTIME_TEMPLATE: RefCell<Option<neovm_core::emacs_core::pdump::types::DumpContextState>> =
        const { RefCell::new(None) };
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn oracle_emacs_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("NEOVM_FORCE_ORACLE_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = manifest.clone();
    for _ in 0..4 {
        let candidate = dir.join("emacs-mirror/emacs/src/emacs");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }

    None
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn oracle_enabled() -> bool {
    oracle_emacs_path().is_some()
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn gnu_window_c_path() -> Option<PathBuf> {
    let mut dir = repo_root();
    for _ in 0..5 {
        let candidate = dir.join("emacs-mirror/emacs/src/window.c");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn elisp_string(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn write_temp_elisp_file(
    prefix: &str,
    suffix: &str,
    content: &str,
) -> Result<tempfile::TempPath, String> {
    let mut file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile()
        .map_err(|e| format!("failed to create temp Elisp file: {e}"))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to write temp Elisp file: {e}"))?;
    file.flush()
        .map_err(|e| format!("failed to flush temp Elisp file: {e}"))?;
    Ok(file.into_temp_path())
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn run_oracle_eval(form: &str) -> Result<String, String> {
    let Some(oracle_bin) = oracle_emacs_path() else {
        return Err("GNU Emacs oracle binary not found".to_string());
    };

    let form_path = write_temp_elisp_file("neovm-oracle-form-", ".el", form)?;
    let program = r#"(condition-case err
    (let ((source-buf (generate-new-buffer " *neovm-oracle-form*"))
          (last nil))
      (unwind-protect
          (progn
            (with-current-buffer source-buf
              (insert-file-contents (getenv "NEOVM_ORACLE_FORM_FILE"))
              (goto-char (point-min)))
            (condition-case nil
                (while t
                  (setq last (eval (read source-buf) t)))
              (end-of-file last))
            (princ (concat "OK " (prin1-to-string last))))
        (when (buffer-live-p source-buf)
          (kill-buffer source-buf)))))
  (error
   (princ (concat "ERR " (prin1-to-string err)))))"#;

    let mem_limit_mb = std::env::var("NEOVM_ORACLE_MEM_LIMIT_MB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(500);
    let mem_limit_bytes = mem_limit_mb * 1024 * 1024;

    let mut cmd = Command::new(oracle_bin);
    cmd.env("NEOVM_ORACLE_FORM_FILE", form_path.as_os_str())
        .env("EMACSNATIVELOADPATH", "/dev/null")
        .args([
            "--batch",
            "-Q",
            "--eval",
            "(setq native-comp-jit-compilation nil inhibit-automatic-native-compilation t native-comp-enable-subr-trampolines nil)",
            "--eval",
            program,
        ]);

    unsafe {
        cmd.pre_exec(move || {
            let rlim = libc::rlimit {
                rlim_cur: mem_limit_bytes as libc::rlim_t,
                rlim_max: mem_limit_bytes as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run GNU Emacs oracle: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "GNU Emacs oracle failed: status={}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub fn run_neovm_eval(form: &str) -> Result<String, String> {
    let mut eval = RUNTIME_TEMPLATE.with(|slot| {
        if slot.borrow().is_none() {
            let mut template = create_bootstrap_evaluator_cached()
                .map_err(|_| "NeoVM bootstrap failed".to_string())?;
            apply_runtime_startup_state(&mut template)
                .map_err(|_| "NeoVM runtime startup failed".to_string())?;
            *slot.borrow_mut() = Some(pdump::snapshot_evaluator(&template));
        }

        let template = slot.borrow();
        let snapshot = template
            .as_ref()
            .ok_or_else(|| "NeoVM runtime template unavailable".to_string())?;
        pdump::restore_snapshot(snapshot).map_err(|e| format!("NeoVM runtime clone failed: {e}"))
    })?;
    eval.set_lexical_binding(true);
    let result = eval.eval_str(form);
    Ok(format_eval_result_with_eval(&eval, &result))
}

/// Collect every `ctx.defsubr("name", target, ...)` registration under
/// `src_root` (recursively, skipping test files) into a name -> target map.
///
/// Registrations used to live in one flat `builtins::init_builtins`; since
/// the per-module `syms_of_*` decomposition (docs/design/neovm-core-layout.md)
/// they sit next to the code they register, so surface audits must scan the
/// whole tree rather than builtins/mod.rs alone.
#[allow(dead_code)] // used by the compat_*_surface audits only
pub fn collect_defsubr_targets(src_root: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let mut stack = vec![src_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.ends_with(".rs") || name.contains("_test") || name == "tests.rs" {
                continue;
            }
            parse_defsubr_targets_into(&path, &mut out);
        }
    }
    out
}

#[allow(dead_code)]
fn parse_defsubr_targets_into(path: &Path, out: &mut std::collections::BTreeMap<String, String>) {
    let source = std::fs::read_to_string(path).expect("read source file");
    let mut block = String::new();
    let mut capturing = false;
    for line in source.lines() {
        if !capturing {
            if let Some(idx) = line.find("ctx.defsubr(") {
                capturing = true;
                block.clear();
                block.push_str(&line[idx..]);
                block.push('\n');
                if line.contains(");")
                    && let Some((name, target)) = extract_defsubr_name_and_target(&block)
                {
                    out.insert(name, target);
                    block.clear();
                    capturing = false;
                }
            }
            continue;
        }
        block.push_str(line);
        block.push('\n');
        if line.contains(");")
            && let Some((name, target)) = extract_defsubr_name_and_target(&block)
        {
            out.insert(name, target);
            block.clear();
            capturing = false;
        }
    }

    // Also pick up entries registered via
    // `register_builtin(ctx, BuiltinRegistration::...("name", target, ...))` —
    // eval-state-aware primitives that don't go through ctx.defsubr. The
    // target may be a path identifier or a closure; accept both.
    let re = regex::Regex::new(
        r#"BuiltinRegistration::[A-Za-z0-9_]+\(\s*"([^"]+)"\s*,\s*((?:\|[^|]*\|\s*)?[^,)]+)"#,
    )
    .expect("builtin reg regex");
    for caps in re.captures_iter(&source) {
        let name = caps[1].to_string();
        let target = caps[2].trim().to_string();
        out.entry(name).or_insert(target);
    }
}

#[allow(dead_code)]
fn extract_defsubr_name_and_target(block: &str) -> Option<(String, String)> {
    let re = regex::Regex::new(r#"ctx\.defsubr\(\s*"([^"]+)""#).expect("defsubr regex");
    let caps = re.captures(block)?;
    let name = caps.get(1)?.as_str().to_string();
    let full = caps.get(0)?;
    let rest = &block[full.end()..];
    let comma = rest.find(',')?;
    let mut target = String::new();
    let mut paren_depth = 0usize;
    let mut in_pipe = false;
    let mut started = false;
    for ch in rest[comma + 1..].chars() {
        if !started && ch.is_whitespace() {
            continue;
        }
        started = true;
        match ch {
            '|' if paren_depth == 0 => {
                in_pipe = !in_pipe;
                target.push(ch);
            }
            '(' if !in_pipe => {
                paren_depth += 1;
                target.push(ch);
            }
            ')' if !in_pipe => {
                paren_depth = paren_depth.saturating_sub(1);
                target.push(ch);
            }
            ',' if !in_pipe && paren_depth == 0 => break,
            _ => target.push(ch),
        }
    }
    Some((
        name,
        target.split_whitespace().collect::<Vec<_>>().join(" "),
    ))
}
