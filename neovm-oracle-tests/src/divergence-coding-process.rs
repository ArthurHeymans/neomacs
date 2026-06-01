//! Divergence tests: coding systems, encoding/decoding, process stubs.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_oracle_parity, assert_oracle_parity_with_env};

#[test]
fn divergence_coding_system_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (coding-system-p 'utf-8)
  (coding-system-p 'iso-8859-1)
  (coding-system-p 'binary)
  (coding-system-p 'us-ascii)
  (coding-system-p 'no-such-coding-system))"#,
    );
}

#[test]
fn divergence_encode_decode_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((str "Héllo 世界")
         (encoded (encode-coding-string str 'utf-8))
         (decoded (decode-coding-string encoded 'utf-8)))
  (list (string-equal str decoded)
        (string-bytes encoded)
        (multibyte-string-p str)
        (multibyte-string-p decoded)))"#,
    );
}

#[test]
fn divergence_encode_decode_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((bytes (unibyte-string 72 101 108 108 111))
         (decoded (decode-coding-string bytes 'binary)))
  (list (string-equal decoded "Hello")
        (multibyte-string-p bytes)
        (multibyte-string-p decoded)))"#,
    );
}

#[test]
fn divergence_coding_system_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (coding-system-base 'utf-8)
  (coding-system-base 'utf-8-dos)
  (coding-system-base 'iso-latin-1)
  (coding-system-eol-type 'utf-8)
  (coding-system-eol-type 'utf-8-dos))"#,
    );
}

#[test]
fn divergence_coding_system_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((cs (find-coding-systems-string "Hello World")))
  (list (consp cs)
        (member 'utf-8 cs)
        (member 'raw-text cs)))"#,
    );
}

#[test]
fn divergence_preferred_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (coding-system-p preferred-coding-system)
  (coding-system-p default-terminal-coding-system)
  (coding-system-p default-buffer-file-coding-system))"#,
    );
}

#[test]
fn divergence_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (listp (process-list))
  (process-list))"#,
    );
}

#[test]
fn divergence_get_buffer_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (processp (get-buffer-process "*scratch*"))
  (null (get-buffer-process "*scratch*"))
  (null (get-process "nonexistent-process-xyz")))"#,
    );
}

#[test]
fn divergence_make_network_process_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (featurep 'make-network-process)
  (fboundp 'make-network-process)
  (fboundp 'make-serial-process))"#,
    );
}

#[test]
fn divergence_make_network_process_invalid_keyword_domains() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((text-quoting-style 'grave))
  (list
   (condition-case err
       (make-network-process :name "np-nowait" :server t :nowait t :service 0)
     (error err))
   (condition-case err
       (make-network-process :name "np-type" :server t :service 0 :type 'bogus)
     (error err))
   (condition-case err
       (make-network-process :name "np-family" :server t :service 0 :family 'bogus)
     (error err))))"#,
    );
}

#[test]
fn divergence_make_network_process_stream_server_accepts_client() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((events nil))
  (condition-case err
      (let* ((srv (make-network-process
                   :name "srv" :server t :service t :host 'local
                   :log (lambda (server client msg)
                          (push (list (process-name client) msg) events))))
             (port (process-contact srv :service))
             (cli (make-network-process :name "cli" :host 'local :service port)))
        (accept-process-output nil 0.2)
        (prog1
            (list (process-status srv)
                  (integerp port)
                  (> port 0)
                  (process-status cli)
                  (length events))
          (delete-process cli)
          (delete-process srv)))
    (error err)))"#,
    );
}

#[test]
fn divergence_make_network_process_explicit_inet_address() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((srv nil)
      (cli nil))
  (condition-case err
      (unwind-protect
          (progn
            (setq srv (make-network-process
                       :name "srv" :server t
                       :local [127 0 0 1 0]
                       :host 42
                       :service 1
                       :family 'bogus))
            (setq cli (make-network-process
                       :name "cli"
                       :remote (process-contact srv :local)
                       :host 42))
            (accept-process-output nil 0.2)
            (list (process-status srv)
                  (vectorp (process-contact srv :local))
                  (integerp (process-contact srv :service))
                  (= (aref (process-contact srv :local) 4)
                     (process-contact srv :service))
                  (process-contact srv :host)
                  (process-contact srv :family)
                  (process-status cli)
                  (vectorp (process-contact cli :remote))
                  (vectorp (process-contact cli :local))
                  (process-contact cli :host)
                  (process-contact cli :service)))
        (when cli (delete-process cli))
        (when srv (delete-process srv)))
    (error err)))"#,
    );
}

#[cfg(unix)]
#[test]
fn divergence_make_network_process_local_stream_server_accepts_client() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((path (make-temp-file "neomacs-local-sock-"))
       (events nil))
  (delete-file path)
  (unwind-protect
      (condition-case err
          (let* ((srv (make-network-process
                       :name "srv" :server t :family 'local :service path
                       :log (lambda (server client msg)
                              (push (list (process-name client)
                                          msg
                                          (process-contact client :remote)
                                          (process-contact client :local))
                                    events))))
                 (cli (make-network-process
                       :name "cli" :family 'local :service path)))
            (accept-process-output nil 0.2)
            (prog1
                (list (process-status srv)
                      (equal (process-contact srv :local) path)
                      (equal (process-contact srv :service) path)
                      (process-status cli)
                      (equal (process-contact cli :remote) path)
                      (equal (process-contact cli :local) "")
                      (length events)
                      (and events
                           (not (null (string-match-p "^srv <[0-9]+>$"
                                                      (caar events)))))
                      (and events (equal (cadar events) "accept from -\n"))
                      (and events (equal (nth 2 (car events)) ""))
                      (and events (equal (nth 3 (car events)) path)))
              (delete-process cli)
              (delete-process srv)))
        (error err))
    (ignore-errors (delete-file path))))"#,
    );
}

#[cfg(unix)]
#[test]
fn divergence_make_network_process_explicit_local_address() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((path (make-temp-file "neomacs-local-address-"))
      (srv nil)
      (cli nil))
  (delete-file path)
  (unwind-protect
      (condition-case err
          (progn
            (setq srv (make-network-process
                       :name "srv" :server t
                       :local path
                       :host "ignored"
                       :service 1))
            (setq cli (make-network-process
                       :name "cli"
                       :remote (process-contact srv :local)
                       :host "bad.invalid"
                       :service 1))
            (accept-process-output nil 0.2)
            (list (process-status srv)
                  (equal (process-contact srv :local) path)
                  (process-contact srv :host)
                  (process-contact srv :service)
                  (process-status cli)
                  (equal (process-contact cli :remote) path)
                  (equal (process-contact cli :local) "")
                  (process-contact cli :host)
                  (process-contact cli :service)))
        (error err))
    (when cli (delete-process cli))
    (when srv (delete-process srv))
    (ignore-errors (delete-file path))))"#,
    );
}

#[test]
fn divergence_num_processors_openmp_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (integerp (num-processors))
  (> (num-processors) 0)
  (integerp (num-processors 'current))
  (> (num-processors 'current) 0)
  (integerp (num-processors 'all))
  (> (num-processors 'all) 0)
  (equal (num-processors 'bogus) (num-processors t)))"#,
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", "3"), ("OMP_THREAD_LIMIT", "0")],
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", "3"), ("OMP_THREAD_LIMIT", "2")],
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", " 4,8"), ("OMP_THREAD_LIMIT", "0")],
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", "0"), ("OMP_THREAD_LIMIT", "1")],
    );
}

#[test]
fn divergence_call_process_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'call-process)
  (fboundp 'call-process-region)
  (fboundp 'start-process)
  (fboundp 'shell-command))"#,
    );
}
