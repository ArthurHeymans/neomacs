use super::*;

fn workspace_temp_dir() -> tempfile::TempDir {
    let parent = std::path::Path::new(env!("CARGO_WORKSPACE_DIR"))
        .join("target")
        .join("neovm-core-file-notify-tests");
    std::fs::create_dir_all(&parent).expect("create workspace test directory");
    tempfile::Builder::new()
        .prefix("inotify-")
        .tempdir_in(parent)
        .expect("create file notification fixture")
}

/// GNU passes `IN_ONLYDIR` to the kernel, which rejects a regular file with
/// ENOTDIR.  Accepting the flag but ignoring it is observably incompatible and
/// can make callers believe a directory-only invariant is enforced when it is
/// not.
#[test]
fn inotify_onlydir_rejects_a_regular_file() {
    reset_file_notify_thread_locals();
    let directory = workspace_temp_dir();
    let file = directory.path().join("regular-file");
    std::fs::write(&file, "contents").expect("seed regular file");
    let mut eval = crate::test_utils::runtime_startup_context();

    let result = inotify_add_watch(
        &mut eval,
        vec![
            Value::string(file.display().to_string()),
            Value::list(vec![Value::symbol("onlydir")]),
            Value::symbol("ignore"),
        ],
    );

    assert!(
        result.is_err(),
        "GNU inotify-add-watch rejects a non-directory when onlydir is requested"
    );
    reset_file_notify_thread_locals();
}
