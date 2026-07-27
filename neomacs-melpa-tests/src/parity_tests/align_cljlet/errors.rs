use expect_test::expect;

use super::assert_align_cljlet_parity;

#[test]
fn align_cljlet_rejects_multiple_binding_pairs_on_one_line_without_partial_edits() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (let ((before (buffer-string)))
       (condition-case err
           (progn (align-cljlet) 'unexpected-success)
         (error
          (list (car err) (error-message-string err)
                (equal before (buffer-string))
                (buffer-string) (point)))))))
 '("(let [apple 2 pear 3\n peach 23] (+ apple pear peach))"
   "{:a 1 :b 2\n :long-key 3}"
   "(cond one 1 two 2\n :else 3)"))"##;
    let expect = expect![[
        r#"OK ((error "multiple pairs on one line" t "(let [apple 2 pear 3\n peach 23] (+ apple pear peach))" 2) (error "multiple pairs on one line" t "{:a 1 :b 2\n :long-key 3}" 2) (error "multiple pairs on one line" t "(cond one 1 two 2\n :else 3)" 2))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_reports_not_in_let_form_from_top_level_code_strings_and_comments() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (clojure-mode)
     (insert (car case))
     (goto-char (cadr case))
     (condition-case err
         (progn (align-cljlet) 'unexpected-success)
       (error
        (list case (car err) (error-message-string err)
              (point) (buffer-string))))))
 '(("(println :top-level)" 10)
   ("\"a string mentioning (let [x 1])\"" 20)
   (";; (let [commented 1])\n(+ 1 2)" 10)
   ("[1 2 3]" 4)))"##;
    let expect = expect![[
        r#"OK ((("(println :top-level)" 10) error "Not in a \"let\" form" 10 "(println :top-level)") (("\"a string mentioning (let [x 1])\"" 20) error "Not in a \"let\" form" 20 "\"a string mentioning (let [x 1])\"") ((";; (let [commented 1])\n(+ 1 2)" 10) error "Not in a \"let\" form" 10 ";; (let [commented 1])\n(+ 1 2)") (("[1 2 3]" 4) error "Not in a \"let\" form" 4 "[1 2 3]"))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_navigation_primitives_have_exact_boundary_failure_contracts() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (clojure-mode)
   (insert "atom")
   (goto-char (point-min))
   (condition-case err
       (acl-try-go-up)
     (error (list (car err) (error-message-string err) (point)))))
 (with-temp-buffer
   (clojure-mode)
   (insert "single")
   (goto-char (point-max))
   (list (acl-goto-next-pair)
         (acl-next-sexp)
         (acl-has-next-sexp)
         (acl-check-for-another-sexp)
         (point)))
 (with-temp-buffer
   (clojure-mode)
   (insert "()")
   (goto-char 2)
   (condition-case err
       (acl-get-width)
     (error (list (car err) (error-message-string err) (point))))))"##;
    let expect = expect![[
        r#"OK ((error "Not in a \"let\" form" 1) (t t t t 1) (scan-error "Scan error: \"Containing expression ends prematurely\", 2, 3" 2))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_malformed_clojure_forms_signal_without_silently_corrupting_the_buffer() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (let ((before (buffer-string)))
       (condition-case err
           (progn (align-cljlet) 'unexpected-success)
         (error
          (list (car err) (error-message-string err)
                (equal before (buffer-string))
                (buffer-string)))))))
 '("(let [x 1\n longer-name 2"
   "(let [x 1\n longer-name] x)"
   "{:short 1\n :unclosed (vector 1 2}"
   "(cond true 1\n false)"))"##;
    let expect = expect![[
        r#"OK ((error "multiple pairs on one line" t "(let [x 1\n longer-name 2") unexpected-success (error "multiple pairs on one line" t "{:short 1\n :unclosed (vector 1 2}") unexpected-success)"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_width_and_take_helpers_preserve_historical_edge_case_behavior() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (n)
    (condition-case err
        (list n (acl-take-n n '(a b c d)))
      (error (list n (car err) (error-message-string err)))))
  '(-3 -1 0 1 4 5 100))
 (mapcar
  (lambda (source)
    (with-temp-buffer
      (clojure-mode)
      (insert source)
      (goto-char (point-min))
      (condition-case err
          (list source (acl-get-width))
        (error (list source (car err) (error-message-string err))))))
  '("ascii" "αβγ" ":namespaced/key" "[a b c]" "#{:a :b}")))"##;
    let expect = expect![[
        r##"OK (((-3 nil) (-1 nil) (0 nil) (1 (a)) (4 #1=(a b c d)) (5 #1#) (100 #1#)) (("ascii" 5) ("αβγ" 3) (":namespaced/key" 15) ("[a b c]" 7) ("#{:a :b}" 8)))"##
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}
