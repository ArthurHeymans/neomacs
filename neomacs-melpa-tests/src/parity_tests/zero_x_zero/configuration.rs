use expect_test::expect;

use super::assert_zero_x_zero_parity;

#[test]
fn zero_x_zero_public_defaults_match_the_pinned_release() {
    let elisp_form = r##"(list
               0x0-default-server
               0x0-use-curl
               (mapcar
                (lambda (entry)
                  (let ((server (cdr entry)))
                    (list
                     (car entry)
                     (plist-get server :scheme)
                     (plist-get server :host)
                     (plist-get server :default-dir)
                     (plist-get server :curl-args-fun)
                     (plist-get server :min-age)
                     (plist-get server :max-age)
                     (plist-get server :max-size))))
                0x0-servers))"##;
    let expect = expect![[
        r#"OK (0x0 if-installed ((0x0 "https" "0x0.st" "~/Desktop" 0x0--make-0x0-curl-args 30 365 536870912) (ttm "https" "ttm.sh" "~/Desktop" 0x0--make-0x0-curl-args 30 365 268435456) (envs "https" "envs.sh" "~/Desktop" 0x0--make-0x0-curl-args 30 365 536870912)))"#
    ]];

    assert_zero_x_zero_parity(elisp_form, expect);
}

#[test]
fn zero_x_zero_choose_server_uses_default_without_a_prefix() {
    let elisp_form = r##"(let ((0x0-servers
                      '((alpha :host "a")
                        (beta :host "b")))
                     (0x0-default-server 'beta)
                     (current-prefix-arg nil))
               (cl-letf (((symbol-function 'completing-read)
                          (lambda (&rest _)
                            (error
                             "must not prompt without prefix"))))
                 (0x0--choose-server)))"##;
    let expect = expect![[r#"OK (:host "b")"#]];

    assert_zero_x_zero_parity(elisp_form, expect);
}

#[test]
fn zero_x_zero_choose_server_prompts_strictly_with_a_prefix() {
    let elisp_form = r##"(let ((0x0-servers
                      '((alpha :host "a")
                        (beta :host "b")))
                     (0x0-default-server 'alpha)
                     (current-prefix-arg '(4))
                     calls)
               (cl-letf (((symbol-function 'completing-read)
                          (lambda (&rest args)
                            (push args calls)
                            "beta")))
                 (list
                  (0x0--choose-server)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK ((:host "b") (("Server: " (alpha beta) nil t nil nil alpha)))"#]];

    assert_zero_x_zero_parity(elisp_form, expect);
}

#[test]
fn zero_x_zero_timeout_formula_covers_full_empty_half_and_invalid_servers() {
    let elisp_form = r##"(let ((server
                      (list
                       :min-age 30
                       :max-age 365
                       :max-size 536870912)))
               (list
                (0x0--calculate-timeout
                 server 536870912)
                (round
                 (0x0--calculate-timeout server 1))
                (0x0--calculate-timeout
                 server 268435456)
                (0x0--calculate-timeout
                 '(:min-age "bad"
                   :max-age 365
                   :max-size 10)
                 1)
                (0x0--calculate-timeout nil 1)))"##;
    let expect = expect!["OK (30.0 365 71.875 nil nil)"];

    assert_zero_x_zero_parity(elisp_form, expect);
}

#[test]
fn zero_x_zero_server_accessors_and_custom_curl_builder_preserve_values() {
    let elisp_form = r##"(let* ((builder
                       (lambda (server file bounded)
                         (list
                          (plist-get server :host)
                          file
                          bounded)))
                      (server
                       (list
                        :host "example.test"
                        :default-dir "/pick/here"
                        :curl-args-fun builder)))
               (list
                (0x0--get-server-default-dir server)
                (eq
                 (0x0--get-server-curl-args-fun server)
                 builder)
                (0x0--make-curl-args
                 server "file.txt" 'bounded)))"##;
    let expect = expect![[r#"OK ("/pick/here" t ("example.test" "file.txt" bounded))"#]];

    assert_zero_x_zero_parity(elisp_form, expect);
}

#[test]
fn zero_x_zero_pick_file_uses_dired_at_point_or_the_server_directory() {
    let elisp_form = r##"(let ((server
                      '(:default-dir "/server/default"))
                     calls)
               (cl-letf (((symbol-function 'derived-mode-p)
                          (lambda (&rest modes)
                            (memq 'dired-mode modes)))
                         ((symbol-function
                           'dired-file-name-at-point)
                          (lambda ()
                            (push 'dired calls)
                            "/dired/file"))
                         ((symbol-function 'read-file-name)
                          (lambda (&rest args)
                            (push
                             (cons 'read-file-name args)
                             calls)
                            "/prompt/file")))
                 (let ((dired-result
                        (0x0--pick-file server)))
                   (cl-letf (((symbol-function
                               'derived-mode-p)
                              (lambda (&rest _) nil)))
                     (list
                      dired-result
                      (0x0--pick-file server)
                      (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK ("/dired/file" "/prompt/file" (dired (read-file-name "Pick a file to share: " "/server/default")))"#
    ]];

    assert_zero_x_zero_parity(elisp_form, expect);
}

#[test]
fn zero_x_zero_bounds_default_to_buffer_limits_and_preserve_explicit_points() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abcdef")
               (list
                (0x0--make-bounds)
                (0x0--make-bounds 2 5)
                (0x0--bounds-size
                 (0x0--make-bounds))
                (0x0--bounds-size
                 '(:start 8 :end 3))))"##;
    let expect = expect!["OK ((:start 1 :end 7) (:start 2 :end 5) 6 -5)"];

    assert_zero_x_zero_parity(elisp_form, expect);
}
