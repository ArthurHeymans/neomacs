use expect_test::expect;

use super::assert_auth_source_keytar_parity;

#[test]
fn auth_source_keytar_enable_adds_keytar_to_front_and_forgets_cached_credentials() {
    let elisp_form = r##"(let ((auth-sources
                                '("~/.authinfo"
                                  "~/.netrc"))
                               calls)
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  (push
                   (copy-tree auth-sources)
                   calls)
                  :cache-cleared)))
            (list
             (auth-source-keytar-enable)
             auth-sources
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (:cache-cleared (keytar "~/.authinfo" "~/.netrc") ((keytar "~/.authinfo" "~/.netrc")))"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_is_idempotent_for_source_membership_but_clears_cache_each_time() {
    let elisp_form = r##"(let ((auth-sources
                                '("first-source"))
                               calls)
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  (push
                   (copy-tree auth-sources)
                   calls)
                  (length calls))))
            (list
             (auth-source-keytar-enable)
             (auth-source-keytar-enable)
             (auth-source-keytar-enable)
             auth-sources
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (1 2 3 (keytar "first-source") ((keytar "first-source") (keytar "first-source") (keytar "first-source")))"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_preserves_existing_keytar_position_in_auth_sources() {
    let elisp_form = r##"(let ((auth-sources
                                '("first"
                                  keytar
                                  "last"))
                               calls)
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  (setq calls
                        (1+ (or calls 0)))
                  :cleared)))
            (list
             (auth-source-keytar-enable)
             auth-sources
             calls)))"##;
    let expect = expect![[r#"OK (:cleared ("first" keytar "last") 1)"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_distinguishes_symbolic_source_from_similarly_named_entries() {
    let elisp_form = r##"(let ((auth-sources
                                '("keytar"
                                  (keytar)
                                  keytar-config
                                  (:source keytar))))
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  :cleared)))
            (list
             (auth-source-keytar-enable)
             auth-sources)))"##;
    let expect =
        expect![[r#"OK (:cleared (keytar "keytar" (keytar) keytar-config (:source keytar)))"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_propagates_cache_clear_failure_after_registering_source() {
    let elisp_form = r##"(let ((auth-sources
                                '("existing")))
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  (error
                   "fixture cache failure"))))
            (list
             (auth-source-keytar-test-error-data
              #'auth-source-keytar-enable)
             auth-sources)))"##;
    let expect = expect![[r#"OK ((:error error ("fixture cache failure")) (keytar "existing"))"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_returns_exact_cache_clear_result() {
    let elisp_form = r##"(mapcar
          (lambda (result)
            (let ((auth-sources nil))
              (cl-letf
                  (((symbol-function
                     'auth-source-forget-all-cached)
                    (lambda ()
                      result)))
                (list
                 result
                 (auth-source-keytar-enable)
                 auth-sources))))
          '(nil
            t
            :cleared
            17
            "done"
            (:cache "result")))"##;
    let expect = expect![[
        r#"OK ((nil nil (keytar)) (t t (keytar)) (:cleared :cleared (keytar)) (17 17 (keytar)) ("done" "done" (keytar)) (#1=(:cache "result") #1# (keytar)))"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_respects_dynamic_auth_sources_binding_without_mutating_global_default()
{
    let elisp_form = r##"(let ((global-before
                                (copy-tree
                                 (default-value
                                  'auth-sources)))
                               dynamic-result)
          (setq dynamic-result
                (let ((auth-sources
                       '("sandbox-authinfo")))
                  (cl-letf
                      (((symbol-function
                         'auth-source-forget-all-cached)
                        (lambda ()
                          :cleared)))
                    (list
                     (auth-source-keytar-enable)
                     auth-sources
                     (default-value
                      'auth-sources)))))
          (list
           dynamic-result
           (default-value
            'auth-sources)
           (equal
            global-before
            (default-value
             'auth-sources))))"##;
    let expect = expect![[
        r#"OK ((:cleared #1=(keytar "sandbox-authinfo") #1#) ("~/.authinfo" "~/.authinfo.gpg" "~/.netrc") t)"#
    ]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}

#[test]
fn auth_source_keytar_enable_uses_structural_membership_for_preexisting_keytar_symbol() {
    let elisp_form = r##"(let ((auth-sources
                                (list
                                 (copy-sequence "first")
                                 (intern
                                  (concat
                                   "key"
                                   "tar"))
                                 (copy-sequence "last"))))
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  :cleared)))
            (list
             (auth-source-keytar-enable)
             auth-sources
             (length
              (seq-filter
               (lambda (entry)
                 (eq entry 'keytar))
               auth-sources)))))"##;
    let expect = expect![[r#"OK (:cleared ("first" keytar "last") 1)"#]];

    assert_auth_source_keytar_parity(elisp_form, expect);
}
