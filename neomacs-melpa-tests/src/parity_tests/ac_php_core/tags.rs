use expect_test::expect;

use super::assert_ac_php_core_parity;

#[test]
fn ac_php_core_tag_data_accessors_and_function_lookup_preserve_identity() {
    let elisp_form = r##"(let* ((classes
                     (make-hash-table
                      :test
                      'case-fold))
                    (functions
                     (make-hash-table
                      :test
                      'case-fold))
                    (inherits
                     (make-hash-table
                      :test
                      'case-fold))
                    (files
                     ["/one.php"
                      "/two.php"])
                    (tags-data
                     (list
                      classes
                      functions
                      inherits
                      files
                      "/project/"))
                    (item
                     ["f"
                      "\\App\\run("
                      "$arg"
                      "1:2"
                      "Result"]))
               (puthash
                "\\App\\run("
                item
                functions)
               (list
                (eq
                 classes
                 (ac-php-g--class-map
                  tags-data))
                (eq
                 functions
                 (ac-php-g--function-map
                  tags-data))
                (eq
                 inherits
                 (ac-php-g--inherit-map
                  tags-data))
                (eq
                 files
                 (ac-php-g--file-list
                  tags-data))
                (ac-php-g--project-root-dir
                 tags-data)
                (eq
                 item
                 (ac-php--get-item-from-funtion-map
                  "\\app\\RUN("
                  functions))
                (ac-php--get-item-from-funtion-map
                 "\\missing"
                 functions)))"##;
    let expect = expect![[r#"OK (t t t t "/project/" t nil)"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_inheritance_resolution_handles_namespace_global_parents_and_cycles() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace App;\nclass Child {}\n")
               (php-mode)
               (goto-char
                (point-max))
               (let ((classes
                      (make-hash-table
                       :test
                       'case-fold))
                     (inherits
                      (make-hash-table
                       :test
                       'case-fold)))
                 (dolist
                     (name
                      '("\\App\\Child"
                        "\\App\\Base"
                        "\\Contract"
                        "\\CycleA"
                        "\\CycleB"))
                   (puthash
                    name
                    []
                    classes))
                 (puthash
                  "\\App\\Child"
                  ["Base"
                   "\\Contract"]
                  inherits)
                 (puthash
                  "\\App\\Base"
                  []
                  inherits)
                 (puthash
                  "\\Contract"
                  []
                  inherits)
                 (puthash
                  "\\CycleA"
                  ["\\CycleB"]
                  inherits)
                 (puthash
                  "\\CycleB"
                  ["\\CycleA"]
                  inherits)
                 (list
                  (ac-php--get-check-class-list
                   "Child"
                   inherits
                   classes)
                  (ac-php--get-check-class-list
                   "\\App\\Child"
                   inherits
                   classes)
                  (ac-php--get-check-class-list
                   "\\CycleA"
                   inherits
                   classes)
                  (ac-php--get-check-class-list
                   "Missing"
                   inherits
                   classes))))"##;
    let expect = expect![[
        r#"OK ((#("\\App\\Child" 1 4 (pos 7)) #("\\App\\Base" 1 4 (pos 7)) "\\Contract") ("\\App\\Child" "\\App\\Base" "\\Contract") ("\\CycleA" "\\CycleB" "\\CycleA") nil)"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_member_lookup_and_listing_use_case_fold_last_definition_and_unique_names() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace App;\nclass Child {}\n")
               (php-mode)
               (goto-char
                (point-max))
               (let ((classes
                      (make-hash-table
                       :test
                       'case-fold))
                     (inherits
                      (make-hash-table
                       :test
                       'case-fold))
                     (base-first
                      ["p" "shared" "base help" "0:1" "BaseType" "\\App\\Base" "public" "0"])
                     (base-only
                      ["m" "baseOnly(" "$x" "0:2" "void" "\\App\\Base" "protected" "0"])
                     (child-first
                      ["p" "shared" "child old" "1:1" "OldType" "\\App\\Child" "private" "0"])
                     (child-last
                      ["p" "SHARED" "child new" "1:2" "NewType" "\\App\\Child" "public" "1"])
                     (child-only
                      ["m" "childOnly(" "" "1:3" "self" "\\App\\Child" "public" "0"]))
                 (puthash
                  "\\App\\Base"
                  (vector
                   base-first
                   base-only)
                  classes)
                 (puthash
                  "\\App\\Child"
                  (vector
                   child-first
                   child-last
                   child-only)
                  classes)
                 (puthash
                  "\\App\\Child"
                  ["\\App\\Base"]
                  inherits)
                 (puthash
                  "\\App\\Base"
                  []
                  inherits)
                 (list
                  (ac-php-get-class-member-info
                   classes
                   inherits
                   "\\App\\Child"
                   "shared")
                  (ac-php-get-class-member-info
                   classes
                   inherits
                   "\\App\\Child"
                   "CHILDONLY(")
                  (ac-php-get-class-member-info
                   classes
                   inherits
                   "\\App\\Child"
                   "missing")
                  (ac-php-get-class-member-list
                   classes
                   inherits
                   "\\App\\Child"))))"##;
    let expect = expect![[
        r#"OK (["p" "SHARED" "child new" "1:2" "NewType" "\\App\\Child" "public" "1"] #1=["m" "childOnly(" "" "1:3" "self" "\\App\\Child" "public" "0"] nil (["m" "baseOnly(" "$x" "0:2" "void" "\\App\\Base" "protected" "0"] #1# ["p" "shared" "child old" "1:1" "OldType" "\\App\\Child" "private" "0"]))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_class_chain_resolution_covers_parent_self_global_relative_and_missing_types() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace App;\nclass Child {}\n")
               (php-mode)
               (goto-char
                (point-max))
               (let* ((classes
                       (make-hash-table
                        :test
                        'case-fold))
                      (inherits
                       (make-hash-table
                        :test
                        'case-fold))
                      (functions
                       (make-hash-table
                        :test
                        'case-fold))
                      (files
                       [])
                      (tags-data
                       (list
                        classes
                        functions
                        inherits
                        files
                        "/project/"))
                      calls)
                 (puthash
                  "\\App\\Child"
                  [["m" "instance(" "" "0:1" "self" "\\App\\Child" "public" "1"]
                   ["p" "relative" "" "0:2" "LocalResult" "\\App\\Child" "public" "0"]
                   ["p" "global" "" "0:3" "\\Shared\\Result" "\\App\\Child" "public" "0"]
                   ["p" "unknown" "" "0:4" nil "\\App\\Child" "public" "0"]]
                  classes)
                 (puthash
                  "\\App\\Base"
                  []
                  classes)
                 (puthash
                  "\\App\\LocalResult"
                  []
                  classes)
                 (puthash
                  "\\Shared\\Result"
                  []
                  classes)
                 (puthash
                  "\\App\\Child"
                  ["\\App\\Base"]
                  inherits)
                 (puthash
                  "\\App\\Base"
                  []
                  inherits)
                 (cl-letf
                     (((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push arguments calls)
                         'messaged)))
                   (list
                    (mapcar
                     (lambda (chain)
                       (list
                        chain
                        (ac-php-get-class-name-by-key-list
                         tags-data
                         chain)))
                     '("\\App\\Child"
                       "\\App\\Child.__parent__"
                       "\\App\\Child.instance("
                       "\\App\\Child.relative"
                       "\\App\\Child.global"
                       "\\App\\Child.unknown"
                       "\\App\\Child.missing"
                       "\\Missing.member"))
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((("\\App\\Child" "\\App\\Child") ("\\App\\Child.__parent__" "\\App\\Base") ("\\App\\Child.instance(" "\\App\\Child") ("\\App\\Child.relative" "\\App\\LocalResult") ("\\App\\Child.global" "\\Shared\\Result") ("\\App\\Child.unknown" nil) ("\\App\\Child.missing" "") ("\\Missing.member" "")) ((" class[\\App\\Child]'s member[missing] not define type ")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_candidate_class_formats_static_properties_metadata_order_and_deduplication() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace App;\nclass Child {}\n")
               (php-mode)
               (goto-char
                (point-max))
               (let* ((classes
                       (make-hash-table
                        :test
                        'case-fold))
                      (inherits
                       (make-hash-table
                        :test
                        'case-fold))
                      (tags-data
                       (list
                        classes
                        (make-hash-table
                         :test
                         'case-fold)
                        inherits
                        []
                        "/project/")))
                 (puthash
                  "\\App\\Child"
                  [["p" "staticValue" "static docs" "0:1" "int" "\\App\\Child" "public" "1"]
                   ["p" "value" "property docs" "0:2" "string" "\\App\\Child" "private" "0"]
                   ["m" "run(" "$arg" "0:3" "Result" "\\App\\Child" "protected" "0"]
                   ["m" "RUN(" "duplicate" "0:4" "Other" "\\App\\Child" "public" "0"]]
                  classes)
                 (puthash
                  "\\App\\Child"
                  []
                  inherits)
                 (mapcar
                  (lambda (candidate)
                    (list
                     (substring-no-properties
                      candidate)
                     (mapcar
                      (lambda (property)
                        (get-text-property
                         0
                         property
                         candidate))
                      '(ac-php-help
                        ac-php-return-type
                        ac-php-tag-type
                        ac-php-access
                        ac-php-static
                        ac-php-from
                        summary))))
                  (ac-php-candidate-class
                   tags-data
                   "\\App\\Child.partial"))))"##;
    let expect = expect![[
        r#"OK (("$staticValue" ("static docs" "int" "p" "public" "1" "\\App\\Child" "int")) ("value" ("property docs" "string" "p" "private" "0" "\\App\\Child" "string")) ("run(" ("$arg" "Result" "m" "protected" "0" "\\App\\Child" "Result")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_buffer_class_full_name_resolution_covers_use_current_root_and_return_modes() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace App;\nuse Vendor\\Package\\Thing as Alias;\nclass Current {}\n")
               (php-mode)
               (goto-char
                (point-max))
               (let ((functions
                      (make-hash-table
                       :test
                       'case-fold)))
                 (dolist
                     (item
                      '(["c" "\\Vendor\\Package\\Thing" "" "0:1" "VendorResult"]
                        ["c" "\\Vendor\\Package\\Thing\\Nested" "" "0:2" "NestedResult"]
                        ["c" "\\App\\Local" "" "0:3" "LocalResult"]
                        ["c" "\\RootOnly" "" "0:4" "RootResult"]
                        ["c" "\\App\\Current" "" "0:5" "CurrentResult"]))
                   (puthash
                    (aref item 1)
                    item
                    functions))
                 (mapcar
                  (lambda (fixture)
                    (list
                     (car fixture)
                     (ac-php--get-class-full-name-in-cur-buffer
                      (car fixture)
                      functions
                      (cdr fixture))))
                  '(("Alias")
                    ("Alias\\Nested")
                    ("Local")
                    ("RootOnly")
                    ("\\RootOnly")
                    ("this")
                    ("Missing")
                    ("Alias" . t)
                    ("Local" . t)
                    ("this" . t)))))"##;
    let expect = expect![[
        r#"OK (("Alias" "\\Vendor\\Package\\Thing") ("Alias\\Nested" "\\Vendor\\Package\\Thing\\Nested") ("Local" "\\App\\Local") ("RootOnly" "\\RootOnly") ("\\RootOnly" "\\RootOnly") ("this" "\\App\\Current") ("Missing" nil) ("Alias" "VendorResult") ("Local" "LocalResult") ("this" "CurrentResult"))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_candidate_other_covers_global_alias_namespace_root_keyword_and_local_variables() {
    let elisp_form = r##"(let ((functions
                    (make-hash-table
                     :test
                     'case-fold))
                   (variables
                    (make-hash-table
                     :test
                     'case-fold))
                   (ac-php-prefix-str
                    "")
                   (tags-data
                    nil))
               (dolist
                   (item
                    '(["c" "\\Acme\\Service" "service docs" "0:1" "Service"]
                      ["c" "\\Vendor\\Thing" "thing docs" "0:2" "Thing"]
                      ["c" "\\App\\LocalThing" "local docs" "0:3" "LocalThing"]
                      ["c" "\\StringThing" "root docs" "0:4" "StringThing"]))
                 (puthash
                  (aref item 1)
                  item
                  functions))
               (puthash
                "$local"
                nil
                variables)
               (puthash
                "$lower"
                nil
                variables)
               (setq
                tags-data
                (list
                 (make-hash-table
                  :test
                  'case-fold)
                 functions
                 (make-hash-table
                  :test
                  'case-fold)
                 []
                 "/project/"))
               (cl-letf
                   (((symbol-function
                      'ac-php--get-all-use-as-name-in-cur-buffer)
                     (lambda ()
                       '(("\\Vendor\\Thing"
                          "Alias"))))
                    ((symbol-function
                      'ac-php-get-cur-namespace-name)
                     (lambda (&optional trim)
                       (if trim
                           "\\App"
                         "\\App\\")))
                    ((symbol-function
                      'ac-php--get-cur-function-vars)
                     (lambda ()
                       variables)))
                 (mapcar
                  (lambda (word)
                    (with-temp-buffer
                      (insert word)
                      (let ((candidates
                             (ac-php-candidate-other
                              tags-data)))
                        (list
                         word
                         (sort
                          (mapcar
                           (lambda (candidate)
                             (list
                              (substring-no-properties
                               candidate)
                              (get-text-property
                               0
                               'ac-php-tag-type
                               candidate)
                              (get-text-property
                               0
                               'ac-php-return-type
                               candidate)))
                           candidates)
                          (lambda (left right)
                            (string-lessp
                             (car left)
                             (car right))))))))
                  '("\\Ac"
                    "Ali"
                    "Loc"
                    "Str"
                    "ret"
                    "$lo"
                    ""))))"##;
    let expect = expect![[
        r#"OK (("\\Ac" (("\\Acme\\Service" "c" "Service"))) ("Ali" (("as" "\\Vendor\\Thing" "\\Vendor\\Thing"))) ("Loc" (("alThing" "c" "LocalThing"))) ("Str" (("ingThing" "c" "StringThing"))) ("ret" (("return" "k" ""))) ("$lo" (("$local" "v" "") ("$lower" "v" ""))) ("" nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_candidate_dispatch_selects_class_or_other_branch_once() {
    let elisp_form = r##"(let ((tags-data
                    'tags)
                   (class-at-point
                    "Class.member")
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-data)
                     (lambda ()
                       (push
                        'tags
                        calls)
                       tags-data))
                    ((symbol-function
                      'ac-php-get-class-at-point)
                     (lambda (received)
                       (push
                        (list
                         'class-at
                         received)
                        calls)
                       class-at-point))
                    ((symbol-function
                      'ac-php-candidate-class)
                     (lambda
                         (received
                          key-list)
                       (push
                        (list
                         'class
                         received
                         key-list)
                        calls)
                       'class-candidates))
                    ((symbol-function
                      'ac-php-candidate-other)
                     (lambda (received)
                       (push
                        (list
                         'other
                         received)
                        calls)
                       'other-candidates)))
                 (let ((class-result
                        (ac-php-candidate)))
                   (setq
                    class-at-point
                    nil)
                   (let ((other-result
                          (ac-php-candidate)))
                     (list
                      class-result
                      other-result
                      (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (class-candidates other-candidates (tags (class-at tags) (class tags "Class.member") tags (class-at tags) (other tags)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}
