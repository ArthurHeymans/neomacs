use expect_test::expect;

use super::assert_apple_container_tramp_parity;

#[test]
fn add_method_builds_the_exact_default_container_exec_transport_contract() {
    let elisp_form = r##"(let ((tramp-methods nil)
      (apple-container-tramp-container-options nil))
  (list
   (apple-container-tramp-add-method)
   tramp-methods))"##;
    let expect = expect![[
        r#"OK (#1=(("container" (tramp-login-program "container") (tramp-login-args (nil ("exec" "-it") ("-u" "%u") ("%h") ("sh"))) (tramp-remote-shell "/bin/sh") (tramp-remote-shell-args ("-i" "-c")))) #1#)"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn add_method_embeds_custom_global_options_before_exec_without_flattening_them() {
    let elisp_form = r##"(let
    ((tramp-methods nil)
     (apple-container-tramp-container-options
      '("--context" "production"
        "--url" "unix:///Users/alice/.container/run.sock")))
  (apple-container-tramp-add-method)
  (car tramp-methods))"##;
    let expect = expect![[
        r#"OK ("container" (tramp-login-program "container") (tramp-login-args (("--context" "production" "--url" "unix:///Users/alice/.container/run.sock") ("exec" "-it") ("-u" "%u") ("%h") ("sh"))) (tramp-remote-shell "/bin/sh") (tramp-remote-shell-args ("-i" "-c")))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn add_method_is_idempotent_when_the_method_definition_is_unchanged() {
    let elisp_form = r##"(let
    ((tramp-methods
      '(("ssh"
         (tramp-login-program "ssh")
         (tramp-login-args (("%h"))))))
     (apple-container-tramp-container-options
      '("--context" "local")))
  (list
   (apple-container-tramp-add-method)
   (apple-container-tramp-add-method)
   tramp-methods
   (length
    (seq-filter
     (lambda (entry)
       (equal (car entry)
              apple-container-tramp-method))
     tramp-methods))))"##;
    let expect = expect![[
        r#"OK (#1=(("container" (tramp-login-program "container") (tramp-login-args (("--context" "local") ("exec" "-it") ("-u" "%u") ("%h") ("sh"))) (tramp-remote-shell "/bin/sh") (tramp-remote-shell-args ("-i" "-c"))) ("ssh" (tramp-login-program "ssh") (tramp-login-args (("%h"))))) #1# #1# 1)"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn changing_options_retains_both_distinct_method_definitions_in_precedence_order() {
    let elisp_form = r##"(let ((tramp-methods nil)
      (apple-container-tramp-container-options nil))
  (apple-container-tramp-add-method)
  (setq apple-container-tramp-container-options
        '("--context" "remote"))
  (apple-container-tramp-add-method)
  (mapcar
   (lambda (entry)
     (list
      (car entry)
      (cadr (assq 'tramp-login-args (cdr entry)))))
   tramp-methods))"##;
    let expect = expect![[
        r#"OK (("container" (("--context" "remote") . #1=(("exec" "-it") ("-u" "%u") ("%h") ("sh")))) ("container" (nil . #1#)))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn setup_calls_method_registration_before_installing_the_completion_parser() {
    let elisp_form = r##"(let (events)
  (cl-letf
      (((symbol-function
         'apple-container-tramp-add-method)
        (lambda ()
          (push '(add-method) events)
          'method-added))
       ((symbol-function 'tramp-set-completion-function)
        (lambda (method functions)
          (push
           (list 'set-completion method functions)
           events)
          'completion-installed)))
    (list
     (apple-container-tramp-setup)
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (completion-installed ((add-method) (set-completion "container" ((apple-container-tramp--parse-running-containers "")))))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn setup_registers_a_usable_method_and_host_completion_entry_together() {
    let elisp_form = r##"(let ((tramp-methods nil)
      (tramp-completion-function-alist nil)
      (apple-container-tramp-container-options
       '("--context" "development")))
  (apple-container-tramp-setup)
  (list
   (assoc apple-container-tramp-method tramp-methods)
   (assoc apple-container-tramp-method
          tramp-completion-function-alist)
   (tramp-get-completion-function
    apple-container-tramp-method)))"##;
    let expect = expect![[
        r#"OK (("container" (tramp-login-program "container") (tramp-login-args (("--context" "development") ("exec" "-it") ("-u" "%u") ("%h") ("sh"))) (tramp-remote-shell "/bin/sh") (tramp-remote-shell-args ("-i" "-c"))) ("container" . #1=((apple-container-tramp--parse-running-containers ""))) ((tramp-parse-default-user-host "container") (tramp-parse-auth-sources "container") (tramp-parse-connection-properties "container") . #1#))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn simple_container_filenames_preserve_optional_user_host_and_remote_localname() {
    let elisp_form = r##"(mapcar
 (lambda (name)
   (let ((vector (tramp-dissect-file-name name)))
     (list
      name
      (tramp-file-name-method vector)
      (tramp-file-name-user vector)
      (tramp-file-name-host vector)
      (tramp-file-name-localname vector)
      (tramp-file-name-hop vector))))
 '("/container:payments:/srv/app/config.toml"
   "/container:root@orders-db:/var/lib/postgresql/data/"
   "/container:1000@3f12a9c7:/workspace/a file.txt"))"##;
    let expect = expect![[
        r#"OK (("/container:payments:/srv/app/config.toml" "container" nil "payments" "/srv/app/config.toml" nil) ("/container:root@orders-db:/var/lib/postgresql/data/" "container" "root" "orders-db" "/var/lib/postgresql/data/" nil) ("/container:1000@3f12a9c7:/workspace/a file.txt" "container" "1000" "3f12a9c7" "/workspace/a file.txt" nil))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn ssh_and_sudo_multihop_filenames_resolve_to_the_container_as_the_final_hop() {
    let elisp_form = r##"(mapcar
 (lambda (name)
   (let ((vector (tramp-dissect-file-name name)))
     (list
      (tramp-file-name-method vector)
      (tramp-file-name-user vector)
      (tramp-file-name-host vector)
      (tramp-file-name-localname vector)
      (tramp-file-name-hop vector))))
 '("/ssh:builder@mac.example|container:app@payments:/srv/app/"
   "/sudo:root@localhost|container:root@database:/var/lib/data"))"##;
    let expect = expect![[
        r#"OK (("container" "app" "payments" "/srv/app/" "ssh:builder@mac.example|") ("container" "root" "database" "/var/lib/data" "sudo:root@localhost|"))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}
