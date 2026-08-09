use super::*;

#[cfg(unix)]
#[test]
fn first_text_change_locks_a_clean_file_visiting_buffer_like_gnu() {
    crate::test_utils::init_test_tracing();
    let root = std::env::current_dir()
        .expect("workspace directory")
        .join("tmp/neovm-core-test-artifacts")
        .join(format!("first-change-lock-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create workspace-local fixture directory");
    let visited = root.join("visited.txt");
    let lock = root.join(".#visited.txt");
    fs::write(&visited, b"before\n").expect("write visited file");
    let visited_value = Value::string(visited.to_string_lossy());

    let mut eval = super::super::eval::Context::new();
    eval.set_variable("create-lockfiles", Value::T);
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .set_buffer_file_name(current, visited_value)
        .expect("set buffer-file-name");
    eval.buffers
        .set_buffer_file_truename(current, visited_value)
        .expect("set buffer-file-truename");

    super::super::editfns::insert_lisp_string_with_change_hooks_in_buffer(
        &mut eval,
        current,
        &LispString::from_utf8("changed"),
    )
    .expect("modify visiting buffer");

    assert!(
        fs::symlink_metadata(&lock).is_ok(),
        "GNU locks a clean file-visiting buffer before its first text change"
    );

    let _ = fs::remove_file(&lock);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn lock_and_unlock_file_dispatch_matching_file_name_handlers_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = super::super::eval::Context::new();

    let result = eval.eval_str(
        r#"(progn
             (setq neovm-file-lock-handler-calls nil)
             (setq file-name-handler-alist
                   (list
                    (cons "\\`/remote:"
                          (lambda (operation &rest arguments)
                            (setq neovm-file-lock-handler-calls
                                  (cons (cons operation arguments)
                                        neovm-file-lock-handler-calls))
                            (if (eq operation 'file-locked-p)
                                :remote-owner
                              :handled)))))
             (list (lock-file "/remote:host:/work/note.txt")
                   (file-locked-p "/remote:host:/work/note.txt")
                   (unlock-file "/remote:host:/work/note.txt")
                   (reverse neovm-file-lock-handler-calls)))"#,
    );

    assert_eq!(
        crate::emacs_core::format_eval_result(&result),
        "OK (:handled :remote-owner nil ((lock-file \"/remote:host:/work/note.txt\") (file-locked-p \"/remote:host:/work/note.txt\") (unlock-file \"/remote:host:/work/note.txt\")))"
    );
}

#[test]
fn new_lock_info_contains_gnu_boot_time_suffix_when_available() {
    let lock_info = current_lock_info_string();
    let parsed = parse_lock_info(&lock_info).expect("parse current lock info");
    assert_eq!(parsed.pid, std::process::id());
    assert_eq!(parsed.boot_time, system_boot_time_sec());
}

#[test]
fn zero_is_never_a_valid_lock_owner_pid() {
    assert!(!process_is_alive(0));
}

#[cfg(windows)]
#[test]
fn windows_process_probe_recognizes_current_process() {
    assert!(process_is_alive(std::process::id()));
}

#[cfg(unix)]
#[test]
fn current_lock_owner_recognizes_dangling_symlink_lockfiles() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lock_path = dir.path().join(".#probe");
    std::os::unix::fs::symlink(current_lock_info_string(), &lock_path)
        .expect("create lock symlink");

    assert!(matches!(
        current_lock_owner(&lock_path).expect("read lock owner"),
        LockOwner::Current
    ));
}

#[cfg(unix)]
#[test]
fn dead_pid_lock_on_this_host_is_zapped_and_reported_free() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lock_path = dir.path().join(".#stale");
    // A pid from a crashed session: pid 1 is init (alive, but use an
    // impossible one). Recycle-proof choice: our own pid is alive, so use
    // a pid that cannot exist (> pid_max default of 4194304).
    let contents = format!("someone@{}.999999999", current_host_name());
    std::os::unix::fs::symlink(&contents, &lock_path).expect("symlink lock");
    match current_lock_owner(&lock_path).expect("owner check") {
        LockOwner::None => {}
        LockOwner::Current => panic!("stale lock cannot be ours"),
        LockOwner::Other(user) => panic!("stale lock must be zapped, got owner {user}"),
    }
    assert!(
        std::fs::symlink_metadata(&lock_path).is_err(),
        "GNU unlinks the stale lockfile in current_lock_owner"
    );
}

#[cfg(unix)]
#[test]
fn live_pid_lock_on_this_host_names_the_other_owner() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lock_path = dir.path().join(".#live");
    // pid 1 is always alive; kill(1, 0) fails with EPERM for non-root,
    // which GNU treats as alive.
    let contents = format!("someone@{}.1", current_host_name());
    std::os::unix::fs::symlink(&contents, &lock_path).expect("symlink lock");
    match current_lock_owner(&lock_path).expect("owner check") {
        LockOwner::Other(user) => assert_eq!(user, "someone"),
        _ => panic!("live-pid lock must report the other owner"),
    }
    assert!(
        std::fs::symlink_metadata(&lock_path).is_ok(),
        "live locks are never zapped"
    );
}

#[cfg(unix)]
#[test]
fn stale_boot_time_zaps_even_a_live_pid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lock_path = dir.path().join(".#reboot");
    // pid 1 alive, but a boot time of 12 (1970) cannot match this boot.
    let contents = format!("someone@{}.1:12", current_host_name());
    std::os::unix::fs::symlink(&contents, &lock_path).expect("symlink lock");
    if system_boot_time_sec() == 0 {
        return; // GNU also omits the comparison when boot time is unavailable.
    }
    match current_lock_owner(&lock_path).expect("owner check") {
        LockOwner::None => {}
        _ => panic!("previous-boot lock must be stale"),
    }
}
