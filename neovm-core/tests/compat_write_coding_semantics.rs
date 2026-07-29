mod common;

use common::{oracle_enabled, run_neovm_eval, run_oracle_eval};

#[test]
fn compat_write_region_shift_jis_bytes_match_gnu_emacs() {
    if !oracle_enabled() {
        eprintln!(
            "skipping write coding audit: set NEOVM_FORCE_ORACLE_PATH or place GNU Emacs mirror alongside the repo"
        );
        return;
    }

    let form = r#"(let ((file (make-temp-file "compat-write-shift-jis-")))
  (unwind-protect
      (with-temp-buffer
        (insert "日本\n")
        (setq buffer-file-coding-system 'japanese-shift-jis-unix)
        (write-region nil nil file nil 'silent)
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert-file-contents-literally file)
          (list last-coding-system-used
                (length (buffer-string))
                (string-to-list (buffer-string)))))
    (delete-file file)))"#;
    let gnu = run_oracle_eval(form).expect("GNU Emacs evaluation");
    let neovm = run_neovm_eval(form).expect("NeoVM evaluation");
    assert_eq!(
        neovm, gnu,
        "write-region Shift-JIS semantics mismatch:\nGNU: {gnu}\nNeoVM: {neovm}"
    );
}
