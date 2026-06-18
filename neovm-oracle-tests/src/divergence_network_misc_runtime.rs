//! Network process loopback (server+client roundtrip, process-contact) and
//! misc parity (format-message quoting, ngettext, key-description, char-fold,
//! special floats, ash/bignum, text-property-search).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn network_process_contact() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((server (make-network-process :name "neo-srv2-xxx" :server t :host 'local
               :service t :family 'ipv4 :noquery t)))
  (let ((local (process-contact server :local)))
    (prog1 (list (processp server) (eq (process-status server) 'listen)
                 (vectorp local) (integerp (aref local (1- (length local)))))
      (delete-process server))))"##,
    );
}

#[test]
fn tcp_server_client() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((received nil) (server nil) (port nil))
  (setq server (make-network-process :name "neo-srv-xxx" :server t :host 'local
                 :service t :family 'ipv4 :noquery t
                 :filter (lambda (proc s) (setq received s)
                           (process-send-string proc "ack"))))
  (let ((local (process-contact server :local)))
    (setq port (aref local (1- (length local)))))
  (let ((client (make-network-process :name "neo-cli-xxx" :host 'local :service port
                  :family 'ipv4 :noquery t)) (cresp ""))
    (set-process-filter client (lambda (_p s) (setq cresp (concat cresp s))))
    (process-send-string client "hi-server")
    (let ((k 0)) (while (and (or (null received) (string= cresp "")) (< k 150))
                   (accept-process-output nil 0.02) (setq k (1+ k))))
    (delete-process client) (delete-process server)
    (list received cresp)))"##,
    );
}

#[test]
fn ash_bignum() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (ash 1 70) (ash (ash 1 70) -65) (logand (1- (ash 1 64)) (ash 1 63))
        (logcount (1- (ash 1 40))) (expt 3 50))"##,
    );
}

#[test]
fn char_fold_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'char-fold)
(with-temp-buffer
  (insert "the cafe is open")
  (goto-char (point-min))
  (let ((case-fold-search t))
    (list (re-search-forward (char-fold-to-regexp "cafe") nil t))))"##,
    );
}

#[test]
fn format_message_quotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((text-quoting-style 'curve))
  (list (format-message "use `foo' here") (substitute-command-keys "\\`C-c\\' test")))"##,
    );
}

#[test]
fn kbd_key_desc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (key-description (kbd "C-c C-x")) (key-description (kbd "M-RET"))
        (listify-key-sequence (kbd "C-a")) (key-description [?\C-a ?\M-b]))"##,
    );
}

#[test]
fn ngettext_fn() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (ngettext "%d file" "%d files" 1) (ngettext "%d file" "%d files" 2))"##,
    );
}

#[test]
fn number_special_floats() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (= 1.0e+INF 1.0e+INF) (isnan 0.0e+NaN)
        (> 1.0e+INF most-positive-fixnum) (format "%s" 1.0e+INF)
        (ftruncate 3.7) (fround 2.5) (ffloor -1.5) (fceiling 1.2))"##,
    );
}

#[test]
fn string_pixel_logical() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "ab\tcd")
  (goto-char (point-max))
  (list (current-column) (char-before) (line-beginning-position)))"##,
    );
}

#[test]
fn text_property_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'text-property-search)
(with-temp-buffer
  (insert "aaBBBcc")
  (put-text-property 3 6 'hi t)
  (goto-char (point-min))
  (let ((m (text-property-search-forward 'hi t t)))
    (list (prop-match-beginning m) (prop-match-end m))))"##,
    );
}
