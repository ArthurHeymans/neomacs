use expect_test::expect;

use super::{
    assert_ariadne_parity, assert_ariadne_signal_parity, assert_ariadne_with_legacy_cl_parity,
};

#[test]
fn length_encoder_and_decoder_round_trip_network_order_boundaries() {
    let elisp_form = r##"(mapcar
         (lambda (length)
           (let ((bytes (ariadne-encode-length length)))
             (with-temp-buffer
               (set-buffer-multibyte nil)
               (insert bytes)
               (goto-char (point-min))
               (list length
                     (length bytes)
                     (string-to-list bytes)
                     (ariadne-decode-length)))))
         '(0 1 255 256 65535 65536 16777215 4294967295))"##;
    let expect = expect![
        "OK ((0 4 (0 0 0 0) 0) (1 4 (0 0 0 1) 1) (255 4 (0 0 0 255) 255) (256 4 (0 0 1 0) 256) (65535 4 (0 0 255 255) 65535) (65536 4 (0 1 0 0) 65536) (16777215 4 (0 255 255 255) 16777215) (4294967295 4 (255 255 255 255) 4294967295))"
    ];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn decode_length_reads_four_bytes_at_point_not_at_buffer_start() {
    let elisp_form = r##"(with-temp-buffer
         (set-buffer-multibyte nil)
         (insert "prefix" (ariadne-encode-length 66051) "suffix")
         (goto-char 7)
         (list (point)
               (ariadne-decode-length)
               (buffer-string)
               (point)))"##;
    let expect = expect![[r#"OK (7 66051 "prefix\0\1\2\3suffix" 7)"#]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn decode_length_with_incomplete_header_signals() {
    let elisp_form = r##"(with-temp-buffer
         (set-buffer-multibyte nil)
         (insert "\0\0\1")
         (goto-char (point-min))
         (ariadne-decode-length))"##;
    let expect = expect!["ERR (args-out-of-range (:buffer nil) 1 5)"];
    assert_ariadne_signal_parity(elisp_form, expect);
}

#[test]
fn have_input_distinguishes_short_header_partial_exact_and_extra_frames() {
    let elisp_form = r##"(mapcar
         (lambda (payload)
           (with-temp-buffer
             (set-buffer-multibyte nil)
             (insert payload)
             (list (buffer-size)
                   (ariadne-have-input-p)
                   (point))))
         (list
          ""
          "\0\0\0"
          (ariadne-encode-length 3)
          (concat (ariadne-encode-length 3) "ab")
          (concat (ariadne-encode-length 3) "abc")
          (concat (ariadne-encode-length 3) "abcdef")))"##;
    let expect = expect!["OK ((0 nil 1) (3 nil 1) (4 nil 1) (6 nil 1) (7 t 1) (10 t 1))"];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn read_decodes_one_real_bert_message_and_consumes_only_its_frame() {
    let elisp_form = r##"(let* ((first (vector 'reply
                        (vector 'loc_known
                                "/workspace/Lib.hs" 12 7)))
              (second (vector 'error
                         (vector 'server 500 'failure
                                 "boom" [])))
              (first-bytes (bert-pack first))
              (second-bytes (bert-pack second)))
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (insert (ariadne-encode-length (length first-bytes))
                   first-bytes
                   (ariadne-encode-length (length second-bytes))
                   second-bytes)
           (let ((before (buffer-size))
                 (result (ariadne-read)))
             (list result
                   before
                   (buffer-size)
                   (ariadne-have-input-p)
                   (ariadne-read)
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ([reply [loc_known "/workspace/Lib.hs" 12 7]] 107 52 t [error [server 500 failure "boom" []]] "")"#
    ]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn read_exposes_exact_binary_string_and_nested_bert_conversion() {
    let elisp_form = r##"(let* ((object
                (vector 'reply
                        (list "λ"
                              (string 0 1 127 255)
                              -17
                              1099511627776
                              (vector t nil))))
              (bytes (bert-pack object)))
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (insert (ariadne-encode-length (length bytes))
                   bytes)
           (let* ((decoded (ariadne-read))
                  (payload (aref decoded 1))
                  (text (nth 0 payload))
                  (binary (nth 1 payload)))
             (list decoded
                   (equal decoded object)
                   (string-to-list text)
                   (multibyte-string-p text)
                   (string-to-list binary)
                   (multibyte-string-p binary)))))"##;
    let expect = expect![[
        r#"OK ([reply ("»" "\0\1\177ÿ" 4294967279 0 [t nil])] nil (187) t (0 1 127 255) t)"#
    ]];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}

#[test]
fn read_or_lose_closes_connection_and_wraps_malformed_input_error() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ariadne-read)
                    (lambda () (error "malformed packet")))
                   ((symbol-function 'ariadne-close)
                    (lambda (process)
                      (push (list :close process) calls))))
           (let ((outcome
                  (condition-case error
                      (list :ok
                            (ariadne-read-or-lose
                             'socket))
                    (error
                     (list :error error)))))
             (list outcome (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((:error (error "ariadne-read: (error \"malformed packet\")")) ((:close socket)))"#
    ]];
    assert_ariadne_parity(elisp_form, expect);
}

#[test]
fn framing_multiple_messages_reports_complete_only_after_each_header_and_body() {
    let elisp_form = r##"(let* ((object '(alpha "beta" 42))
              (body (bert-pack object))
              (header (ariadne-encode-length (length body))))
         (with-temp-buffer
           (set-buffer-multibyte nil)
           (let (states)
             (dolist (chunk
                      (list (substring header 0 2)
                            (substring header 2)
                            (substring body 0 1)
                            (substring body 1)))
               (goto-char (point-max))
               (insert chunk)
               (push (list (buffer-size)
                           (ariadne-have-input-p))
                     states))
             (list (nreverse states)
                   (ariadne-read)
                   (buffer-size)))))"##;
    let expect = expect![r#"OK (((2 nil) (4 nil) (5 nil) (30 t)) (alpha "beta" 42) 0)"#];
    assert_ariadne_with_legacy_cl_parity(elisp_form, expect);
}
