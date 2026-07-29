mod common;

use common::{oracle_enabled, run_neovm_eval, run_oracle_eval};

#[test]
fn batch_keyboard_macro_drives_character_readers_like_gnu_emacs() {
    if !oracle_enabled() {
        eprintln!(
            "skipping batch keyboard-macro character-input audit: set NEOVM_FORCE_ORACLE_PATH or place GNU Emacs mirror alongside the repo"
        );
        return;
    }

    let form = r#"(progn
  (defun neovm-test-read-char ()
    (interactive)
    (setq neovm-test-read-char-result
          (list (read-char "char: ")
                (key-description (this-command-keys)))))
  (defun neovm-test-read-event ()
    (interactive)
    (setq neovm-test-read-event-result
          (list (read-event "event: ")
                (key-description (this-command-keys)))))
  (defun neovm-test-read-char-exclusive ()
    (interactive)
    (setq neovm-test-read-char-exclusive-result
          (list (read-char-exclusive "exclusive: ")
                (key-description (this-command-keys)))))
  (global-set-key (kbd "C-c c") #'neovm-test-read-char)
  (global-set-key (kbd "C-c e") #'neovm-test-read-event)
  (global-set-key (kbd "C-c x") #'neovm-test-read-char-exclusive)
  (execute-kbd-macro (kbd "C-c c a"))
  (execute-kbd-macro (kbd "C-c e b"))
  (execute-kbd-macro (kbd "C-c x c"))
  (list neovm-test-read-char-result
        neovm-test-read-event-result
        neovm-test-read-char-exclusive-result))"#;

    let gnu = run_oracle_eval(form).expect("GNU Emacs evaluation");
    let neovm = run_neovm_eval(form).expect("NeoVM evaluation");

    assert_eq!(
        neovm, gnu,
        "batch keyboard-macro character reader semantics differ from GNU Emacs"
    );
}
