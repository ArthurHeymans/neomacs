use std::env;
use std::path::PathBuf;
use std::process::Command;

fn git_stdout(workspace_root: &PathBuf, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(arguments)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
}

fn main() {
    let workspace_root = PathBuf::from(
        env::var_os("CARGO_WORKSPACE_DIR")
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).into_os_string()),
    );
    let revision = git_stdout(&workspace_root, &["rev-parse", "HEAD"])
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=NEOMACS_PERF_GIT_SHA={revision}");

    if let Some(git_head) = git_stdout(&workspace_root, &["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={git_head}");
    }
    if let Some(reference) = git_stdout(&workspace_root, &["symbolic-ref", "-q", "HEAD"])
        && let Some(reference_path) =
            git_stdout(&workspace_root, &["rev-parse", "--git-path", &reference])
    {
        println!("cargo:rerun-if-changed={reference_path}");
    }
}
