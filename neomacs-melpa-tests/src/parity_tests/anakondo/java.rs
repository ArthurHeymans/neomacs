use expect_test::expect;

use super::assert_anakondo_parity;

#[test]
fn class_map_preserves_name_member_collection_and_mutable_hash_identity() {
    let elisp_form = r##"(let* ((members
                            (list
                             'first-method
                             'second-field))
                           (class-map
                            (anakondo--make-class-map
                             "com.acme.Tools"
                             members)))
                      (puthash :loaded t class-map)
                      (list
                       (hash-table-p class-map)
                       (gethash :name class-map)
                       (eq
                        (gethash
                         :methods-and-fields
                         class-map)
                        members)
                       (gethash :loaded class-map)
                       (hash-table-count class-map)))"##;
    let expect = expect![[r#"OK (t "com.acme.Tools" t t 3)"#]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn jar_analysis_filters_non_jars_inner_classes_and_clojure_init_classes() {
    let elisp_form = r##"(let (commands)
                      (cl-letf
                          (((symbol-function 'shell-command)
                            (lambda
                                (command
                                 &optional
                                 _output-buffer
                                 _error-buffer)
                              (push command commands)
                              (insert
                               (if
                                   (string-match-p
                                    "first.jar"
                                    command)
                                   "META-INF/MANIFEST.MF\nclojure/lang/AFn.class\nclojure/lang/AFn$1.class\nfoo/bar__init.class\nRoot.class\n"
                                 "com/acme/Tools.class\ncom/acme/Tools$Nested.class\ncom/acme/Util.class\n"))
                              0)))
                        (list
                         (anakondo--jar-analize-sync
                          '("lib/first.jar"
                            "target/classes"
                            "vendor with space/second.jar"))
                         (nreverse commands))))"##;
    let expect = expect![[
        r#"OK (("clojure.lang.AFn" "com.acme.Util" "com.acme.Tools") ("jar tf 'lib/first.jar'" "jar tf 'vendor with space/second.jar'"))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn javap_analysis_extracts_only_public_static_methods_and_fields_with_signatures() {
    let elisp_form = r##"(let (commands)
                      (cl-letf
                          (((symbol-function 'shell-command)
                            (lambda
                                (command
                                 &optional
                                 _output-buffer
                                 _error-buffer)
                              (push command commands)
                              (insert
                               "Compiled from \"Tools.java\"\n\
public class com.acme.Tools {\n\
  public static final java.lang.String VERSION;\n\
  public static int max(int, int);\n\
  public static void reset();\n\
  public int instance(int);\n\
  public com.acme.Tools();\n\
}\n")
                              0)))
                        (let* ((class-map
                                (anakondo--java-analyze-class-map
                                 "target/classes:vendor/a.jar"
                                 "com.acme.Tools"))
                               (members
                                (gethash
                                 :methods-and-fields
                                 class-map)))
                          (list
                           (gethash :name class-map)
                           (mapcar
                            (lambda (member)
                              (list
                               (gethash
                                :return-type member)
                               (gethash :name member)
                               (gethash
                                :signature member)
                               (gethash
                                :method? member)))
                            members)
                           (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK ("com.acme.Tools" (("void" "reset" "()" t) ("int" "max" "(int, int)" t) ("java.lang.String" "VERSION" nil nil)) ("javap -cp 'target/classes:vendor/a.jar' -public 'com.acme.Tools'"))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn java_boot_classpath_parser_reads_multiline_property_until_next_setting() {
    let elisp_form = r##"(let (commands)
                      (cl-letf
                          (((symbol-function 'shell-command)
                            (lambda
                                (command
                                 &optional
                                 _output-buffer
                                 _error-buffer)
                              (push command commands)
                              (insert
                               "Property settings:\n\
    java.home = /jdk\n\
    sun.boot.class.path = /boot/one.jar\n\
        /boot/two.jar\n\
        /boot/three.jar\n\
    java.class.path = /application/classes\n")
                              0)))
                        (list
                         (anakondo--get-java-boot-classpath-list)
                         (nreverse commands))))"##;
    let expect = expect![[
        r#"OK (("/boot/three.jar" "/boot/two.jar" "/boot/one.jar") ("java -XshowSettings:properties -version"))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn java_analysis_classpath_combines_tools_deps_and_boot_entries_in_both_formats() {
    let elisp_form = r##"(cl-letf
                      (((symbol-function
                         'anakondo--get-project-path)
                        (lambda ()
                          "src:target/classes:vendor/a.jar\n"))
                       ((symbol-function
                         'anakondo--get-java-boot-classpath-list)
                        (lambda ()
                          '("/boot/rt.jar"
                            "/boot/jsse.jar"))))
                      (list
                       (anakondo--get-java-analysis-classpath
                        'list)
                       (anakondo--get-java-analysis-classpath
                        'cp)
                       (anakondo--get-java-analysis-classpath
                        'unknown)))"##;
    let expect = expect![[
        r#"OK (("src" "target/classes" "vendor/a.jar" "/boot/rt.jar" "/boot/jsse.jar") "src:target/classes:vendor/a.jar:/boot/rt.jar:/boot/jsse.jar" nil)"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn java_project_analysis_populates_every_discovered_class_as_lazy_without_loading_members() {
    let elisp_form = r##"(let ((cache
                           (make-hash-table))
                          events)
                      (cl-letf
                          (((symbol-function
                             'anakondo--get-java-analysis-classpath)
                            (lambda (as)
                              (push
                               (list 'classpath as)
                               events)
                              '("one.jar"
                                "two.jar")))
                           ((symbol-function
                             'anakondo--jar-analize-sync)
                            (lambda (classpath)
                              (push
                               (list 'jars classpath)
                               events)
                              '("com.acme.Tools"
                                "clojure.lang.RT"))))
                        (list
                         (anakondo--java-project-analyse-sync
                          cache)
                         (hash-table-count cache)
                         (mapcar
                          (lambda (class)
                            (let ((class-map
                                   (gethash
                                    (anakondo--string->keyword
                                     class)
                                    cache)))
                              (list
                               (gethash :name class-map)
                               (gethash
                                :methods-and-fields
                                class-map))))
                          '("com.acme.Tools"
                            "clojure.lang.RT"))
                         (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil 2 (("com.acme.Tools" lazy) ("clojure.lang.RT" lazy)) ((classpath list) (jars ("one.jar" "two.jar"))))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}
