//! Complex combo batch 425 — 19 probes into esoteric/unusual areas:
//! format-spec deeper, char-fold-to-regexp deeper, regexp-opt with
//! paren/shy, key-valid-p edge cases, isearch-filter-predicate,
//! dired-mark/unmark, time-stamp, format-spec with modifiers,
//! ewoc/elib widget, tq/task-queue, atimer-run-at-time,
//! itimer/idle-timer, substitute-env-vars, file-name-case-insensitive-p,
//! file-ownership-preserved-p, system-users/system-groups,
//! memory-limit, gc-status-features, and emacs-pid.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// format-spec with modifiers and character specs.
#[test]
fn div_cx425_format_spec_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(progn (require 'format-spec)
  (let ((spec (format-spec-make ?a "hello" ?b "world")))
    (list (format-spec "%a %b" spec)
          (format-spec "%a" spec))))
"##,
        expect_test::expect![[r#""OK (\"hello world\" \"hello\")""#]],
    );
}

/// char-fold-to-regexp with multibyte and ASCII equivalents.
#[test]
fn div_cx425_char_fold_to_regexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (char-fold-to-regexp "cafe")
      (char-fold-to-regexp "a")
      (char-fold-to-regexp "12"))
"##,
        expect_test::expect![[
            r#""OK (\"\\\\(?:c[\u{301}\u{302}\u{307}\u{30c}\u{327}]\\\\|[cçćĉċčᶜḉⅽⓒｃ𝐜𝑐𝒄𝒸𝓬𝔠𝕔𝖈𝖼𝗰𝘤𝙘𝚌]\\\\)\\\\(?:a[\u{300}-\u{304}\u{306}-\u{30a}\u{30c}\u{30f}\u{311}\u{323}\u{325}\u{328}]\\\\|[aªà-åāăąǎǟǡǻȁȃȧᵃḁạảấầẩẫậắằẳẵặₐⓐａ𝐚𝑎𝒂𝒶𝓪𝔞𝕒𝖆𝖺𝗮𝘢𝙖𝚊]\\\\)\\\\(?:f\u{307}\\\\|[fᶠḟⓕｆ𝐟𝑓𝒇𝒻𝓯𝔣𝕗𝖋𝖿𝗳𝘧𝙛𝚏]\\\\)\\\\(?:e[\u{300}-\u{304}\u{306}-\u{309}\u{30c}\u{30f}\u{311}\u{323}\u{327}\u{328}\u{32d}\u{330}]\\\\|[eè-ëēĕėęěȅȇȩᵉḕḗḙḛḝẹẻẽếềểễệₑℯⅇⓔｅ𝐞𝑒𝒆𝓮𝔢𝕖𝖊𝖾𝗲𝘦𝙚𝚎]\\\\)\" \"\\\\(?:a[\u{300}-\u{304}\u{306}-\u{30a}\u{30c}\u{30f}\u{311}\u{323}\u{325}\u{328}]\\\\|[aªà-åāăąǎǟǡǻȁȃȧᵃḁạảấầẩẫậắằẳẵặₐⓐａ𝐚𝑎𝒂𝒶𝓪𝔞𝕒𝖆𝖺𝗮𝘢𝙖𝚊]\\\\)\" \"\\\\(?:[1¹₁①１𜳱𝟏𝟙𝟣𝟭𝟷🯱][2²₂②２𜳲𝟐𝟚𝟤𝟮𝟸🯲]\\\\|⑫\\\\)\")""#
        ]],
    );
}

/// regexp-opt with paren and shy-group options.
#[test]
fn div_cx425_regexp_opt_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (regexp-opt '("hello" "hello-world") 'paren)
      (regexp-opt '("abc" "def") 'shy))
"##,
        expect_test::expect![[
            r#""OK (\"\\\\(hello\\\\(?:-world\\\\)?\\\\)\" \"\\\\(abc\\\\|def\\\\)\")""#
        ]],
    );
}

/// key-valid-p with various edge case inputs.
#[test]
fn div_cx425_key_valid_p_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (key-valid-p "a")
      (key-valid-p "C-c C-x M-a")
      (key-valid-p "")
      (key-valid-p "mouse-1")
      (key-valid-p "C-1"))
"##,
        expect_test::expect![[r#""OK (t t nil nil t)""#]],
    );
}

/// substitute-env-vars with multibyte.
#[test]
fn div_cx425_substitute_env_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((process-environment (cons "MY_VAR=café世界" process-environment)))
  (list (substitute-env-vars "$MY_VAR")
        (substitute-env-vars "prefix-$MY_VAR-suffix")
        (substitute-env-vars "no-env")))
"##,
        expect_test::expect![[r#""OK (\"café世界\" \"prefix-café世界-suffix\" \"no-env\")""#]],
    );
}

/// file-name-case-insensitive-p / file-ownership-preserved-p.
#[test]
fn div_cx425_file_case_ownership() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (file-name-case-insensitive-p "/")
      (file-ownership-preserved-p "/tmp"))
"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

/// system-users / system-groups: user/group database queries.
#[test]
fn div_cx425_system_users_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (condition-case e (system-users) (error (car e)))
      (condition-case e (system-groups) (error (car e))))
"##,
        expect_test::expect![[
            r#""OK ((\"nobody\" \"nixbld32\" \"nixbld31\" \"nixbld30\" \"nixbld29\" \"nixbld28\" \"nixbld27\" \"nixbld26\" \"nixbld25\" \"nixbld24\" \"nixbld23\" \"nixbld22\" \"nixbld21\" \"nixbld20\" \"nixbld19\" \"nixbld18\" \"nixbld17\" \"nixbld16\" \"nixbld15\" \"nixbld14\" \"nixbld13\" \"nixbld12\" \"nixbld11\" \"nixbld10\" \"nixbld9\" \"nixbld8\" \"nixbld7\" \"nixbld6\" \"nixbld5\" \"nixbld4\" \"nixbld3\" \"nixbld2\" \"nixbld1\" \"sftpuser\" \"exec\" \"nscd\" \"rtkit\" \"systemd-oom\" \"sshd\" \"avahi\" \"dhcpcd\" \"flatpak\" \"jackaudio\" \"ollama\" \"guixbuilder0\" \"guixbuilder1\" \"guixbuilder2\" \"guixbuilder3\" \"guixbuilder4\" \"guixbuilder5\" \"guixbuilder6\" \"guixbuilder7\" \"guixbuilder8\" \"guixbuilder9\" \"fwupd-refresh\" \"geoclue\" \"distcc\" \"minio\" \"sddm\" \"systemd-timesync\" \"systemd-resolve\" \"systemd-network\" \"systemd-coredump\" \"polkituser\" \"messagebus\" \"root\") (\"nogroup\" \"nixbld\" \"nscd\" \"polkituser\" \"rtkit\" \"systemd-coredump\" \"systemd-oom\" \"sshd\" \"avahi\" \"dhcpcd\" \"ydotool\" \"flatpak\" \"jackaudio\" \"resolvconf\" \"ollama\" \"guixbuild\" \"sftp\" \"fwupd-refresh\" \"geoclue\" \"clock\" \"pipewire\" \"distcc\" \"shadow\" \"sgx\" \"render\" \"kvm\" \"minio\" \"sddm\" \"input\" \"systemd-timesync\" \"systemd-resolve\" \"systemd-network\" \"docker\" \"users\" \"keys\" \"vboxusers\" \"systemd-journal\" \"adm\" \"utmp\" \"dialout\" \"video\" \"tape\" \"cdrom\" \"lp\" \"uucp\" \"floppy\" \"audio\" \"disk\" \"messagebus\" \"tty\" \"kmem\" \"wheel\" \"root\"))""#
        ]],
    );
}

/// memory-limit / gc-status-features / emacs-pid.
#[test]
fn div_cx425_memory_gc_status() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (memory-limit)
      (gc-status-features)
      (emacs-pid))
"##,
        expect_test::expect![[r#""ERR (void-function gc-status-features)""#]],
    );
}

/// file-newest-backup / find-backup-file-name deeper.
#[test]
fn div_cx425_backup_file_deeper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((f "/tmp/neo-cx425-fixed.el"))
  (list (file-newest-backup f)
        (backup-file-name-p (concat f "~"))
        (make-backup-file-name f)))
"##,
        expect_test::expect![[r#""OK (nil 23 \"/tmp/neo-cx425-fixed.el~\")""#]],
    );
}

/// atimer: run-at-time with 0 delay in batch.
#[test]
fn div_cx425_atimer_run_at_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((fired nil))
  (run-at-time 0 nil (lambda () (setq fired t)))
  (sit-for 0.1)
  fired)
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

/// dired-mark/unmark operations.
#[test]
fn div_cx425_dired_mark_unmark() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(require 'dired)
(let ((tmpdir (make-temp-file "neo-cx425-dm-" t)))
  (with-temp-file (expand-file-name "a.txt" tmpdir) (insert "x"))
  (with-temp-file (expand-file-name "b.txt" tmpdir) (insert "y"))
  (unwind-protect
      (with-temp-buffer
        (dired tmpdir)
        (dired-mark 1)
        (dired-unmark 1)
        (length (dired-get-marked-files)))
    (delete-directory tmpdir t)))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

/// time-stamp: automatic time stamp formatting.
#[test]
fn div_cx425_time_stamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(progn (require 'time-stamp)
  (list (stringp (time-stamp-string))
        (boundp 'time-stamp-format)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

/// log-file-suffixes / byte-compile-dest-file.
#[test]
fn div_cx425_log_byte_dest() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(progn (require 'bytecomp)
  (condition-case e
      (byte-compile-dest-file "/tmp/test.el")
    (error (car e))))
"##,
        expect_test::expect![[r#""OK \"/tmp/test.elc\"""#]],
    );
}

/// file-name-split / file-name-canonicalize.
#[test]
fn div_cx425_file_name_split_canon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (file-name-split "/a/b/c.txt")
      (file-name-split "a/b")
      (condition-case e (file-name-canonicalize "/tmp/../tmp/.") (error (car e))))
"##,
        expect_test::expect![[
            r#""OK ((\"\" \"a\" \"b\" \"c.txt\") (\"a\" \"b\") void-function)""#
        ]],
    );
}

/// read-char-from-minibuffer / read-char-by-name deeper.
#[test]
fn div_cx425_read_char_from_minibuffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity(
        r##"
(condition-case e
    (read-char-by-name "test: " t)
  (error (car e)))
"##,
    );
}

/// global-substring / substring-no-properties with multibyte.
#[test]
fn div_cx425_substring_no_props_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (substring-no-properties "café世界" 1 4)
      (substring "café世界" 1 4))
"##,
        expect_test::expect![[r#""OK (\"afé\" \"afé\")""#]],
    );
}

/// integer-or-marker-p / number-or-marker-p / natnump.
#[test]
fn div_cx425_number_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (integer-or-marker-p 5)
      (integer-or-marker-p (make-marker))
      (number-or-marker-p 5.5)
      (natnump -1)
      (natnump 0)
      (natnump 1))
"##,
        expect_test::expect![[r#""OK (t t t nil t t)""#]],
    );
}
