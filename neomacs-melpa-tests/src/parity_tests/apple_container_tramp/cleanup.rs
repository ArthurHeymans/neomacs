use expect_test::expect;

use super::{assert_apple_container_tramp_parity, assert_apple_container_tramp_signal_parity};

#[test]
fn cleanup_preserves_current_tramp_record_keys_and_dumps_the_nonempty_cache() {
    let elisp_form = r##"(let*
    ((tramp-cache-data (make-hash-table :test #'equal))
     (tramp-cache-data-changed nil)
     (running
      (tramp-dissect-file-name
       "/container:root@payments:/srv/app"))
     (stopped
      (tramp-dissect-file-name
       "/container:root@retired-worker:/srv/app"))
     events)
  (puthash running 'running-properties tramp-cache-data)
  (puthash stopped 'stopped-properties tramp-cache-data)
  (cl-letf
      (((symbol-function 'process-lines)
        (lambda (program &rest arguments)
          (push (list 'process program arguments) events)
          '("ID IMAGE STATUS"
            "payments api:v4 running")))
       ((symbol-function 'tramp-dump-connection-properties)
        (lambda ()
          (push
           (list 'dumped tramp-cache-data-changed)
           events)
          'dumped))
       ((symbol-function 'delete-file)
        (lambda (file)
          (push (list 'unexpected-delete file) events))))
    (list
     (apple-container-tramp-cleanup)
     (type-of running)
     (vectorp running)
     (gethash running tramp-cache-data 'absent)
     (gethash stopped tramp-cache-data 'absent)
     (hash-table-count tramp-cache-data)
     tramp-cache-data-changed
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (dumped cons nil running-properties stopped-properties 2 t ((process "container" ("ls")) (dumped t)))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn cleanup_rejects_legacy_vector_keys_before_it_can_filter_documented_id_name_rows() {
    let elisp_form = r##"(let
    ((tramp-cache-data (make-hash-table :test #'equal))
     (key ["container" "payments"]))
  (puthash key 'properties tramp-cache-data)
  (cl-letf
      (((symbol-function
         'apple-container-tramp--running-containers)
        (lambda ()
          '(("payments" "Payments API")
            ("orders-db" "Orders Database")))))
    (apple-container-tramp-cleanup)))"##;
    let expect = expect![[r#"ERR (wrong-type-argument tramp-file-name ["container" "payments"])"#]];
    assert_apple_container_tramp_signal_parity(elisp_form, expect);
}

#[test]
fn cleanup_deletes_the_persistency_file_when_the_cache_is_already_empty() {
    let elisp_form = r##"(let
    ((tramp-cache-data (make-hash-table :test #'equal))
     (tramp-cache-data-changed nil)
     (tramp-persistency-file-name
      "./tmp/apple-container-tramp/persistency.el")
     events)
  (cl-letf
      (((symbol-function
         'apple-container-tramp--running-containers)
        (lambda () nil))
       ((symbol-function 'delete-file)
        (lambda (file)
          (push
           (list 'deleted file tramp-cache-data-changed)
           events)
          'deleted))
       ((symbol-function 'tramp-dump-connection-properties)
        (lambda ()
          (push 'unexpected-dump events))))
    (list
     (apple-container-tramp-cleanup)
     (hash-table-count tramp-cache-data)
     tramp-cache-data-changed
     (nreverse events))))"##;
    let expect =
        expect![[r#"OK (deleted 0 t ((deleted "./tmp/apple-container-tramp/persistency.el" t)))"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn cleanup_ignores_persistency_deletion_failures_after_emptying_the_cache() {
    let elisp_form = r##"(let
    ((tramp-cache-data (make-hash-table :test #'equal))
     (tramp-cache-data-changed nil)
     (tramp-persistency-file-name
      "./tmp/apple-container-tramp/read-only.el")
     events)
  (cl-letf
      (((symbol-function
         'apple-container-tramp--running-containers)
        (lambda () nil))
       ((symbol-function 'delete-file)
        (lambda (file)
          (push (list 'delete-attempt file) events)
          (signal 'file-error
                  (list "Permission denied" file))))
       ((symbol-function 'tramp-dump-connection-properties)
        (lambda ()
          (push 'unexpected-dump events))))
    (list
     (apple-container-tramp-cleanup)
     tramp-cache-data-changed
     (nreverse events))))"##;
    let expect =
        expect![[r#"OK (nil t ((delete-attempt "./tmp/apple-container-tramp/read-only.el")))"#]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn cleanup_can_be_invoked_interactively_and_persists_the_empty_cache_state() {
    let elisp_form = r##"(let
    ((tramp-cache-data (make-hash-table :test #'equal))
     (tramp-cache-data-changed nil)
     (tramp-persistency-file-name
      "./tmp/apple-container-tramp/interactive.el")
     events)
  (cl-letf
      (((symbol-function
         'apple-container-tramp--running-containers)
        (lambda () nil))
       ((symbol-function 'delete-file)
        (lambda (file)
          (push (list 'deleted file) events)
          'deleted))
       ((symbol-function 'tramp-dump-connection-properties)
        (lambda ()
          (push 'unexpected-dump events))))
    (list
     (call-interactively
      #'apple-container-tramp-cleanup)
     (commandp 'apple-container-tramp-cleanup)
     (interactive-form
      'apple-container-tramp-cleanup)
     (hash-table-count tramp-cache-data)
     tramp-cache-data-changed
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (deleted t (interactive nil) 0 t ((deleted "./tmp/apple-container-tramp/interactive.el")))"#
    ]];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn cleanup_dumps_without_evicting_modern_container_or_unrelated_records() {
    let elisp_form = r##"(let*
    ((tramp-cache-data (make-hash-table :test #'equal))
     (tramp-cache-data-changed nil)
     (stopped
      (tramp-dissect-file-name
       "/container:root@retired:/srv"))
     (ssh
      (tramp-dissect-file-name
       "/ssh:builder@mac.example:/workspace"))
     events)
  (puthash stopped 'stopped tramp-cache-data)
  (puthash ssh 'ssh-session tramp-cache-data)
  (cl-letf
      (((symbol-function
         'apple-container-tramp--running-containers)
        (lambda () nil))
       ((symbol-function 'tramp-dump-connection-properties)
        (lambda ()
          (push
           (list
            'dumped
            tramp-cache-data-changed
            (hash-table-count tramp-cache-data))
           events)
          'dumped))
       ((symbol-function 'delete-file)
        (lambda (file)
          (push (list 'unexpected-delete file) events))))
    (list
     (apple-container-tramp-cleanup)
     (gethash stopped tramp-cache-data 'absent)
     (gethash ssh tramp-cache-data 'absent)
     (hash-table-count tramp-cache-data)
     (nreverse events))))"##;
    let expect = expect!["OK (dumped stopped ssh-session 2 ((dumped t 2)))"];
    assert_apple_container_tramp_parity(elisp_form, expect);
}

#[test]
fn cleanup_with_real_discovery_rows_still_rejects_legacy_vector_cache_keys() {
    let elisp_form = r##"(let*
    ((tramp-cache-data (make-hash-table :test #'equal))
     (key ["container" "payments"]))
  (puthash key 'properties tramp-cache-data)
  (cl-letf
      (((symbol-function 'process-lines)
        (lambda (&rest _arguments)
          '("ID IMAGE STATUS"
            "payments api:v4 running"
            "orders-db postgres:v17 running"))))
    (apple-container-tramp-cleanup)))"##;
    let expect = expect![[r#"ERR (wrong-type-argument tramp-file-name ["container" "payments"])"#]];
    assert_apple_container_tramp_signal_parity(elisp_form, expect);
}
