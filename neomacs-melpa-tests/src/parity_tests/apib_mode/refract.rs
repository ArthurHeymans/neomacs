use expect_test::expect;

use super::{assert_apib_mode_parity, assert_apib_mode_signal_parity};

#[test]
fn element_predicate_distinguishes_refract_types_across_realistic_elements() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (list
    (car case)
    (apib-refract-element-p (cadr case) (car case))))
 '(("parseResult" (:element "parseResult" :content []))
   ("asset" (:element "asset" :attributes (:contentType (:content "application/json"))))
   ("category" (:element "asset"))
   ("" (:element ""))
   ("asset" (:content "body"))))"##;
    let expect =
        expect![[r#"OK (("parseResult" t) ("asset" t) ("category" nil) ("" t) ("asset" nil))"#]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn element_predicate_signals_for_non_string_element_tags_like_gnu_emacs() {
    let elisp_form = r##"(apib-refract-element-p
 '(:element 42 :content "invalid")
 "asset")"##;
    let expect = expect!["ERR (wrong-type-argument stringp 42)"];
    assert_apib_mode_signal_parity(elisp_form, expect);
}

#[test]
fn refract_mapc_walks_nested_vectors_elements_attributes_and_scalar_content_in_order() {
    let elisp_form = r##"(let
    ((document
      '(:element "parseResult"
        :content
        [(:element "category"
          :meta (:classes ["api"])
          :content
          [(:element "asset"
            :attributes
            (:contentType
             (:element "string"
              :content "application/json"))
            :content "{\"status\":\"ok\"}")
           (:element "asset"
            :attributes
            (:contentType
             (:element "string"
              :content "application/schema+json"))
            :content "{\"type\":\"object\"}")])]))
     visited)
  (apib-refract-mapc
   (lambda (value)
     (push
      (cond
       ((vectorp value) (list 'vector (length value)))
       ((stringp value) (list 'scalar value))
       (t (list 'element (plist-get value :element))))
      visited))
   document)
  (nreverse visited))"##;
    let expect = expect![[
        r#"OK ((element "parseResult") (vector 1) (element "category") (vector 2) (element "asset") (scalar "{\"status\":\"ok\"}") (element "asset") (scalar "{\"type\":\"object\"}"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn refract_mapc_handles_empty_nil_and_vector_only_trees_without_extra_callbacks() {
    let elisp_form = r##"(list
 (let (visited)
   (apib-refract-mapc
    (lambda (value) (push value visited))
    nil)
   visited)
 (let (visited)
   (apib-refract-mapc
    (lambda (value)
      (push
       (if (vectorp value)
           (list 'vector (length value))
         (plist-get value :element))
       visited))
    [(:element "one") (:element "two")])
   (nreverse visited))
 (let (visited)
   (apib-refract-mapc
    (lambda (value) (push value visited))
    [])
   (nreverse visited)))"##;
    let expect = expect![[r#"OK (nil ((vector 2) "one" "two") ([]))"#]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn refract_mapc_visits_sibling_assets_even_when_content_is_nil_or_empty() {
    let elisp_form = r##"(let
    ((document
      '(:element "parseResult"
        :content
        [(:element "asset" :content nil)
         (:element "asset" :content "")
         (:element "asset" :content "{}")]))
     assets)
  (apib-refract-mapc
   (lambda (value)
     (when (and (listp value)
                (apib-refract-element-p value "asset"))
       (push (plist-get value :content) assets)))
   document)
  (nreverse assets))"##;
    let expect = expect![[r#"OK (nil "" "{}")"#]];
    assert_apib_mode_parity(elisp_form, expect);
}
