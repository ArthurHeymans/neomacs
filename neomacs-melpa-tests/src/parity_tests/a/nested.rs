use expect_test::expect;

use super::assert_a_parity;

#[test]
fn a_assoc_in_updates_existing_mixed_paths_without_mutating_the_source() {
    let elisp_form = r##"(let* ((source
                      (a-alist
                       :foo
                       (a-alist
                        :bar [1 2 3])))
                     (result
                      (a-assoc-in
                       source
                       [:foo :bar 2]
                       5)))
               (list
                (a-get-in result
                          [:foo :bar 2])
                (a-get-in source
                          [:foo :bar 2])
                (eq result source)
                result
                source))"##;
    let expect = expect![[r#"OK (5 3 nil ((:foo (:bar . [1 2 5]))) ((:foo (:bar . [1 2 3]))))"#]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_assoc_in_builds_missing_alist_layers_and_empty_keys_return_the_original() {
    let elisp_form = r##"(let* ((source
                      (a-alist
                       :existing 1))
                     (result
                      (a-assoc-in
                       source
                       [:new :branch 2]
                       5)))
               (list
                result
                (a-get-in result
                          [:new :branch 2])
                source
                (eq source
                    (a-assoc-in
                     source [] 9))))"##;
    let expect = expect![[r#"OK (((:new (:branch (2 . 5))) . #1=((:existing . 1))) 5 #1# t)"#]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_assoc_in_extends_nested_vectors_and_preserves_each_original_level() {
    let elisp_form = r##"(let* ((inner [zero])
                     (source
                      (vector inner))
                     (result
                      (a-assoc-in
                       source [0 3]
                       'three)))
               (list
                result
                source
                inner
                (eq source result)
                (eq inner
                    (aref result 0))))"##;
    let expect = expect!["OK ([[zero nil nil three]] [#1=[zero]] #1# nil nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_update_passes_existing_or_nil_values_and_extra_arguments_once() {
    let elisp_form = r##"(let ((source
                    '((:name . "A")))
                   events)
               (let ((updated
                      (a-update
                       source :name
                       (lambda (old suffix)
                         (push
                          (list old suffix)
                          events)
                         (concat old suffix))
                       "-x"))
                     (inserted
                      (a-update
                       source :missing
                       (lambda (old prefix)
                         (push
                          (list old prefix)
                          events)
                         (concat
                          prefix
                          (if old "old"
                            "nil")))
                       "was-")))
                 (list
                  updated
                  inserted
                  source
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (((:name . "A-x")) ((:missing . "was-nil") . #1=((:name . "A"))) #1# (("A" "-x") (nil "was-")))"#
    ]];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_update_clones_hashes_and_vectors_and_can_extend_a_vector_index() {
    let elisp_form = r##"(let* ((table
                      (a-hash-table
                       :count 2))
                     (new-table
                      (a-update
                       table :count #'+ 3))
                     (vector [1 2])
                     (new-vector
                      (a-update
                       vector 4
                       (lambda (old)
                         (if old
                             'unexpected
                           'new)))))
               (list
                (a-get new-table
                       :count)
                (a-get table :count)
                (eq table new-table)
                new-vector
                vector
                (eq vector
                    new-vector)))"##;
    let expect = expect!["OK (5 2 nil [1 2 nil nil new] [1 2] nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_update_in_updates_existing_and_missing_paths_with_extra_arguments() {
    let elisp_form = r##"(let* ((source
                      (a-alist
                       :stats
                       (a-alist
                        :score 9)))
                     (incremented
                      (a-update-in
                       source
                       [:stats :score]
                       #'+ 1))
                     (inserted
                      (a-update-in
                       source
                       [:stats :label]
                       (lambda (old prefix)
                         (concat
                          prefix
                          (if old "old"
                            "new")))
                       "state-")))
               (list
                (a-get-in incremented
                          [:stats :score])
                (a-get-in inserted
                          [:stats :label])
                (a-get-in source
                          [:stats :score])
                (a-has-key
                 (a-get source :stats)
                 :label)))"##;
    let expect = expect!["OK (10 \"state-new\" 9 nil)"];

    assert_a_parity(elisp_form, expect);
}

#[test]
fn a_update_in_with_empty_keys_returns_identity_without_calling_the_function() {
    let elisp_form = r##"(let ((source
                    (a-list :value 1))
                   (calls 0))
               (let ((result
                      (a-update-in
                       source []
                       (lambda (&rest _)
                         (setq calls
                               (1+ calls))
                         'changed)
                       'extra)))
                 (list
                  (eq source result)
                  result calls)))"##;
    let expect = expect!["OK (t ((:value . 1)) 0)"];

    assert_a_parity(elisp_form, expect);
}
