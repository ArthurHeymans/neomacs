use expect_test::expect;

use super::assert_anaphora_parity;

#[test]
fn anaphora_if_binds_truthy_and_nil_results_once_across_both_branches() {
    let elisp_form = r##"(list
         (aif (1+ 1)
             (1+ it))
         (aif (1+ 1)
             (progn
               (1+ it)
               (1+ it)))
         (aif (1+ 1)
             (progn
               (setq it
                     (1+ it))
               (1+ it)))
         (aif nil
             (+ 5 it)
           (null it))
         (let ((calls 0))
           (list
            (aif
                (progn
                  (setq calls
                        (1+ calls))
                  nil)
                :never
              (list it calls))
            calls))
         (aif
             '(:status ok
               :records (1 2 3))
             (list
              (plist-get it :status)
              (apply #'+
                     (plist-get
                      it :records)))
           :missing))"##;
    let expect = expect!["OK (3 3 4 t ((nil 1) 1) (ok 6))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_prog1_returns_the_mutable_first_value_after_real_side_effects() {
    let elisp_form = r##"(let (events)
         (list
          (aprog1
              (list :draft)
            (push :created events)
            (setcdr it
                    '(:validated
                      :persisted))
            (push
             (copy-tree it)
             events)
            :ignored)
          (nreverse events)
          (aprog1 5
            (setq it
                  (1+ it))
            10)
          (condition-case error
              (aprog1
                  (1+ it)
                (1+ it))
            (error
             (list
              (car error)
              (cadr error))))))"##;
    let expect = expect![
        "OK ((:draft :validated :persisted) (:created (:draft :validated :persisted)) 6 (void-variable it))"
    ];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_prog2_runs_the_leading_form_then_returns_the_mutable_second_value() {
    let elisp_form = r##"(let (events)
         (list
          (aprog2
              (push :prepare events)
              (list :payload)
            (push :body events)
            (setcdr it
                    '(:normalized))
            :ignored)
          (nreverse events)
          (aprog2 1 5
            (setq it
                  (1+ it))
            10)
          (condition-case error
              (aprog2
                  (1+ it)
                  1
                1)
            (error
             (list
              (car error)
              (cadr error))))
          (condition-case error
              (aprog2
                  1
                  (1+ it)
                1)
            (error
             (list
              (car error)
              (cadr error))))))"##;
    let expect = expect![
        "OK ((:payload :normalized) (:prepare :body) 6 (void-variable it) (void-variable it))"
    ];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_when_runs_multi_form_bodies_only_for_truthy_values() {
    let elisp_form = r##"(let ((calls 0)
                              events)
         (list
          (awhen
              (progn
                (setq calls
                      (1+ calls))
                '(:user "Ada"
                  :active t))
            (push
             (plist-get it :user)
             events)
            (list
             :accepted
             (plist-get it :active)))
          (awhen
              (progn
                (setq calls
                      (1+ calls))
                nil)
            (push :never events))
          calls
          (nreverse events)
          (awhen (1+ 1)
            (setq it
                  (1+ it))
            (1+ it))))"##;
    let expect = expect![[r#"OK ((:accepted t) nil 2 ("Ada") 4)"#]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_while_rebinds_it_on_each_iteration_and_exposes_mutable_sequences() {
    let elisp_form = r##"(list
         (let ((queue
                '((alpha . 1)
                  (beta . 2)
                  (gamma . 3)))
               processed)
           (awhile queue
             (push
              (list
               (caar it)
               (apply #'+
                      (mapcar #'cdr
                              it)))
              processed)
             (setq queue
                   (cdr queue)))
           (nreverse processed))
         (let ((items '(1 2 3 4))
               snapshots)
           (awhile items
             (push 5 it)
             (push
              (copy-sequence it)
              snapshots)
             (setq items
                   (cdr items)))
           (nreverse snapshots)))"##;
    let expect =
        expect!["OK (((alpha 6) (beta 5) (gamma 3)) ((5 1 2 3 4) (5 2 3 4) (5 3 4) (5 4)))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_and_short_circuits_and_rebinds_each_successive_result() {
    let elisp_form = r##"(let (events)
         (list
          (aand
           (progn
             (push :lookup events)
             '(:id 7
               :enabled t))
           (progn
             (push
              (plist-get it :id)
              events)
             (plist-get
              it :enabled))
           (progn
             (push :enabled events)
             '("a" "b" "c"))
           (length it))
          (nreverse events)
          (aand
           (push :first events)
           nil
           (push :never events))
          (nreverse events)
          (aand)
          (aand 42)
          (condition-case error
              (aand
               (1+ it)
               (1+ it))
            (error
             (list
              (car error)
              (cadr error))))))"##;
    let expect =
        expect!["OK (3 (:lookup 7 . #1=(:enabled :first)) nil #1# t 42 (void-variable it))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_cond_returns_bare_tests_and_binds_selected_clause_values() {
    let elisp_form = r##"(let ((attempts 0)
                              result)
         (setq result
               (acond
                ((progn
                   (setq attempts
                         (1+ attempts))
                   nil)
                 :never)
                ((progn
                   (setq attempts
                         (1+ attempts))
                   '(:code 202
                     :body "queued"))
                 (list
                  :accepted
                  (plist-get it :code)
                  (upcase
                   (plist-get
                    it :body))))
                (t :fallback)))
         (list
          result
          attempts
          (acond (1))
          (acond (1 nil))
          (acond
           (:foo)
           ("bar")
           (:baz))
          (acond
           (nil 4)
           (2 (1+ it)))
          (let (value)
            (acond
             ((+ 2 2)
              (setq value 38)
              (setq value
                    (+ value it))
              value)
             (t nil)))))"##;
    let expect = expect![[r#"OK ((:accepted 202 "QUEUED") 2 1 nil :foo 3 42)"#]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_lambda_supports_recursive_factorial_tree_walks_and_closure_state() {
    let elisp_form = r##"(let* ((factorial
                        (alambda (number)
                          (if
                              (= number 0)
                              1
                            (* number
                               (self
                                (1- number))))))
                       (target 'a)
                       (walker
                        (alambda (tree)
                          (if
                              (consp tree)
                              (+
                               (self
                                (car tree))
                               (self
                                (cdr tree)))
                            (if
                                (eq tree target)
                                1
                              0)))))
         (list
          (mapcar factorial
                  '(0 1 5 7))
          (mapcar walker
                  '((a b c)
                    (d a r
                       (p a))
                    (d a r)
                    (a a)))
          (let* ((calls 0)
                (sum
                 (alambda (numbers)
                   (setq calls
                         (1+ calls))
                   (if numbers
                       (+
                        (car numbers)
                        (self
                         (cdr numbers)))
                     0))))
            (list
             (funcall sum
                      '(2 4 6 8))
             calls))))"##;
    let expect = expect!["OK ((1 1 120 5040) (1 2 1 2) (20 5))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_block_threads_results_and_supports_early_named_returns() {
    let elisp_form = r##"(list
         (ablock pipeline
           1
           (1+ it)
           (1+ it))
         (ablock pipeline
           1
           (1+ it)
           (1+ it)
           0)
         (ablock pipeline
           1
           (1+ it)
           (cl-return-from
               pipeline))
         (ablock pipeline
           1
           (1+ it)
           (1+ it)
           (cl-return-from
               pipeline
             (list :stopped
                   (1+ it))))
         (ablock empty)
         (ablock single
           '(:only value)))"##;
    let expect = expect!["OK (3 0 nil (:stopped 4) nil (:only value))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_let_binds_once_for_multi_step_practical_transformations() {
    let elisp_form = r##"(let ((calls 0))
         (list
          (alet
              (progn
                (setq calls
                      (1+ calls))
                '((name . "Ada")
                  (roles admin editor)))
            (let ((name
                   (cdr
                    (assq 'name it)))
                  (roles
                   (cdr
                    (assq 'roles it))))
              (list
               (upcase name)
               (length roles)
               (memq 'admin
                     roles))))
          calls
          (alet (+ 1 1)
            it)
          (alet nil
            (list
             it
             (null it)))))"##;
    let expect = expect![[r#"OK (("ADA" 2 (admin editor)) 1 2 (nil t))"#]];
    assert_anaphora_parity(elisp_form, expect);
}
