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

/// GNU gets the boot time from the utmp BOOT_TIME record (gnulib
/// boot-time.c), which systemd stamps seconds LATER than the kernel's
/// /proc/stat btime.  current_lock_owner's staleness check tolerates only
/// one second of skew, so a boot time from any other source makes neomacs
/// zap live GNU locks as stale.  Pin the source, not just the concept.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn boot_time_source_is_the_utmp_boot_record_like_gnu() {
    let Some(utmp_boot) = boot_time_from_utmp_sec() else {
        return; // No utmp boot record: the fallback path is all we have.
    };
    assert_eq!(
        system_boot_time_sec(),
        utmp_boot,
        "boot time must come from the utmp BOOT_TIME record, as GNU's does"
    );
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
        LockOwner::Other(clasher) => panic!("stale lock must be zapped, got owner {clasher:?}"),
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
        LockOwner::Other(clasher) => {
            assert_eq!(clasher.user, "someone");
            assert_eq!(clasher.pid, 1);
            assert_eq!(
                clasher.opponent(),
                format!("someone@{} (pid 1)", current_host_name())
            );
        }
        _ => panic!("live-pid lock must report the other owner"),
    }
    assert!(
        std::fs::symlink_metadata(&lock_path).is_ok(),
        "live locks are never zapped"
    );
}

/// Spawn a definitely-live process this user owns, so liveness probes never
/// depend on pid-1 EPERM subtleties.
#[cfg(unix)]
fn spawn_live_owner() -> std::process::Child {
    std::process::Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("spawn sleep child")
}

#[cfg(unix)]
fn visit_file_in_current_buffer(eval: &mut super::super::eval::Context, visited: &Path) {
    let visited_value = Value::string(visited.to_string_lossy());
    eval.set_variable("create-lockfiles", Value::T);
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .set_buffer_file_name(current, visited_value)
        .expect("set buffer-file-name");
    eval.buffers
        .set_buffer_file_truename(current, visited_value)
        .expect("set buffer-file-truename");
}

/// GNU lock_file (src/filelock.c) calls ask-user-about-lock when another
/// process owns the lock, and any signal it raises — the batch-mode
/// file-locked signal from userlock.el in particular — propagates and
/// aborts the modification.
#[cfg(unix)]
#[test]
fn modifying_externally_locked_file_propagates_file_locked_and_leaves_buffer_untouched() {
    crate::test_utils::init_test_tracing();
    let dir = tempfile::tempdir().expect("tempdir");
    let visited = dir.path().join("note.txt");
    fs::write(&visited, b"hello\n").expect("write visited file");
    let lock_path = dir.path().join(".#note.txt");
    let mut owner = spawn_live_owner();

    let mut eval = super::super::eval::Context::new();
    visit_file_in_current_buffer(&mut eval, &visited);
    // Load the visited contents like find-file would, then mark the buffer
    // clean so the contested lock governs the NEXT (first) modification.
    eval.eval_str(r#"(progn (insert "hello\n") (set-buffer-modified-p nil))"#)
        .expect("seed visited buffer contents");
    let contents = format!("someone@{}.{}", current_host_name(), owner.id());
    std::os::unix::fs::symlink(&contents, &lock_path).expect("symlink lock");
    eval.eval_str(
        r#"(fset 'ask-user-about-lock
               (lambda (file opponent)
                 (signal 'file-locked
                         (list file opponent "Cannot resolve lock conflict in batch mode"))))"#,
    )
    .expect("define batch ask-user-about-lock");

    let result = eval.eval_str(r#"(insert "EDIT")"#);
    let formatted = crate::emacs_core::format_eval_result(&result);
    assert_eq!(
        formatted,
        format!(
            "ERR (file-locked (\"{}\" \"someone@{} (pid {})\" \"Cannot resolve lock conflict in batch mode\"))",
            visited.display(),
            current_host_name(),
            owner.id(),
        ),
        "GNU propagates the ask-user-about-lock signal and refuses the edit"
    );

    let buffer_after = eval.eval_str("(buffer-string)");
    assert_eq!(
        crate::emacs_core::format_eval_result(&buffer_after),
        "OK \"hello\n\"",
        "a refused edit must not modify the buffer"
    );
    assert_eq!(
        fs::read_link(&lock_path)
            .expect("lock survives")
            .to_string_lossy(),
        contents,
        "a refused edit must not steal the other process's lock"
    );

    let _ = owner.kill();
    let _ = owner.wait();
}

/// GNU lock_file rewrites the clasher info USER@HOST.PID:BOOT into
/// "USER@HOST (pid PID)" before handing it to ask-user-about-lock.
#[cfg(unix)]
#[test]
fn ask_user_about_lock_steal_and_proceed_answers_match_gnu() {
    crate::test_utils::init_test_tracing();
    let dir = tempfile::tempdir().expect("tempdir");
    let visited = dir.path().join("note.txt");
    fs::write(&visited, b"hello\n").expect("write visited file");
    let lock_path = dir.path().join(".#note.txt");
    let mut owner = spawn_live_owner();
    let contents = format!(
        "someone@{}.{}:{}",
        current_host_name(),
        owner.id(),
        system_boot_time_sec().max(1),
    );

    // Answer nil: proceed without taking the lock.
    std::os::unix::fs::symlink(&contents, &lock_path).expect("symlink lock");
    let mut eval = super::super::eval::Context::new();
    visit_file_in_current_buffer(&mut eval, &visited);
    eval.eval_str(
        r#"(progn
             (setq neovm-lock-args nil)
             (fset 'ask-user-about-lock
                   (lambda (file opponent)
                     (setq neovm-lock-args (list file opponent))
                     nil)))"#,
    )
    .expect("define recording ask-user-about-lock");
    eval.eval_str(r#"(insert "EDIT")"#)
        .expect("proceed answer edits anyway");
    assert_eq!(
        crate::emacs_core::format_eval_result(&eval.eval_str("neovm-lock-args")),
        format!(
            "OK (\"{}\" \"someone@{} (pid {})\")",
            visited.display(),
            current_host_name(),
            owner.id(),
        ),
        "opponent string must be USER@HOST (pid PID) with the boot time stripped"
    );
    assert_eq!(
        fs::read_link(&lock_path)
            .expect("lock survives")
            .to_string_lossy(),
        contents,
        "answer nil edits the file but leaves the other lock in place"
    );

    // Answer t: steal the lock, then edit.
    let mut eval = super::super::eval::Context::new();
    visit_file_in_current_buffer(&mut eval, &visited);
    eval.eval_str(r#"(fset 'ask-user-about-lock (lambda (file opponent) t))"#)
        .expect("define stealing ask-user-about-lock");
    eval.eval_str(r#"(insert "EDIT")"#)
        .expect("steal answer edits");
    assert_eq!(
        fs::read_link(&lock_path)
            .expect("stolen lock")
            .to_string_lossy(),
        current_lock_info_string(),
        "answer t forces the lock over to us"
    );

    let _ = owner.kill();
    let _ = owner.wait();
}

/// GNU current_lock_owner returns EINVAL for unparseable lock contents;
/// lock_file deliberately ignores that errno (no prompt, edit proceeds),
/// while file-locked-p reports it as a file-error.
#[cfg(unix)]
#[test]
fn unparseable_lock_contents_are_an_error_not_another_owner() {
    crate::test_utils::init_test_tracing();
    let dir = tempfile::tempdir().expect("tempdir");
    let visited = dir.path().join("note.txt");
    fs::write(&visited, b"hello\n").expect("write visited file");
    let lock_path = dir.path().join(".#note.txt");
    std::os::unix::fs::symlink("complete garbage", &lock_path).expect("symlink lock");

    let mut eval = super::super::eval::Context::new();
    visit_file_in_current_buffer(&mut eval, &visited);
    eval.eval_str(
        r#"(fset 'ask-user-about-lock
               (lambda (file opponent)
                 (signal 'file-locked (list file opponent))))"#,
    )
    .expect("define signalling ask-user-about-lock");

    eval.eval_str(r#"(insert "EDIT")"#)
        .expect("GNU ignores the EINVAL from lock_if_free and never prompts");

    let locked_p = eval.eval_str(&format!("(file-locked-p \"{}\")", visited.display()));
    assert!(
        crate::emacs_core::format_eval_result(&locked_p).starts_with("ERR (file-error"),
        "GNU file-locked-p reports EINVAL via report_file_errno, got {}",
        crate::emacs_core::format_eval_result(&locked_p),
    );
}

/// GNU zaps an empty lock file (buggy-filesystem leftover) and reports the
/// file free.
#[cfg(unix)]
#[test]
fn empty_lock_file_is_zapped_and_reported_free() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lock_path = dir.path().join(".#empty");
    fs::write(&lock_path, b"").expect("write empty lock");
    match current_lock_owner(&lock_path).expect("owner check") {
        LockOwner::None => {}
        _ => panic!("empty lock file must be zapped and reported free"),
    }
    assert!(
        fs::symlink_metadata(&lock_path).is_err(),
        "GNU unlinks the empty lock file"
    );
}

/// GNU Ffile_locked_p returns only the USER part for another owner.
#[cfg(unix)]
#[test]
fn file_locked_p_names_only_the_user_for_another_owner() {
    crate::test_utils::init_test_tracing();
    let dir = tempfile::tempdir().expect("tempdir");
    let visited = dir.path().join("note.txt");
    fs::write(&visited, b"hello\n").expect("write visited file");
    let lock_path = dir.path().join(".#note.txt");
    let mut owner = spawn_live_owner();
    let contents = format!("someone@{}.{}", current_host_name(), owner.id());
    std::os::unix::fs::symlink(&contents, &lock_path).expect("symlink lock");

    let mut eval = super::super::eval::Context::new();
    let locked_p = eval.eval_str(&format!("(file-locked-p \"{}\")", visited.display()));
    assert_eq!(
        crate::emacs_core::format_eval_result(&locked_p),
        "OK \"someone\"",
    );

    let _ = owner.kill();
    let _ = owner.wait();
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
