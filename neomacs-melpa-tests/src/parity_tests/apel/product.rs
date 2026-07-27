use expect_test::expect;

use super::{assert_apel_signal_parity, assert_apel_source_parity};

#[test]
fn product_definition_accessors_and_mutators_round_trip_all_fields() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0)))
                      (product-define "client" nil '(1 2 0) "Juniper")
                      (let ((product (product-find-by-name "client")))
                        (let ((initial
                               (list (product-name product)
                                     (product-family product)
                                     (product-version product)
                                     (product-code-name product)
                                     (product-checkers product)
                                     (product-family-products product)
                                     (product-features product)
                                     (product-version-string product))))
                          (product-set-name product "client-renamed")
                          (product-set-family product "suite")
                          (product-set-version product '(2 1))
                          (product-set-code-name product "Maple")
                          (product-set-checkers product '(ignore))
                          (product-set-family-products product '("child"))
                          (product-set-features product '(client-core))
                          (product-set-version-string product "2.1-custom")
                          (list initial product))))"##;
    let expect = expect![[
        r#"OK (("client" nil (1 2 0) "Juniper" nil nil nil nil) ["client-renamed" "suite" (2 1) "Maple" (ignore) ("child") (client-core) "2.1-custom"])"#
    ]];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn product_family_workflow_deduplicates_additions_and_removes_children() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0)))
                      (product-define "suite" nil '(4 0))
                      (product-define "client" "suite" '(1 0))
                      (product-define "worker" "suite" '(2 0))
                      (product-add-to-family "suite" "client")
                      (let ((before
                             (copy-sequence
                              (product-family-products
                               (product-find-by-name "suite")))))
                        (product-remove-from-family "suite" "client")
                        (list before
                              (product-family-products
                               (product-find-by-name "suite"))
                              (product-family
                               (product-find-by-name "client")))))"##;
    let expect = expect![[r#"OK (("worker" "client") ("worker") "suite")"#]];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn product_feature_registration_supports_lookup_removal_and_polymorphic_find() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0)))
                      (product-define "client" nil '(1 0))
                      (let ((product (product-find-by-name "client")))
                        (product-add-feature product 'apel-demo-feature)
                        (put 'apel-demo-feature 'product product)
                        (provide 'apel-demo-feature)
                        (let ((registered
                               (list (product-features product)
                                     (eq product
                                         (product-find-by-feature
                                          'apel-demo-feature))
                                     (eq product
                                         (product-find 'apel-demo-feature))
                                     (eq product (product-find "client"))
                                     (eq product (product-find product)))))
                          (product-remove-feature product 'apel-demo-feature)
                          (list registered (product-features product)))))"##;
    let expect = expect!["OK (((apel-demo-feature) t t t t) nil)"];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn product_checker_pipeline_honors_ignore_force_order_and_explicit_versions() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0))
                           (events nil))
                      (product-define "client" nil '(3 2))
                      (let ((product (product-find-by-name "client")))
                        (product-add-checkers
                         product
                         (lambda (actual target)
                           (push (list :first actual target) events))
                         'ignore
                         (lambda (actual target)
                           (push (list :last actual target) events)))
                        (product-run-checkers product '(4 0))
                        (let ((without-force (copy-tree events)))
                          (product-run-checkers product '(4 0) t)
                          (list (product-checkers product)
                                without-force
                                events
                                (let ((product-ignore-checkers t))
                                  (product-add-checkers
                                   product (lambda (_a _b) :never))
                                  (length (product-checkers product)))))))"##;
    let expect = expect![
        "OK ((#[(actual target) ((setq events (cons (list :last actual target) events))) (#2=(events (:first #1=(4 0) #1#) (:last #1# #1#)))] ignore #[(actual target) ((setq events (cons (list :first actual target) events))) (#2#)]) nil ((:first #3=(4 0) #3#) (:last #3# #3#)) 3)"
    ];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn product_string_generation_walks_real_family_features_and_verbose_names() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0)))
                      (product-define "suite" nil '(5 0) "Oak")
                      (product-define "client" "suite" '(2 7) "Maple")
                      (product-define "worker" "suite" nil nil)
                      (dolist (entry '(("suite" suite-feature)
                                       ("client" client-feature)
                                       ("worker" worker-feature)))
                        (let ((product (product-find-by-name (car entry))))
                          (product-add-feature product (cadr entry))))
                      (list (product-version-as-string
                             (product-find-by-name "client"))
                            (product-string-1
                             (product-find-by-name "client"))
                            (product-string-1
                             (product-find-by-name "client") t)
                            (product-string "suite")
                            (product-string-verbose "suite")))"##;
    let expect = expect![[
        r#"OK ("2.7" "client/2.7" "client/2.7 (Maple)" "suite/5.0 worker client/2.7" "suite/5.0 (Oak) worker client/2.7 (Maple)")"#
    ]];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn product_iteration_filtering_versions_listing_and_parser_cover_edge_cases() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0))
                           (visited nil))
                      (product-define "suite" nil '(2 0))
                      (product-define "active" "suite" '(2 1))
                      (product-define "inactive" "suite" '(1 9))
                      (product-add-feature
                       (product-find-by-name "active") 'active-feature)
                      (product-for-each
                       "suite" nil
                       (lambda (product prefix)
                         (push (concat prefix (product-name product)) visited))
                       "seen:")
                      (list (sort visited #'string<)
                            (product-version-compare '(1 2 0) '(1 1 9))
                            (product-version-compare '(1 2) '(1 2 0))
                            (product-version-compare '(1 2 0) '(1 2))
                            (product-version>= "active" '(2 0))
                            (sort (mapcar #'product-name
                                          (product-list-products))
                                  #'string<)
                            (mapcar #'product-parse-version-string
                                    '("client v1.2.3-beta (Maple)"
                                      "release 10.4"
                                      "no-version"))))"##;
    let expect = expect![[
        r#"OK (("seen:active") 1 -1 1 t ("active" "inactive" "suite") (((1 2 3) "Maple" "1.2.3-beta") ((10 4) nil "10.4") (nil nil nil)))"#
    ]];
    assert_apel_source_parity("product.el", elisp_form, expect);
}

#[test]
fn invalid_product_and_missing_family_surface_exact_error_contracts() {
    let elisp_form = r##"(let ((product-obarray (make-vector 13 0)))
                      (product-define "client" nil '(1 0))
                      (product-add-to-family "missing" "client"))"##;
    let expect = expect![[r#"ERR (error "Family product ‘missing’ is not defined")"#]];
    assert_apel_signal_parity("product.el", elisp_form, expect);
}
