use expect_test::expect;

use super::assert_abgaben_parity;

#[test]
fn abgaben_maybe_unzip_dispatches_zip_tar_rar_and_plain_files_exactly() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'call-process)
                     (lambda (&rest arguments)
                       (push
                        (list
                         default-directory
                         arguments)
                        events)
                       0)))
                 (list
                  (abgaben--maybe-unzip
                   "/submissions/group/week"
                   "work.zip")
                  (abgaben--maybe-unzip
                   "/submissions/group/week/"
                   "bundle.tar.gz")
                  (abgaben--maybe-unzip
                   "/submissions/group/week/"
                   "legacy.rar")
                  (abgaben--maybe-unzip
                   "/submissions/group/week/"
                   "notes.pdf")
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("work" "bundle" "legacy" "notes.pdf" (("/submissions/group/week" ("mkdir" nil nil nil "work")) ("/submissions/group/week" ("unzip" nil nil nil "work.zip" "-d" "work")) ("/submissions/group/week/" ("mkdir" nil nil nil "bundle")) ("/submissions/group/week/" ("tar" nil nil nil "-xaf" "bundle.tar.gz" "-C" "bundle")) ("/submissions/group/week/" ("mkdir" nil nil nil "legacy")) ("/submissions/group/week/" ("unrar" nil nil nil "x" "legacy.rar" "legacy"))))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_maybe_unzip_case_insensitive_detection_retains_uppercase_suffix_quirk() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'call-process)
                     (lambda (&rest arguments)
                       (push arguments events)
                       0)))
                 (list
                  (abgaben--maybe-unzip "/d/" "UPPER.ZIP")
                  (abgaben--maybe-unzip "/d/" "UPPER.TAR.GZ")
                  (abgaben--maybe-unzip "/d/" "UPPER.RAR")
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("UPPER.ZIP" "UPPER.TAR.GZ" "UPPER.RAR" (("mkdir" nil nil nil "UPPER.ZIP") ("unzip" nil nil nil "UPPER.ZIP" "-d" "UPPER.ZIP") ("mkdir" nil nil nil "UPPER.TAR.GZ") ("tar" nil nil nil "-xaf" "UPPER.TAR.GZ" "-C" "UPPER.TAR.GZ") ("mkdir" nil nil nil "UPPER.RAR") ("unrar" nil nil nil "x" "UPPER.RAR" "UPPER.RAR")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_maybe_unzip_returns_subdirectory_even_when_processes_fail() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'call-process)
                     (lambda (&rest arguments)
                       (push arguments events)
                       127)))
                 (list
                  (abgaben--maybe-unzip "/d/" "broken.zip")
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("broken" (("mkdir" nil nil nil "broken") ("unzip" nil nil nil "broken.zip" "-d" "broken")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}
