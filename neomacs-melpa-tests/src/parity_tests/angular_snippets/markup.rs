use expect_test::expect;

use super::assert_angular_snippets_parity;

#[test]
fn angular_snippets_finds_closest_directive_across_realistic_multiline_markup() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<article\n"
   "  ng-controller=\"CatalogCtrl\"\n"
   "  class=\"catalog\"\n"
   "  ng-repeat=\"product in products\"\n"
   "  ng-class=\"{'sold-out': !product.stock}\">\n"
   "  {{ product.name }}\n"
   "</article>")
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (list needle
           (point)
           (ng-snip/closest-ng-identifer)))
   '("CatalogCtrl"
     "catalog"
     "products"
     "sold-out"
     "product.name")))"##;
    let expect = expect![[
        r#"OK (("CatalogCtrl" 38 "ng-controller") ("catalog" 34 "ng-controller") ("products" 90 "ng-repeat") ("sold-out" 114 "ng-class") ("product.name" 152 "ng-class"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_closest_identifier_preserves_point_and_match_text() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<input ng-model=\"profile.email\" "
   "ng-change=\"validate(profile)\" />")
  (search-backward "profile.email")
  (let ((before (point)))
    (list
     before
     (ng-snip/closest-ng-identifer)
     (point)
     (match-string 0)
     (buffer-substring-no-properties
      (line-beginning-position)
      (line-end-position)))))"##;
    let expect = expect![[
        r#"OK (18 "ng-model" 18 "ng-model" "<input ng-model=\"profile.email\" ng-change=\"validate(profile)\" />")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_closest_identifier_signals_when_no_directive_precedes_point() {
    let elisp_form = r##"(with-temp-buffer
  (insert "<div class=\"plain\">content</div>")
  (goto-char (point-max))
  (condition-case error
      (ng-snip/closest-ng-identifer)
    (error
     (list
      (car error)
      (error-message-string error)
      (point)))))"##;
    let expect = expect![[r#"OK (end-of-buffer "End of buffer" 33)"#]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_closest_identifier_rejects_malformed_ng_token() {
    let elisp_form = r##"(with-temp-buffer
  (insert "<div ng-123=\"bad\">content</div>")
  (search-backward "bad")
  (condition-case error
      (ng-snip/closest-ng-identifer)
    (error
     (list
      (car error)
      (error-message-string error)
      (point)
      (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (error "No angular identifier at point" 14 "<div ng-123=\"bad\">content</div>")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_spacing_helper_only_inserts_before_adjacent_attribute_text() {
    let elisp_form = r##"(mapcar
  (lambda (case)
    (with-temp-buffer
      (insert (car case))
      (goto-char
       (or
        (cdr case)
        (point-max)))
      (let ((before (point))
            (result
             (ng-snip/maybe-space-after-attr)))
        (list
         (car case)
         before
         result
         (point)
         (buffer-string)))))
  '(("ng-model=\"item\"ng-change=\"save()\"")
    ("ng-model=\"item\" ng-change=\"save()\"")
    ("ng-model=\"item\"/>")
    ("ng-model=\"item\">")
    ("ng-model=\"item\"/")
    ("ng-model=\"item\"")
    ("prefixsuffix" . 7)))"##;
    let expect = expect![[
        r#"OK (("ng-model=\"item\"ng-change=\"save()\"" 34 nil 34 "ng-model=\"item\"ng-change=\"save()\"") ("ng-model=\"item\" ng-change=\"save()\"" 35 nil 35 "ng-model=\"item\" ng-change=\"save()\"") ("ng-model=\"item\"/>" 18 nil 18 "ng-model=\"item\"/>") ("ng-model=\"item\">" 17 nil 17 "ng-model=\"item\">") ("ng-model=\"item\"/" 17 nil 17 "ng-model=\"item\"/") ("ng-model=\"item\"" 16 nil 16 "ng-model=\"item\"") ("prefixsuffix" 7 nil 8 "prefix suffix"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_spacing_helper_supports_repeated_attribute_construction() {
    let elisp_form = r##"(with-temp-buffer
  (insert "<button ")
  (dolist
      (attribute
       '("ng-click=\"submit()\""
         "ng-disabled=\"form.$invalid\""
         "class=\"primary\""))
    (insert attribute)
    (ng-snip/maybe-space-after-attr))
  (insert ">Save</button>")
  (list
   (buffer-string)
   (point)
   (looking-at-p "\\'")
   (save-excursion
     (goto-char (point-min))
     (while
         (search-forward "ng-" nil t))
     (point))))"##;
    let expect = expect![[
        r#"OK ("<button ng-click=\"submit()\"ng-disabled=\"form.$invalid\"class=\"primary\">Save</button>" 84 t 31)"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}
