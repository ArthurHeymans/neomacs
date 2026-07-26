use expect_test::expect;

use super::{assert_ac_php_core_parity, assert_ac_php_core_signal_parity};

#[test]
fn ac_php_core_private_symbol_lookup_returns_class_member_self_and_user_function_records() {
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
                    (member
                     ["m" "instance(" "" "0:12" "self" "\\App\\Child" "public" "1"])
                    (function
                     ["f" "\\App\\run(" "$arg" "1:20" "Result"])
                    (tags-data
                     (list
                      classes
                      functions
                      inherits
                      ["/class.php"
                       "/function.php"]
                      "/project/"))
                    (class-at-point
                     "\\App\\Child.partial")
                    (word
                     "instance("))
               (puthash
                "\\App\\Child"
                (vector
                 member)
                classes)
               (puthash
                "\\App\\Child"
                []
                inherits)
               (puthash
                "\\App\\run("
                function
                functions)
               (cl-letf
                   (((symbol-function
                      'ac-php-get-class-at-point)
                     (lambda
                         (_tags
                          &optional
                          _pos)
                       class-at-point))
                    ((symbol-function
                      'ac-php--get-cur-word-with-function-flag)
                     (lambda ()
                       word))
                    ((symbol-function
                      'ac-php--get-cur-word)
                     (lambda ()
                       (string-remove-suffix
                        "("
                        word)))
                    ((symbol-function
                      'ac-php--get-class-full-name-in-cur-buffer)
                     (lambda
                         (name
                          _map
                          _return)
                       (and
                        (string=
                         name
                         "run(")
                        "\\App\\run("))))
                 (let ((class-result
                        (ac-php-find-symbol-at-point-pri
                         tags-data)))
                   (setq
                    class-at-point
                    nil
                    word
                    "run(")
                   (let ((function-result
                          (ac-php-find-symbol-at-point-pri
                           tags-data)))
                     (setq
                      word
                      "run")
                     (let ((forced-function
                            (ac-php-find-symbol-at-point-pri
                             tags-data t))
                           (forced-id
                            (ac-php-find-symbol-at-point-pri
                             tags-data nil t)))
                       (list
                        class-result
                        function-result
                        forced-function
                        forced-id))))))"##;
    let expect = expect![[
        r#"OK (("class_member" "0:12" "\\App\\Child" ["m" "instance(" "" "0:12" "self" "\\App\\Child" "public" "1"]) ("user_function" "1:20" "Result" #1=["f" "\\App\\run(" "$arg" "1:20" "Result"]) ("user_function" "1:20" "Result" #1#) nil)"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_public_symbol_navigation_routes_user_class_system_and_local_fallbacks() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "symbol")
               (let* ((classes
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
                      (tags-data
                       (list
                        classes
                        functions
                        inherits
                        ["/class.php"
                         "/function.php"]
                        "/project/"))
                      results
                      dollar
                      calls)
                 (cl-letf
                     (((symbol-function
                        'ac-php-get-tags-data)
                       (lambda ()
                         tags-data))
                      ((symbol-function
                        'ac-php-get-cur-word-with-dollar)
                       (lambda ()
                         dollar))
                      ((symbol-function
                        'ac-php-find-symbol-at-point-pri)
                       (lambda (&rest _arguments)
                         (pop results)))
                      ((symbol-function
                        'ac-php-location-stack-push)
                       (lambda ()
                         (push
                          '(push)
                          calls)
                         'pushed))
                      ((symbol-function
                        'ac-php-goto-location)
                       (lambda
                           (location
                            &optional
                            other-window)
                         (push
                          (list
                           'goto location
                           other-window)
                          calls)
                         'jumped))
                      ((symbol-function
                        'ac-php--goto-local-var-def)
                       (lambda (variable)
                         (push
                          (list
                           'local variable)
                          calls)
                         'local-jump))
                      ((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'message
                           arguments)
                          calls)
                         'messaged)))
                   (setq
                    dollar
                    "run"
                    results
                    (list
                     (list
                      "user_function"
                      "1:20"
                      "Result"
                      ["f" "\\App\\run(" "$arg" "1:20" "Result"])))
                   (let ((user
                          (ac-php-find-symbol-at-point)))
                     (setq
                      dollar
                      "member"
                      results
                      (list
                       (list
                        "class_member"
                        "0:12"
                        "Result"
                        ["m" "member(" "" "0:12" "Result" "\\App\\Child" "public" "0"])))
                     (let ((class
                            (ac-php-find-symbol-at-point
                             t)))
                       (goto-char
                        (point-min))
                       (setq
                        dollar
                        "system"
                        results
                        (list
                         (list
                          "user_function"
                          "sys:1"
                          "Result"
                          ["f" "\\system(" "" "sys:1" "Result"])))
                       (let ((system
                              (ac-php-find-symbol-at-point)))
                         (setq
                          dollar
                          "$missing"
                          results
                          '(nil nil nil))
                         (let ((fallback
                                (ac-php-find-symbol-at-point)))
                           (list
                            user
                            class
                            system
                            fallback
                            (point)
                            (nreverse calls)))))))))"##;
    let expect = expect![[
        r#"OK (jumped jumped messaged local-jump 2 (#1=(push) (goto "/function.php:20" nil) #1# (goto "/class.php:12" nil) (message "need install : composer require jetbrains/phpstorm-stubs ") (local "$missing")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_public_symbol_navigation_exposes_vector_nth_signal_for_local_user_variables() {
    let elisp_form = r##"(let ((tags-data
                    (list
                     (make-hash-table)
                     (make-hash-table)
                     (make-hash-table)
                     []
                     "/project/")))
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-data)
                     (lambda ()
                       tags-data))
                    ((symbol-function
                      'ac-php-get-cur-word-with-dollar)
                     (lambda ()
                       "$local"))
                    ((symbol-function
                      'ac-php-find-symbol-at-point-pri)
                     (lambda (&rest _arguments)
                       (list
                        "user_function"
                        "0:1"
                        "Result"
                        ["v" "\\local" "" "0:1" "Result"]))))
                 (ac-php-find-symbol-at-point)))"##;
    let expect = expect![[r#"ERR (wrong-type-argument listp ["v" "\\local" "" "0:1" "Result"])"#]];

    assert_ac_php_core_signal_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_local_variable_jump_skips_comment_and_string_occurrences() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nfunction demo() {\n  // $target in comment\n  $text = \"$target in string\";\n  $target = make();\n  $target->run();\n}\n")
               (php-mode)
               (goto-char
                (point-max))
               (search-backward
                "$target->")
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'ac-php-location-stack-push)
                       (lambda ()
                         (push
                          (point)
                          calls)
                         'pushed)))
                   (let ((return
                          (ac-php--goto-local-var-def
                           "$target")))
                     (list
                      return
                      (point)
                      (buffer-substring-no-properties
                       (line-beginning-position)
                       (line-end-position))
                      (nreverse calls))))))"##;
    let expect = expect![[r#"OK (nil 89 "  $target = make();" (102))"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_generate_definition_copies_inferred_variable_or_property_annotations() {
    let elisp_form = r##"(let ((class-at-point
                    "\\App\\Service.make(")
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-data)
                     (lambda ()
                       'tags))
                    ((symbol-function
                      'ac-php--get-cur-word)
                     (lambda ()
                       "item"))
                    ((symbol-function
                      'ac-php-get-class-at-point)
                     (lambda (&rest _arguments)
                       class-at-point))
                    ((symbol-function
                      'ac-php-get-class-name-by-key-list)
                     (lambda
                         (_tags chain)
                       (push
                        (list
                         'class chain)
                        calls)
                       "\\App\\Result"))
                    ((symbol-function
                      'kill-new)
                     (lambda (text)
                       (push
                        (list
                         'kill text)
                        calls)
                       'copied)))
                 (with-temp-buffer
                   (insert
                    "$item = $service->make();")
                   (goto-char
                    (point-min))
                   (search-forward
                    "item")
                   (let ((variable
                          (call-interactively
                           'ac-php-gen-def)))
                     (setq
                      class-at-point
                      nil)
                     (erase-buffer)
                     (insert
                      "public Thing $item;")
                     (goto-char
                      (point-max))
                     (let ((property
                            (call-interactively
                             'ac-php-gen-def)))
                       (list
                        variable
                        property
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (copied copied ((class "\\App\\Service.make(") (kill "\n\11/**  @var \\App\\Result $item */\n") (kill "\n\11/**  @var <...> $item */\n")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_show_tip_formats_class_members_user_functions_and_missing_results() {
    let elisp_form = r##"(let ((results
                    (list
                     (list
                      "class_member"
                      "0:1"
                      "Result"
                      ["m" "run(" "[#$arg#]" "0:1" "Result" "\\App\\Base" "protected" "0"])
                     (list
                      "class_member"
                      "0:2"
                      "string"
                      ["p" "value" "docs" "0:2" "string" "\\App\\Base" "public" "0"])
                     (list
                      "user_function"
                      "sys:1"
                      "void"
                      ["f" "\\system(" "[#$arg#]" "sys:1" "void"])
                     nil))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-data)
                     (lambda ()
                       'tags))
                    ((symbol-function
                      'ac-php-find-symbol-at-point-pri)
                     (lambda (_tags)
                       (pop results)))
                    ((symbol-function
                      'popup-tip)
                     (lambda (text)
                       (push text calls)
                       'shown)))
                 (list
                  (ac-php-show-tip)
                  (ac-php-show-tip)
                  (ac-php-show-tip)
                  (ac-php-show-tip)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (shown shown shown nil ("run($arg )\n\11[  type]:Result\n\11[access]:protected\n\11[  from]:\\App\\Base" "value\n\11[  type]:string\n\11[access]:public\n\11[  from]:\\App\\Base" "[user]:\\system($arg )\n[  type]:void"))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_eldoc_formats_class_methods_properties_and_user_functions_with_faces() {
    let elisp_form = r##"(let ((results
                    (list
                     (list
                      "class_member"
                      "0:1"
                      "Result"
                      ["m" "run(" "$arg" "0:1" "Result" "\\App\\Base" "protected" "0"])
                     (list
                      "class_member"
                      "0:2"
                      "string"
                      ["p" "value" "" "0:2" "string" "\\App\\Base" "public" "0"])
                     (list
                      "user_function"
                      "1:3"
                      "void"
                      ["f" "\\App\\helper(" "$one,$two" "1:3" "void"])
                     nil)))
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-data)
                     (lambda ()
                       'tags))
                    ((symbol-function
                      'ac-php-find-symbol-at-point-pri)
                     (lambda (_tags)
                       (pop results))))
                 (mapcar
                  (lambda (_index)
                    (let ((document
                           (ac-php-eldoc-documentation-function)))
                      (and document
                           (list
                            (substring-no-properties
                             document)
                            (let (faces
                                  (position 0))
                              (while
                                  (< position
                                     (length document))
                                (let ((face
                                       (get-text-property
                                        position
                                        'face
                                        document)))
                                  (when face
                                    (push
                                     (list
                                      position
                                      face)
                                     faces)))
                                (setq
                                 position
                                 (or
                                  (next-single-property-change
                                   position
                                   'face
                                   document)
                                  (length document))))
                              (nreverse
                               faces))))))
                  '(1 2 3 4))))"##;
    let expect = expect![[
        r#"OK (("protected \\App\\Base::run($arg):Result" ((0 font-lock-keyword-face) (21 font-lock-function-name-face))) ("public \\App\\Base::value:string" ((0 font-lock-keyword-face) (18 font-lock-variable-name-face))) ("\\App\\helper($one,$two):void" ((0 font-lock-function-name-face))) nil)"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_show_project_info_reports_project_and_vendor_counts_exactly() {
    let elisp_form = r##"(let* ((project-functions
                     (make-hash-table))
                    (vendor-functions
                     (make-hash-table))
                    (project-data
                     (list
                      (make-hash-table)
                      project-functions
                      (make-hash-table)
                      ["/one.php"
                       "/two.php"]
                      "/project/"))
                    (vendor-data
                     (list
                      (make-hash-table)
                      vendor-functions
                      (make-hash-table)
                      ["/vendor.php"]
                      "/project/"))
                    calls)
               (puthash
                "one"
                t
                project-functions)
               (puthash
                "two"
                t
                project-functions)
               (puthash
                "vendor"
                t
                vendor-functions)
               (cl-letf
                   (((symbol-function
                      'ac-php-get-tags-file)
                     (lambda ()
                       '("/project/"
                         "/cache/tags.el"
                         "/cache/vendor.el")))
                    ((symbol-function
                      'ac-php-get-tags-data)
                     (lambda ()
                       project-data))
                    ((symbol-function
                      'ac-php-load-data)
                     (lambda (&rest _arguments)
                       vendor-data))
                    ((symbol-function
                      'file-attributes)
                     (lambda (_file)
                       '(nil nil nil nil nil
                             fixture-time)))
                    ((symbol-function
                      'format-time-string)
                     (lambda
                         (format time)
                       (push
                        (list
                         'time format time)
                        calls)
                       "2026-02-10 08:46:58"))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'message
                         arguments)
                        calls)
                       'messaged)))
                 (list
                  (call-interactively
                   'ac-php-show-cur-project-info)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (messaged ((time "%Y-%m-%d %H:%M:%S" fixture-time) (message "root dir           : %s\nconfig file        : %s%s\ntags file          : %s\ntags last gen time : %s\nfile count         : %s\ndefine count       : %s\nvendor file count  : %s\nvendor define count: %s\n" "/project/" "/project/" ".ac-php-conf.json" "/cache/tags.el" "2026-02-10 08:46:58" 2 2 1 1)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}
