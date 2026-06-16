//! Complex combo batch 212 — `xdg` / `password-cache` / `auth-source` /
//! `auth-source-pass` / `secrets` / `netrc` availability and metadata.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx212_xdg_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'xdg)
      (list (fboundp 'xdg-data-home)
            (fboundp 'xdg-config-home)
            (fboundp 'xdg-cache-home)
            (fboundp 'xdg-data-dirs)
            (fboundp 'xdg-config-dirs)
            (fboundp 'xdg-desktop-read-file)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_password_cache_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'password-cache)
      (list (fboundp 'password-read)
            (fboundp 'password-cache-add)
            (fboundp 'password-cache-remove)
            (fboundp 'password-cache-search)
            (boundp 'password-cache-expiry)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_auth_source_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'auth-source)
      (list (fboundp 'auth-source-search)
            (fboundp 'auth-source-forget)
            (fboundp 'auth-source-forget-all)
            (boundp 'auth-sources)
            (boundp 'auth-source-do-cache)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_auth_source_pass_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (featurep 'auth-source-pass)
          (fboundp 'auth-source-pass-enable)
          (boundp 'auth-source-pass-filename))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_secrets_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'secrets)
      (list (fboundp 'secrets-get-collections)
            (fboundp 'secrets-create-collection)
            (boundp 'secrets-enabled)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_netrc_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'netrc)
      (list (fboundp 'netrc-machine)
            (fboundp 'netrc-port)
            (boundp 'netrc-file)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_epa_epg_key_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'epg)
      (list (fboundp 'epg-make-context)
            (fboundp 'epg-encrypt-string)
            (fboundp 'epg-decrypt-string)
            (fboundp 'epg-sign-string)
            (fboundp 'epg-verify-string)
            (fboundp 'epg-list-keys)
            (boundp 'epg-gpg-program)
            (boundp 'epg-gpgsm-program)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_plstore_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (featurep 'plstore)
          (fboundp 'plstore-open)
          (boundp 'plstore-cache))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx212_secure_hash_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data "Hello café 世界"))
  (list (secure-hash 'md5 data)
        (secure-hash 'sha1 data)
        (secure-hash 'sha224 data)
        (secure-hash 'sha256 data)
        (secure-hash 'sha384 data)
        (secure-hash 'sha512 data)
        (length (secure-hash 'sha256 data))))
"##,
    );
}

#[test]
fn div_cx212_hash_with_file_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((path (make-temp-file "neo-cx212-hash"))
       (data "test data for hashing\nsecond line\n"))
  (with-temp-buffer
    (insert data)
    (write-region (point-min) (point-max) path nil 'silent))
  (let ((hash-file (secure-hash 'sha256 (with-temp-buffer
                                           (insert-file-contents path)
                                           (buffer-string))))
        (hash-data (secure-hash 'sha256 data)))
    (delete-file path)
    (list (string= hash-file hash-data)
          (length hash-file))))
"##,
    );
}

#[test]
fn div_cx212_auth_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'auth-source)
      (require 'password-cache)
      (let ((hash (secure-hash 'sha256 "auth-source mega test")))
        (with-temp-buffer
          (buffer-enable-undo)
          (insert (format "Auth mega: %s" hash))
          (put-text-property 1 6 'face 'bold)
          (let ((m (set-marker (make-marker) 10))
                (ov (make-overlay 4 18)))
            (overlay-put ov 'face 'italic)
            (overlay-put ov 'evaporate t)
            (narrow-to-region 2 25)
            (let ((state (list (fboundp 'auth-source-search)
                               (boundp 'password-cache-expiry)
                               hash
                               (buffer-string)
                               (marker-position m)
                               (overlay-start ov) (overlay-end ov)
                               (text-properties-at 1))))
              (undo)
              (widen)
              (list state (buffer-string) (marker-position m)
                    (overlay-start ov) (overlay-end ov)
                    (text-properties-at 1)))))))
  (error (list :errored (car e))))
"##,
    );
}
