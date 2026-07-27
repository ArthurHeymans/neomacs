use expect_test::expect;

use super::{assert_archive_cpio_parity, assert_archive_cpio_signal_parity};

#[test]
fn parses_every_supported_posix_file_kind_with_conventional_permissions() {
    let elisp_form = r##"(mapcar
 (lambda (entry)
   (list
    (car entry)
    (archive-cpio--parse-mode
     (+ (car entry) (cdr entry)))))
 '((#o140000 . #o755)
   (#o120000 . #o777)
   (#o100000 . #o644)
   (#o060000 . #o660)
   (#o040000 . #o750)
   (#o020000 . #o600)
   (#o010000 . #o644)))"##;
    let expect = expect![[
        r#"OK ((49152 "srwxr-xr-x") (40960 "lrwxrwxrwx") (32768 "-rw-r--r--") (24576 "brw-rw----") (16384 "drwxr-x---") (8192 "crw-------") (4096 "prw-r--r--"))"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn parses_owner_group_and_other_permission_bits_independently() {
    let elisp_form = r##"(mapcar
 (lambda (permissions)
   (list permissions
         (archive-cpio--parse-mode
          (+ #o100000 permissions))))
 '(#o000 #o001 #o002 #o004 #o010 #o020 #o040
   #o100 #o200 #o400 #o111 #o222 #o444 #o751
   #o640 #o777))"##;
    let expect = expect![[
        r#"OK ((0 "----------") (1 "---------x") (2 "--------w-") (4 "-------r--") (8 "------x---") (16 "-----w----") (32 "----r-----") (64 "---x------") (128 "--w-------") (256 "-r--------") (73 "---x--x--x") (146 "--w--w--w-") (292 "-r--r--r--") (489 "-rwxr-x--x") (416 "-rw-r-----") (511 "-rwxrwxrwx"))"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn setuid_and_setgid_bits_render_lowercase_when_execute_is_present() {
    let elisp_form = r##"(mapcar
 (lambda (permissions)
   (archive-cpio--parse-mode
    (+ #o100000 permissions)))
 '(#o4000 #o4100 #o4700 #o4755
   #o2000 #o2010 #o2070 #o2755
   #o6000 #o6110 #o6755))"##;
    let expect = expect![[
        r#"OK ("---S------" "---s------" "-rws------" "-rwsr-xr-x" "------S---" "------s---" "----rws---" "-rwxr-sr-x" "---S--S---" "---s--s---" "-rwsr-sr-x")"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn setuid_and_setgid_bits_render_uppercase_without_execute_permission() {
    let elisp_form = r##"(mapcar
 (lambda (permissions)
   (archive-cpio--parse-mode
    (+ #o100000 permissions)))
 '(#o4000 #o4600 #o4644 #o2000 #o2060 #o2644
   #o6000 #o6666))"##;
    let expect = expect![[
        r#"OK ("---S------" "-rwS------" "-rwSr--r--" "------S---" "----rwS---" "-rw-r-Sr--" "---S--S---" "-rwSrwSrw-")"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn sticky_bit_is_ignored_exactly_like_upstream_for_directories_and_files() {
    let elisp_form = r##"(mapcar
 (lambda (mode)
   (list mode
         (archive-cpio--parse-mode mode)))
 '(#o041777 #o041755 #o041000
   #o101777 #o101755 #o101000))"##;
    let expect = expect![[
        r#"OK ((17407 "drwxrwxrwx") (17389 "drwxr-xr-x") (16896 "d---------") (33791 "-rwxrwxrwx") (33773 "-rwxr-xr-x") (33280 "----------"))"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn high_unrelated_bits_do_not_change_file_kind_or_permissions() {
    let elisp_form = r##"(mapcar
 (lambda (mode)
   (archive-cpio--parse-mode mode))
 (list
  #o100644
  (+ (ash 1 24) #o100644)
  (+ (ash 1 30) #o040755)
  (+ (ash 7 20) #o120777)))"##;
    let expect = expect![[r#"OK ("-rw-r--r--" "-rw-r--r--" "drwxr-xr-x" "lrwxrwxrwx")"#]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn missing_file_kind_signals_the_exact_unknown_mode_contract() {
    let elisp_form = r##"(archive-cpio--parse-mode #o755)"##;
    let expect = expect![[r#"ERR (error "Unknown mode 493")"#]];
    assert_archive_cpio_signal_parity(elisp_form, expect);
}

#[test]
fn unsupported_whiteout_and_mixed_kind_bits_signal_instead_of_guessing() {
    let elisp_form = r##"(archive-cpio--parse-mode #o160644)"##;
    let expect = expect![[r#"ERR (error "Unknown mode 57764")"#]];
    assert_archive_cpio_signal_parity(elisp_form, expect);
}
