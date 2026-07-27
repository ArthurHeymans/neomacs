use expect_test::expect;

use super::assert_anaphora_parity;

#[test]
fn anaphora_case_evaluates_expression_once_and_exposes_it_in_selected_clause() {
    let elisp_form = r##"(let ((calls 0))
         (list
          (acase
              (progn
                (setq calls
                      (1+ calls))
                1)
            (0 :zero)
            (1
             (list
              :matched
              it
              calls))
            (otherwise :other))
          calls
          (acase ?b
            (?a "a")
            (?c "c")
            (?d "d")
            (otherwise
             (string ?b it)))
          (acase 'ready
            ((pending waiting)
             :not-yet)
            ((ready complete)
             (list
              :state it)))))"##;
    let expect = expect![[r#"OK ((:matched 1 1) 1 "bb" (:state ready))"#]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_case_clause_keys_do_not_evaluate_or_mutate_the_bound_value() {
    let elisp_form = r##"(let ((value 0))
         (acase
             (progn
               (setq value
                     (1+ value))
               value)
           (0 :zero)
           ((setq it
                  (1+ it))
            (list :mutated it))
           (1
            (list
             :selected
             it
             value))))"##;
    let expect = expect!["OK (:selected 1 1)"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_ecase_returns_selected_payloads_and_preserves_exact_miss_errors() {
    let elisp_form = r##"(list
         (aecase
             'write
           (read
            '(:permission read))
           (write
            (list
             :permission it
             :allowed t)))
         (condition-case error
             (aecase ?b
               (?a "a")
               (?c "c")
               (?d "d"))
           (error
            (list
             (car error)
             (cdr error)))))"##;
    let expect = expect![[
        r#"OK ((:permission write :allowed t) (error ("cl-ecase failed: 98, (97 99 100)")))"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_typecase_dispatches_real_data_shapes_and_returns_nil_without_match() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (atypecase value
             (integer
              (list
               :integer it
               (1+ it)))
             (float
              (list
               :float it
               (truncate it)))
             (string
              (list
               :string
               (length it)
               (upcase it)))
             (cons
              (list
               :list
               (car it)
               (length it)))
             (hash-table
              (list
               :table
               (hash-table-count
                it)))))
         (list
          7
          1.75
          "Ada"
          '(alpha beta)
          (let ((table
                 (make-hash-table
                  :test 'equal)))
            (puthash "x" 1 table)
            table)
          (make-vector 2 :x)))"##;
    let expect = expect![[
        r#"OK ((:integer 7 8) (:float 1.75 1) (:string 3 "ADA") (:list alpha 2) (:table 1) nil)"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_etypecase_dispatches_supported_values_and_reports_exact_type_errors() {
    let elisp_form = r##"(list
         (aetypecase 1.0
           (integer
            (+ 2 it))
           (float
            (1- it)))
         (aetypecase
             '("one" "two")
           (string
            :string)
           (list
            (mapcar #'upcase
                    it)))
         (condition-case error
             (aetypecase
                 "Foo"
               (fixnum :number)
               (hash-table :table))
           (error
            (list
             (car error)
             (cdr error)))))"##;
    let expect = expect![[
        r#"OK (0.0 ("ONE" "TWO") (error ("cl-etypecase failed: Foo, (fixnum hash-table)")))"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_pcase_supports_destructuring_guards_and_fallbacks_with_it() {
    let elisp_form = r##"(mapcar
         (lambda (event)
           (apcase event
             ((and
               `(:user ,name
                       :score ,score)
               (guard
                (>= score 10)))
              (list
               :promote
               name
               score
               it))
             (`(:user ,name
                      :score ,score)
              (list
               :retain
               name
               score
               it))
             (_
              (list
               :unknown it))))
         '((:user "Ada"
                  :score 12)
           (:user "Grace"
                  :score 7)
           (:system shutdown)))"##;
    let expect = expect![[
        r#"OK ((:promote "Ada" 12 (:user "Ada" :score 12)) (:retain "Grace" 7 (:user "Grace" :score 7)) (:unknown (:system shutdown)))"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_long_and_short_dispatch_names_produce_identical_practical_results() {
    let elisp_form = r##"(list
         (equal
          (acase 2
            (1 :one)
            (2 (list :two it)))
          (anaphoric-case 2
            (1 :one)
            (2 (list :two it))))
         (equal
          (atypecase
              "value"
            (string
             (list
              (length it)
              it)))
          (anaphoric-typecase
              "value"
            (string
             (list
              (length it)
              it))))
         (equal
          (apcase
              '(ok 204)
            (`(ok ,code)
             (list it code)))
          (anaphoric-pcase
              '(ok 204)
            (`(ok ,code)
             (list it code)))))"##;
    let expect = expect!["OK (t t t)"];
    assert_anaphora_parity(elisp_form, expect);
}
