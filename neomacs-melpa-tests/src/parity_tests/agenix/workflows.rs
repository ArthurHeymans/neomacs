use expect_test::expect;

use super::assert_agenix_parity;

/// Opening a secret.  `auto-mode-alist' routes the `.age' file to
/// `agenix-mode-if-with-secrets-nix', which finds the `secrets.nix' above it,
/// asks a real `nix-instantiate' for that secret's recipients, and runs `age'
/// with the user's identity.  The argument vector is pinned in full: the
/// package builds `--decrypt --identity <key> <file>' itself, so a lost flag or
/// a file passed on stdin instead of by path is a failing test.  What the user
/// ends up with is a writable buffer holding the plaintext, with saving
/// diverted to the package and auto-save switched off so no plaintext copy can
/// appear beside the secret.
#[test]
fn decrypts_a_secret_into_a_buffer_when_the_file_is_opened() {
    let elisp_form = r##"
        (progn
          (agx-test-install-age)
          (agx-test-project)
          (let ((key (agx-test-keygen "id_ed25519")))
            (agx-test-authorize key)
            (setq agenix-key-files (list key))
            (agx-test-encrypt-fixture "db-password.age" "DB_PASSWORD=hunter2\n")
            (agx-test-open "db-password.age")
            (list :state (agx-test-state)
                  :recipients agenix--keys
                  :encrypted-file (file-name-nondirectory agenix--encrypted-fp)
                  :directory (agx-test-entries)
                  :age-runs (agx-test-run-count)
                  :recorded (agx-test-records))))
    "##;

    let expect = expect![[
        r#"OK (:state (:mode agenix-mode :read-only nil :modified nil :point 1 :buffer "DB_PASSWORD=hunter2\n" :write-contents-functions (agenix-save-decrypted) :auto-save nil) :recipients ("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB1alicealicealicealicealicealiceal alice@example" "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB2bobbobbobbobbobbobbobbobbobbobbo bob@example") :encrypted-file "db-password.age" :directory ("." ".." "db-password.age" "secrets.nix") :age-runs 1 :recorded (("01-age" . "argv:\n  --decrypt\n  --identity\n  [ORACLE-SANDBOX]/agenix/keys/id_ed25519\n  [ORACLE-SANDBOX]/agenix/project/db-password.age\ncwd: [ORACLE-SANDBOX]/agenix/project\nstdin: <empty>\n")))"#
    ]];

    assert_agenix_parity(elisp_form, expect);
}

/// The round trip.  The user rewrites the secret and saves; the package pipes
/// the buffer through `age --encrypt' with one `--recipient' per key from
/// `secrets.nix' and `-o' naming the same path, then reverts, which decrypts
/// again -- so what comes back is what was typed, trailing newline included.
/// Three things are pinned besides the argv: the plaintext travels on stdin and
/// never appears anywhere in the secrets directory, the file on disk is
/// ciphertext, and the lock file Emacs puts beside the secret while the buffer
/// is modified is gone once the save finishes.
#[test]
fn re_encrypts_on_save_without_the_plaintext_reaching_disk() {
    let elisp_form = r##"
        (progn
          (agx-test-install-age)
          (agx-test-project)
          (let ((key (agx-test-keygen "id_ed25519")))
            (agx-test-authorize key)
            (setq agenix-key-files (list key))
            (agx-test-encrypt-fixture "db-password.age" "DB_PASSWORD=hunter2\n")
            (agx-test-open "db-password.age")
            (delete-region (point-min) (point-max))
            (execute-kbd-macro "DB_PASSWORD=correct-horse-battery-staple")
            (execute-kbd-macro (kbd "RET"))
            (execute-kbd-macro "DB_HOST=db.internal")
            (execute-kbd-macro (kbd "RET"))
            (list
             :while-editing (list :state (agx-test-state)
                                  :directory (agx-test-entries))
             :saved (progn (save-buffer)
                           (list :state (agx-test-state)
                                 :directory (agx-test-entries)
                                 :age-runs (agx-test-run-count)))
             :on-disk (list :ciphertext
                            (agx-test-file-text
                             (expand-file-name "db-password.age" agx-test-root))
                            :plaintext-anywhere
                            (agx-test-plaintext-on-disk-p "correct-horse-battery-staple"))
             :recorded (agx-test-records))))
    "##;

    let expect = expect![[
        r#"OK (:while-editing (:state (:mode agenix-mode :read-only nil :modified t :point 62 :buffer "DB_PASSWORD=correct-horse-battery-staple\nDB_HOST=db.internal\n" :write-contents-functions #1=(agenix-save-decrypted) :auto-save nil) :directory ("." ".#db-password.age" ".." "db-password.age" "secrets.nix")) :saved (:state (:mode agenix-mode :read-only nil :modified nil :point 62 :buffer "DB_PASSWORD=correct-horse-battery-staple\nDB_HOST=db.internal\n" :write-contents-functions #1# :auto-save nil) :directory ("." ".." "db-password.age" "secrets.nix") :age-runs 3) :on-disk (:ciphertext "-----BEGIN AGE ENCRYPTED FILE-----\nREJfUEFTU1dPUkQ9Y29ycmVjdC1ob3JzZS1iYXR0ZXJ5LXN0YXBsZQpEQl9IT1NUPWRiLmludGVy\nbmFsCg==\n-----END AGE ENCRYPTED FILE-----\n" :plaintext-anywhere nil) :recorded (("01-age" . "argv:\n  --decrypt\n  --identity\n  [ORACLE-SANDBOX]/agenix/keys/id_ed25519\n  [ORACLE-SANDBOX]/agenix/project/db-password.age\ncwd: [ORACLE-SANDBOX]/agenix/project\nstdin: <empty>\n") ("02-age" . "argv:\n  --encrypt\n  --recipient\n  ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB1alicealicealicealicealicealiceal alice@example\n  --recipient\n  ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB2bobbobbobbobbobbobbobbobbobbobbo bob@example\n  -o\n  [ORACLE-SANDBOX]/agenix/project/db-password.age\ncwd: [ORACLE-SANDBOX]/agenix/project\nstdin:\nDB_PASSWORD=correct-horse-battery-staple\nDB_HOST=db.internal\n") ("03-age" . "argv:\n  --decrypt\n  --identity\n  [ORACLE-SANDBOX]/agenix/keys/id_ed25519\n  [ORACLE-SANDBOX]/agenix/project/db-password.age\ncwd: [ORACLE-SANDBOX]/agenix/project\nstdin: <empty>\n")))"#
    ]];

    assert_agenix_parity(elisp_form, expect);
}

/// `agenix-key-files' is the customization a user actually sets.  Entries may
/// be paths or functions returning paths, `~' is expanded, and anything that
/// does not exist is dropped -- so only the surviving identities become
/// `--identity' flags, in the order given.  A second workflow detail falls out
/// of it: because every surviving key here is a real unprotected ed25519 key,
/// `ssh-keygen' says none is password protected and the package takes its
/// non-interactive path without prompting.
#[test]
fn key_files_customization_decides_the_identity_flags() {
    let elisp_form = r##"
        (progn
          (agx-test-install-age)
          (agx-test-project)
          (let* ((primary (agx-test-keygen "id_ed25519"))
                 (backup (agx-test-keygen "id_backup"))
                 (absent (expand-file-name "id_absent" agx-test-keys)))
            (agx-test-authorize primary backup)
            (setq agenix-key-files
                  (list primary (lambda () backup) absent "~/no-such-key"))
            (agx-test-encrypt-fixture "db-password.age" "API_TOKEN=abc123\n")
            (agx-test-open "db-password.age")
            (list :configured (length agenix-key-files)
                  :resolved (mapcar #'file-name-nondirectory
                                    (agenix--process-agenix-key-files))
                  :none-password-protected
                  (seq-every-p (lambda (path)
                                 (not (agenix--identity-protected-p path)))
                               (agenix--process-agenix-key-files))
                  :state (agx-test-state)
                  :recorded (agx-test-records))))
    "##;

    let expect = expect![[
        r#"OK (:configured 4 :resolved ("id_ed25519" "id_backup") :none-password-protected t :state (:mode agenix-mode :read-only nil :modified nil :point 1 :buffer "API_TOKEN=abc123\n" :write-contents-functions (agenix-save-decrypted) :auto-save nil) :recorded (("01-age" . "argv:\n  --decrypt\n  --identity\n  [ORACLE-SANDBOX]/agenix/keys/id_ed25519\n  --identity\n  [ORACLE-SANDBOX]/agenix/keys/id_backup\n  [ORACLE-SANDBOX]/agenix/project/db-password.age\ncwd: [ORACLE-SANDBOX]/agenix/project\nstdin: <empty>\n")))"#
    ]];

    assert_agenix_parity(elisp_form, expect);
}

/// The everyday failure: the secret was encrypted for someone else, so `age'
/// exits non-zero.  The package raises its own error from inside
/// `agenix-mode', which `normal-mode' catches and reports, so the user is left
/// looking at a read-only buffer of the raw armoured ciphertext rather than an
/// empty or half-decrypted one.  age's own stderr is quoted back in the
/// message, the file on disk is untouched, and no lock file is left behind.
#[test]
fn reports_the_failure_and_shows_ciphertext_when_no_key_matches() {
    let elisp_form = r##"
        (progn
          (agx-test-install-age)
          (agx-test-project)
          (let ((wrong (agx-test-keygen "id_wrong")))
            (agx-test-authorize "/no-identity-is-authorised")
            (setq agenix-key-files (list wrong))
            (agx-test-encrypt-fixture "db-password.age" "TOP_SECRET=do-not-leak\n")
            (agx-test-open "db-password.age")
            (list :state (agx-test-state)
                  :messages (agx-test-messages "Decryption failed")
                  :directory (agx-test-entries)
                  :plaintext-anywhere (agx-test-plaintext-on-disk-p "do-not-leak")
                  :recorded (agx-test-records))))
    "##;

    let expect = expect![[
        r#"OK (:state (:mode agenix-mode :read-only t :modified nil :point 1 :buffer "-----BEGIN AGE ENCRYPTED FILE-----\nVE9QX1NFQ1JFVD1kby1ub3QtbGVhawo=\n-----END AGE ENCRYPTED FILE-----\n" :write-contents-functions nil :auto-save nil) :messages ("File mode specification error: (error \"Decryption failed: age: error: no identity matched any of the recipients\\n. Please close the buffer and try again\")") :directory ("." ".." "db-password.age" "secrets.nix") :plaintext-anywhere nil :recorded (("01-age" . "argv:\n  --decrypt\n  --identity\n  [ORACLE-SANDBOX]/agenix/keys/id_wrong\n  [ORACLE-SANDBOX]/agenix/project/db-password.age\ncwd: [ORACLE-SANDBOX]/agenix/project\nstdin: <empty>\n")))"#
    ]];

    assert_agenix_parity(elisp_form, expect);
}

/// Two ways `secrets.nix' and the filesystem disagree, both of which stop the
/// package before it ever runs `age'.  A `.age' file that `secrets.nix' does
/// not declare produces a warning naming the file, with Nix's own evaluation
/// error quoted underneath.  A secret that *is* declared but does not exist yet
/// is the normal way to create one: the buffer is left empty and writable with
/// a message saying it will be created on save.
#[test]
fn refuses_to_decrypt_an_undeclared_secret_and_prepares_a_new_one() {
    let elisp_form = r##"
        (progn
          (agx-test-install-age)
          (agx-test-project)
          (let ((key (agx-test-keygen "id_ed25519")))
            (agx-test-authorize key)
            (setq agenix-key-files (list key))
            (agx-test-encrypt-fixture "undeclared.age" "orphan\n")
            (list
             :undeclared (progn
                           (agx-test-open "undeclared.age")
                           (list :state (agx-test-state)
                                 :warning (agx-test-warning)
                                 :age-runs (agx-test-run-count)))
             :declared-but-missing
             (progn
               (agx-test-project "new-secret.age")
               (agx-test-open "new-secret.age")
               (list :state (agx-test-state)
                     :messages (agx-test-messages "Not decrypting")
                     :age-runs (agx-test-run-count)
                     :directory (agx-test-entries))))))
    "##;

    let expect = expect![[
        r#"OK (:undeclared (:state (:mode agenix-mode :read-only t :modified nil :point 1 :buffer "-----BEGIN AGE ENCRYPTED FILE-----\nb3JwaGFuCg==\n-----END AGE ENCRYPTED FILE-----\n" :write-contents-functions #1=(agenix-save-decrypted) :auto-save nil) :warning (:warning ("Warning (emacs): Nix evaluation error." "Probably file [ORACLE-SANDBOX]/agenix/project/undeclared.age is not declared as a secret in ’secrets.nix’ file.") :nix-reported-missing-attribute t) :age-runs 0) :declared-but-missing (:state (:mode agenix-mode :read-only nil :modified nil :point 1 :buffer "" :write-contents-functions #1# :auto-save nil) :messages ("Not decrypting. File [ORACLE-SANDBOX]/agenix/project/new-secret.age does not exist and will be created when you will save this buffer.") :age-runs 0 :directory ("." ".." "secrets.nix" "undeclared.age")))"#
    ]];

    assert_agenix_parity(elisp_form, expect);
}
