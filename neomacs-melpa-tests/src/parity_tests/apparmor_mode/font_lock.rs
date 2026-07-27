use expect_test::expect;

use super::assert_apparmor_mode_parity;

#[test]
fn apparmor_mode_fontifies_all_primary_policy_keywords() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (text target)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (goto-char (point-min))
             (search-forward target)
             (list target
                   (match-beginning 0)
                   (get-text-property
                    (match-beginning 0) 'face)))))
         (mapcar
          (lambda (case) (apply #'face-at case))
          '(("capability dac_override," "capability")
            ("network inet dgram," "network")
            ("dbus (send) bus=session," "dbus")
            ("file r /etc/passwd," "file")
            ("deny /etc/shadow r," "deny")
            ("userns," "userns"))))"##;
    let expect = expect![[
        r#"OK (("capability" 1 font-lock-keyword-face) ("network" 1 font-lock-keyword-face) ("dbus" 1 font-lock-keyword-face) ("file" 1 font-lock-keyword-face) ("deny" 1 font-lock-keyword-face) ("userns" 1 font-lock-keyword-face))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_include_directives_paths_and_if_exists() {
    let elisp_form = r##"(cl-labels
         ((faces
           (text targets)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (mapcar
              (lambda (target)
                (goto-char (point-min))
                (search-forward target)
                (list target
                      (match-beginning 0)
                      (get-text-property
                       (match-beginning 0) 'face)))
              targets))))
         (list
          (faces "include <abstractions/base>"
                 '("include" "abstractions/base"))
          (faces "include <tunables/global>"
                 '("include" "tunables/global"))
          (faces "include if exists <local/dig>"
                 '("include" "local/dig"))))"##;
    let expect = expect![[
        r#"OK ((("include" 1 font-lock-preprocessor-face) ("abstractions/base" 10 font-lock-string-face)) (("include" 1 font-lock-preprocessor-face) ("tunables/global" 10 font-lock-string-face)) (("include" 1 font-lock-preprocessor-face) ("local/dig" 20 font-lock-string-face)))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_abi_directive_and_version_path() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert "abi <abi/5.0>,")
         (font-lock-ensure)
         (mapcar
          (lambda (target)
            (goto-char (point-min))
            (search-forward target)
            (list target
                  (match-beginning 0)
                  (get-text-property
                   (match-beginning 0) 'face)))
          '("abi" "abi/5.0" ",")))"##;
    let expect = expect![[
        r#"OK (("abi" 1 font-lock-preprocessor-face) ("abi/5.0" 6 font-lock-string-face) ("," 14 font-lock-builtin-face))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_variable_references_but_preserves_comments() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (text target)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (goto-char (point-min))
             (search-forward target)
             (list target
                   (get-text-property
                    (match-beginning 0) 'face)
                   (nth 4
                        (syntax-ppss
                         (match-beginning 0)))))))
         (mapcar
          (lambda (case) (apply #'face-at case))
          '(("/usr/lib/@{multiarch}/gconv/*.so mr,"
             "@{multiarch}")
            ("owner @{HOME}/.config r," "@{HOME}")
            ("@{etc_ro}/ld.so.cache mr," "@{etc_ro}")
            ("@{arg1} r," "@{arg1}")
            ("@{a} r," "@{a}")
            ("# allow @{HOME}" "@{HOME}"))))"##;
    let expect = expect![[
        r#"OK (("@{multiarch}" font-lock-variable-name-face nil) ("@{HOME}" font-lock-variable-name-face nil) ("@{etc_ro}" font-lock-variable-name-face nil) ("@{arg1}" font-lock-variable-name-face nil) ("@{a}" font-lock-variable-name-face nil) ("@{HOME}" font-lock-comment-face t))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_capability_names_as_types() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (name)
           (with-temp-buffer
             (apparmor-mode)
             (insert "capability " name ",")
             (font-lock-ensure)
             (goto-char (point-min))
             (search-forward name)
             (list name
                   (get-text-property
                    (match-beginning 0) 'face)))))
         (mapcar #'face-at
                 '("dac_override" "dac_read_search"
                   "sys_admin" "net_bind_service")))"##;
    let expect = expect![[
        r#"OK (("dac_override" font-lock-type-face) ("dac_read_search" font-lock-type-face) ("sys_admin" font-lock-type-face) ("net_bind_service" font-lock-type-face))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_profile_names_attachments_and_flags() {
    let elisp_form = r##"(cl-labels
         ((faces
           (text targets)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (mapcar
              (lambda (target)
                (goto-char (point-min))
                (search-forward target)
                (list target
                      (get-text-property
                       (match-beginning 0) 'face)))
              targets))))
         (list
          (faces "profile dig /usr/bin/dig {"
                 '("profile" "dig" "/usr/bin/dig"))
          (faces
           "profile alsamixer /{usr,}/bin/alsamixer {"
           '("alsamixer" "{" "}"))
          (faces
           "profile busybox /usr/bin/busybox flags=(unconfined) {"
           '("busybox" "/usr/bin/busybox" "flags"
             "unconfined"))))"##;
    let expect = expect![[
        r#"OK ((("profile" font-lock-keyword-face) ("dig" font-lock-function-name-face) ("/usr/bin/dig" font-lock-variable-name-face)) (("alsamixer" font-lock-function-name-face) ("{" font-lock-builtin-face) ("}" font-lock-builtin-face)) (("busybox" font-lock-function-name-face) ("/usr/bin/busybox" font-lock-variable-name-face) ("flags" nil) ("unconfined" nil)))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_prefix_and_suffix_file_permissions() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (text target)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (goto-char (point-min))
             (search-forward target)
             (list text target
                   (get-text-property
                    (match-beginning 0) 'face)))))
         (mapcar
          (lambda (case) (apply #'face-at case))
          '(("file r /etc/passwd," "r")
            ("file mr /usr/bin/dig," "mr")
            ("/usr/bin/dig mr," "mr")
            ("/etc/passwd r," "r")
            ("owner file rw @{HOME}/config," "rw")
            ("deny /etc/shadow r," "r"))))"##;
    let expect = expect![[
        r#"OK (("file r /etc/passwd," "r" font-lock-constant-face) ("file mr /usr/bin/dig," "mr" font-lock-constant-face) ("/usr/bin/dig mr," "mr" font-lock-constant-face) ("/etc/passwd r," "r" font-lock-constant-face) ("owner file rw @{HOME}/config," "rw" font-lock-constant-face) ("deny /etc/shadow r," "r" font-lock-constant-face))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_transition_and_statement_operators() {
    let elisp_form = r##"(cl-labels
         ((faces
           (text targets)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (mapcar
              (lambda (target)
                (goto-char (point-min))
                (search-forward target)
                (list target
                      (get-text-property
                       (match-beginning 0) 'face)))
              targets))))
         (list
          (faces "file Cx /path -> child,"
                 '("Cx" "->" "child" ","))
          (faces "@{HOME}+=/srv/home" '("@{HOME}" "+="))
          (faces "set rlimit cpu <= 10," '("rlimit" "<=" ","))))"##;
    let expect = expect![[
        r#"OK ((("Cx" font-lock-constant-face) ("->" font-lock-builtin-face) ("child" font-lock-function-name-face) ("," font-lock-builtin-face)) (("@{HOME}" font-lock-variable-name-face) ("+=" font-lock-builtin-face)) (("rlimit" font-lock-keyword-face) ("<=" nil) ("," font-lock-builtin-face)))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_quoted_paths_permissions_and_variables() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (text target)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (goto-char (point-min))
             (search-forward target)
             (list target
                   (match-beginning 0)
                   (get-text-property
                    (match-beginning 0) 'face)))))
         (mapcar
          (lambda (case) (apply #'face-at case))
          '(("\"@{HOME}/My Documents/file\" rw," "@{HOME}")
            ("\"@{HOME}/My Documents/file\" rw," "rw")
            ("file rw \"@{HOME}/My Documents/file\"," "@{HOME}")
            ("file rw \"@{HOME}/My Documents/file\"," "rw")
            ("file \"@{HOME}/My Documents/file\" rw," "@{HOME}")
            ("file \"@{HOME}/My Documents/file\" rw," "rw")
            ("\"/path/with spaces\" r," "r")
            ("file Cx \"/path/with spaces\" -> child," "Cx"))))"##;
    let expect = expect![[
        r#"OK (("@{HOME}" 2 font-lock-variable-name-face) ("rw" 29 font-lock-constant-face) ("@{HOME}" 10 font-lock-variable-name-face) ("rw" 6 font-lock-constant-face) ("@{HOME}" 7 font-lock-variable-name-face) ("rw" 34 font-lock-constant-face) ("r" 21 font-lock-constant-face) ("Cx" 6 font-lock-constant-face))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_distinguishes_embedded_hashes_from_real_comments() {
    let elisp_form = r##"(cl-labels
         ((faces
           (text targets)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (mapcar
              (lambda (target)
                (goto-char (point-min))
                (search-forward target)
                (list target
                      (get-text-property
                       (match-beginning 0) 'face)
                      (nth 4
                           (syntax-ppss
                            (match-beginning 0)))))
              targets))))
         (list
          (faces "/usr/lib/libfoo.so.1# mr,"
                 '("#" "mr"))
          (faces "/usr/lib/libfoo.so.1#2 mr,"
                 '("#" "2" "mr"))
          (faces "file mr /usr/lib/libfoo.so.1#2,"
                 '("mr" "#" "2"))
          (faces "/usr/lib/libfoo.so.1 mr,#comment"
                 '("#" "comment"))))"##;
    let expect = expect![[
        r##"OK ((("#" nil nil) ("mr" font-lock-constant-face nil)) (("#" nil nil) ("2" nil nil) ("mr" font-lock-constant-face nil)) (("mr" font-lock-constant-face nil) ("#" nil nil) ("2" nil nil)) (("#" font-lock-comment-delimiter-face nil) ("comment" font-lock-comment-face t)))"##
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_fontifies_aare_globs_but_not_variable_braces() {
    let elisp_form = r##"(cl-labels
         ((face-at
           (text target occurrence)
           (with-temp-buffer
             (apparmor-mode)
             (insert text)
             (font-lock-ensure)
             (goto-char (point-min))
             (dotimes (_ occurrence)
               (search-forward target))
             (list text target occurrence
                   (get-text-property
                    (match-beginning 0) 'face)))))
         (mapcar
          (lambda (case) (apply #'face-at case))
          '(("/usr/lib/*/file mr," "*" 1)
            ("/usr/bin/** mr," "**" 1)
            ("/dev/{urandom,null} r," "{" 1)
            ("/dev/{urandom,null} r," "}" 1)
            ("file r /usr/lib/*.so," "*" 1)
            ("profile alsamixer /{usr,}/bin/alsamixer {" "{" 1)
            ("@{HOME}/*.so mr," "{" 1)
            ("@{HOME}/*.so mr," "}" 1)
            ("@{HOME}/*.so mr," "*" 1)
            ("/usr/lib/libfoo.so.? mr," "?" 1))))"##;
    let expect = expect![[
        r#"OK (("/usr/lib/*/file mr," "*" 1 font-lock-regexp-grouping-construct) ("/usr/bin/** mr," "**" 1 font-lock-regexp-grouping-construct) ("/dev/{urandom,null} r," "{" 1 font-lock-builtin-face) ("/dev/{urandom,null} r," "}" 1 font-lock-builtin-face) ("file r /usr/lib/*.so," "*" 1 font-lock-regexp-grouping-construct) ("profile alsamixer /{usr,}/bin/alsamixer {" "{" 1 font-lock-builtin-face) ("@{HOME}/*.so mr," "{" 1 font-lock-variable-name-face) ("@{HOME}/*.so mr," "}" 1 font-lock-variable-name-face) ("@{HOME}/*.so mr," "*" 1 font-lock-regexp-grouping-construct) ("/usr/lib/libfoo.so.? mr," "?" 1 font-lock-regexp-grouping-construct))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}
