use expect_test::expect;

use super::assert_align_cljlet_parity;

#[test]
fn align_cljlet_recognizes_every_supported_form_and_literal_map_at_the_opening_delimiter() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char (point-min))
     (list source (acl-found-alignable-form))))
 '("(let [x 1])" "(for [x xs])" "(when-let [x 1])"
   "(if-let [x 1])" "(binding [*x* 1])" "(loop [x 1])"
   "(with-open [r input])" "(cond true 1)" "(condp = x 1 2)"
   "(defroutes app (GET \"/\" [] ok))" "(case x 1 :one)"
   "(alt! ch1 ch2)" "{:host \"localhost\" :port 80}"
   "(map vector xs)" "[x 1]" "ordinary-symbol"))"##;
    let expect = expect![[
        r#"OK (("(let [x 1])" 0) ("(for [x xs])" 0) ("(when-let [x 1])" 5) ("(if-let [x 1])" 3) ("(binding [*x* 1])" 0) ("(loop [x 1])" 0) ("(with-open [r input])" 0) ("(cond true 1)" 0) ("(condp = x 1 2)" 0) ("(defroutes app (GET \"/\" [] ok))" 0) ("(case x 1 :one)" 0) ("(alt! ch1 ch2)" 0) ("{:host \"localhost\" :port 80}" t) ("(map vector xs)" nil) ("[x 1]" nil) ("ordinary-symbol" nil))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_preserves_its_historical_form_name_matching_boundaries() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char (point-min))
     (list source (acl-found-alignable-form))))
 '("(letfn [(f [])])" "(outlet [x 1])" "(letter [x 1])"
   "(before [x xs])" "(format [x xs])" "(condiment x)"
   "(showcase x)" "(alternate x)" "(my-binding [x 1])"
   "(namespace/let [x 1])"))"##;
    let expect = expect![[
        r#"OK (("(letfn [(f [])])" 0) ("(outlet [x 1])" 3) ("(letter [x 1])" 0) ("(before [x xs])" 2) ("(format [x xs])" 0) ("(condiment x)" 0) ("(showcase x)" 4) ("(alternate x)" 0) ("(my-binding [x 1])" 3) ("(namespace/let [x 1])" 10))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_finds_the_nearest_nested_alignable_form_from_real_body_positions() {
    let elisp_form = r##"(mapcar
 (lambda (needle)
   (with-temp-buffer
     (clojure-mode)
     (insert "(defn process [rows]\n  (let [limit 10\n        selected (for [row rows\n                      :when (> row limit)]\n                   {:row row\n                    :large true})]\n    selected))")
     (goto-char (point-min))
     (search-forward needle)
     (condition-case err
         (progn
           (acl-backward-to-code)
           (acl-find-alignable-form)
           (list needle
                 (buffer-substring-no-properties
                  (point) (min (point-max) (+ (point) 12)))
                 (point) (current-column)))
       (error
        (list needle (car err) (error-message-string err))))))
 '("limit" ":when" ":row" "\n    selected"))"##;
    let expect = expect![[
        r#"OK (("limit" "(let [limit " 24 2) (":when" "(for [row ro" 56 17) (":row" "{:row row\n  " 133 19) ("\n    selected" "(let [limit " 24 2))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_detects_discarded_reader_forms_from_prefix_body_and_whitespace_positions() {
    let elisp_form = r##"(with-temp-buffer
  (clojure-mode)
  (insert "#_(slow-call 1)\n  #_ {:old true}\nactive")
  (mapcar
   (lambda (position)
     (goto-char position)
     (list position
           (buffer-substring-no-properties
            position (min (point-max) (+ position 4)))
           (acl-is-commented?)))
   (number-sequence 1 (point-max))))"##;
    let expect = expect![[
        r##"OK ((1 "#_(s" t) (2 "_(sl" nil) (3 "(slo" t) (4 "slow" nil) (5 "low-" nil) (6 "ow-c" nil) (7 "w-ca" nil) (8 "-cal" nil) (9 "call" nil) (10 "all " nil) (11 "ll 1" nil) (12 "l 1)" nil) (13 " 1)\n" nil) (14 "1)\n " nil) (15 ")\n  " nil) (16 "\n  #" nil) (17 "  #_" t) (18 " #_ " t) (19 "#_ {" t) (20 "_ {:" nil) (21 " {:o" t) (22 "{:ol" nil) (23 ":old" nil) (24 "old " nil) (25 "ld t" nil) (26 "d tr" nil) (27 " tru" nil) (28 "true" nil) (29 "rue}" nil) (30 "ue}\n" nil) (31 "e}\na" nil) (32 "}\nac" nil) (33 "\nact" nil) (34 "acti" nil) (35 "ctiv" nil) (36 "tive" nil) (37 "ive" nil) (38 "ve" nil) (39 "e" nil) (40 "" nil))"##
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_forward_logical_sexp_skips_metadata_tags_and_discarded_forms() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (clojure-mode)
     (insert (car case))
     (goto-char (cadr case))
     (acl-forward-sexp (caddr case))
     (list case (point)
           (buffer-substring-no-properties
            (point-min) (point))
           (buffer-substring-no-properties
            (point) (point-max)))))
 '(("alpha beta" 1 nil)
   ("alpha beta" 6 nil)
   ("#foo/bar [1 2 3] [4 5]" 1 nil)
   ("^long value tail" 1 nil)
   ("#_(ignored call) value tail" 1 nil)
   ("value #_(ignored call) tail" 1 nil)
   ("value #_(ignored call) tail" 6 nil)
   ("#_(ignored call) value tail" 1 t)))"##;
    let expect = expect![[
        r##"OK ((("alpha beta" 1 nil) 6 "alpha" " beta") (("alpha beta" 6 nil) 11 "alpha beta" "") (("#foo/bar [1 2 3] [4 5]" 1 nil) 17 "#foo/bar [1 2 3]" " [4 5]") (("^long value tail" 1 nil) 12 "^long value" " tail") (("#_(ignored call) value tail" 1 nil) 23 "#_(ignored call) value" " tail") (("value #_(ignored call) tail" 1 nil) 6 "value" " #_(ignored call) tail") (("value #_(ignored call) tail" 6 nil) 28 "value #_(ignored call) tail" "") (("#_(ignored call) value tail" 1 t) 3 "#_" "(ignored call) value tail"))"##
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_pair_navigation_width_and_remaining_sexp_probes_match_real_bindings() {
    let elisp_form = r##"(with-temp-buffer
  (clojure-mode)
  (insert "[short 1\n^String medium-name \"two\"\n#_(old) longest-binding (+ 1 2)]")
  (goto-char 2)
  (let (records)
    (dotimes (index 3)
      (push
       (list (point)
             (thing-at-point 'sexp t)
             (acl-get-width)
             (acl-has-next-sexp)
             (acl-check-for-another-sexp)
             (save-excursion
               (acl-goto-next-pair)
               (list (point) (thing-at-point 'sexp t))))
       records)
      (when (< index 2)
        (acl-goto-next-pair)))
    (list (nreverse records)
          (progn (goto-char 2) (acl-calc-width))
          (acl-take-n 0 '(a b c))
          (acl-take-n 2 '(a b c))
          (acl-take-n 9 '(a b c)))))"##;
    let expect = expect![[
        r#"OK (((2 "short" 5 t t (10 "^String")) (10 "^String" 19 t t (44 "longest-binding")) (44 "longest-binding" 15 t t (67 "(+ 1 2)"))) 19 nil (a b) (a b c))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_position_to_start_handles_each_special_form_header() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char (point-min))
     (acl-position-to-start)
     (list source (point) (thing-at-point 'sexp t)
           (current-column))))
 '("(let [alpha 1 beta 2])"
   "(cond alpha 1 beta 2)"
   "(condp = value 1 :one 2 :two)"
   "(case value 1 :one 2 :two)"
   "(alt! channel-a channel-b)"
   "{:alpha 1 :beta 2}"))"##;
    let expect = expect![[
        r#"OK (("(let [alpha 1 beta 2])" 7 "alpha" 6) ("(cond alpha 1 beta 2)" 7 "alpha" 6) ("(condp = value 1 :one 2 :two)" 16 "1" 15) ("(case value 1 :one 2 :two)" 13 "1" 12) ("(alt! channel-a channel-b)" 7 "channel-a" 6) ("{:alpha 1 :beta 2}" 2 ":alpha" 1))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}
