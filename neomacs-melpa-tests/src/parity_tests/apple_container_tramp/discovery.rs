use expect_test::expect;

use super::assert_apple_container_tramp_parity;

#[test]
fn running_containers_parses_realistic_container_ls_rows_and_drops_the_header() {
    let elisp_form = r##"(let ((apple-container-tramp-container-options nil)
      calls)
  (cl-letf
      (((symbol-function 'process-lines)
        (lambda (program &rest arguments)
          (push (cons program arguments) calls)
          '("ID         IMAGE                  OS     STATUS"
            "3f12a9c7   ghcr.io/acme/api:42    linux  running"
            "orders-db  postgres:17            linux  running"
            "worker_1   ghcr.io/acme/job:edge  linux  running"))))
    (list
     (apple-container-tramp--running-containers)
     (nreverse calls))))"##;
    let expect = expect![[r#"OK (("3f12a9c7" "orders-db" "worker_1") (("container" "ls")))"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn running_containers_places_every_custom_container_option_before_the_ls_subcommand() {
    let elisp_form = r##"(let
    ((apple-container-tramp-container-options
      '("--context" "production"
        "--url" "unix:///Users/alice/.container/run.sock"
        "--debug"))
     calls)
  (cl-letf
      (((symbol-function 'process-lines)
        (lambda (program &rest arguments)
          (push (cons program arguments) calls)
          '("ID IMAGE STATUS"
            "payments registry.local/payments:v7 running"))))
    (list
     (apple-container-tramp--running-containers)
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("payments") (("container" "--context" "production" "--url" "unix:///Users/alice/.container/run.sock" "--debug" "ls")))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn running_containers_preserves_daemon_order_duplicates_and_case_sensitive_ids() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'process-lines)
      (lambda (&rest _arguments)
        '("ID IMAGE STATUS"
          "Api-Blue image:v1 running"
          "api-blue image:v2 running"
          "Api-Blue image:v1 running"
          "0000abcd image:v3 running"))))
  (apple-container-tramp--running-containers))"##;
    let expect = expect![[r#"OK ("Api-Blue" "api-blue" "Api-Blue" "0000abcd")"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn running_containers_exposes_blank_and_short_daemon_rows_without_filtering_them() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'process-lines)
      (lambda (&rest _arguments)
        '("ID IMAGE STATUS"
          "   "
          "single-field"
          "  padded-id\timage:v1\trunning  "
          "\t"))))
  (apple-container-tramp--running-containers))"##;
    let expect = expect![[r#"OK (nil "single-field" "padded-id" nil)"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn running_containers_returns_nil_for_header_only_and_completely_empty_output() {
    let elisp_form = r##"(let ((outputs
       '(("ID IMAGE STATUS")
         nil)))
  (cl-letf
      (((symbol-function 'process-lines)
        (lambda (&rest _arguments)
          (pop outputs))))
    (list
     (apple-container-tramp--running-containers)
     (apple-container-tramp--running-containers))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn running_containers_suppresses_container_cli_failures_and_returns_nil() {
    let elisp_form = r##"(let (calls)
  (cl-letf
      (((symbol-function 'process-lines)
        (lambda (program &rest arguments)
          (push (cons program arguments) calls)
          (error "container daemon is unavailable"))))
    (list
     (apple-container-tramp--running-containers)
     (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil (("container" "ls")))"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn completion_parser_returns_empty_user_host_pairs_and_ignores_the_filename_argument() {
    let elisp_form = r##"(let (calls)
  (cl-letf
      (((symbol-function
         'apple-container-tramp--running-containers)
        (lambda ()
          (push 'discovered calls)
          '("api-blue" "orders-db" "worker_1"))))
    (list
     (apple-container-tramp--parse-running-containers
      "/ssh:build@mac.example|container:/srv/")
     (apple-container-tramp--parse-running-containers nil)
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((("" "api-blue") ("" "orders-db") ("" "worker_1")) (("" "api-blue") ("" "orders-db") ("" "worker_1")) (discovered discovered))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}
