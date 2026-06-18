//! Complex combo batch 418 — 20 probes into process coding, JSON deep,
//! system info, version, calendar, timezone, format-seconds, auth-source,
//! password caching, mail stubs, and prettify-symbols.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// process-coding-system: querying process encoding.
#[test]
fn div_cx418_process_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx418-pc"
                          :command '("echo" "test")
                          :connection-type 'pipe :buffer nil
                          :coding 'utf-8-unix)))
  (accept-process-output proc 2)
  (let ((coding (process-coding-system proc)))
    (list (coding-system-p (car coding))
          (coding-system-p (cdr coding))))
  (delete-process proc))
"##,
    );
}

/// set-process-coding-system: changing process encoding.
#[test]
fn div_cx418_set_process_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx418-spc"
                          :command '("echo" "test")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (set-process-coding-system proc 'latin-1 'utf-8)
  (let ((coding (process-coding-system proc)))
    (list (car coding) (cdr coding)))
  (delete-process proc))
"##,
    );
}

/// open-network-stream (may be stubbed).
#[test]
fn div_cx418_open_network_stream() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (open-network-stream "neo-cx418-net" nil "localhost" 0)
  (error (car e)))
"##,
    );
}

/// json-read with json-object-type = hash-table.
#[test]
fn div_cx418_json_read_hash_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((json-object-type 'hash-table)
      (json-array-type 'list))
  (let ((data (json-read-from-string "{\"a\":1,\"b\":2,\"c\":3}")))
    (list (hash-table-p data)
          (gethash "a" data)
          (gethash "c" data))))
"##,
    );
}

/// json-pretty-print: formatting JSON with indentation.
#[test]
fn div_cx418_json_pretty_print() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data (json-read-from-string "{\"a\":[1,2,3],\"b\":{\"c\":4}}")))
  (json-pretty-print data))
"##,
    );
}

/// emacs-version / system-configuration queries.
#[test]
fn div_cx418_emacs_version_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (stringp (emacs-version))
      (> (length (emacs-version)) 10)
      (stringp (system-configuration))
      (stringp (system-configuration-options)))
"##,
    );
}

/// emacs-build-time / emacs-repository-version.
#[test]
fn div_cx418_emacs_build_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (stringp (format-time-string "%Y" emacs-build-time))
      (or (null emacs-repository-version)
          (stringp emacs-repository-version)))
"##,
    );
}

/// system-type and system-name differences.
#[test]
fn div_cx418_system_type_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (symbol-name system-type)
      (stringp (system-name)))
"##,
    );
}

/// format-seconds: formatting time intervals.
#[test]
fn div_cx418_format_seconds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format-seconds "%Y %D %H %M %S" 3661.0)
      (format-seconds "%h:%m:%s" 3661)
      (format-seconds "%z" 3600))
"##,
    );
}

/// decoded-time-add / decoded-time-period.
#[test]
fn div_cx418_decoded_time_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dt (decode-time (encode-time 0 0 0 1 1 2024 nil))))
  (list (condition-case e (decoded-time-add dt (decoded-time-period "P1D"))
          (error (car e)))
        (condition-case e (decoded-time-add dt (decoded-time-period "PT1H"))
          (error (car e)))))
"##,
    );
}

/// timezone functions: current-time-zone-offset.
#[test]
fn div_cx418_timezone_offset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (current-time-zone-offset)
      (/ (current-time-zone-offset) 3600))
"##,
    );
}

/// calendar day-of-week / day-number.
#[test]
fn div_cx418_calendar_day_week() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'calendar)
(let ((date '(1 1 2024)))
  (list (calendar-day-of-week date)
        (calendar-day-number date)
        (calendar-iso-date date)))
"##,
    );
}

/// auth-source: authentication backend (may be stubbed).
#[test]
fn div_cx418_auth_source() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'auth-source)
(list (auth-source-pick-first-password :host "example.com" :port "smtp")
      (auth-source-forget+ :host "example.com"))
"##,
    );
}

/// password-cache: cache operations only (no interactive prompt).
#[test]
fn div_cx418_password_cache() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(require 'password-cache)
(password-cache-add "test" "secret")
(list (password-cache-remove "test")
      (password-cache-remove "nonexistent"))
"##,
    );
}

/// compose-mail / mail-bury (mail stubs).
#[test]
fn div_cx418_compose_mail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (compose-mail "test@example.com" "subject")
  (error (car e)))
"##,
    );
}

/// prettify-symbols-mode: symbol prettification.
#[test]
fn div_cx418_prettify_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'prog-mode)
  (with-temp-buffer
    (emacs-lisp-mode)
    (setq-local prettify-symbols-alist '(("lambda" . ?λ)))
    (prettify-symbols-mode 1)
    (insert "(lambda (x) x)")
    (list (prettify-symbols-mode)
          (buffer-string))))
"##,
    );
}

/// locale-info: system locale information.
#[test]
fn div_cx418_locale_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (locale-info 'codeset) (error (car e)))
      (condition-case e (locale-info 'days) (error (car e)))
      (condition-case e (locale-info 'months) (error (car e))))
"##,
    );
}

/// parse-time: parsing date/time strings.
#[test]
fn div_cx418_parse_time_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'parse-time)
  (list (parse-time-string "2024-01-15 14:30:00")
        (parse-time-string "2024-06-16")))
"##,
    );
}

/// seconds-to-string / time-to-duration.
#[test]
fn div_cx418_seconds_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (seconds-to-string 0)
      (seconds-to-string 60)
      (seconds-to-string 3661))
"##,
    );
}
