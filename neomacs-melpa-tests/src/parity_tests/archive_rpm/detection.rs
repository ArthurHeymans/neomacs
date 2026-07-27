use expect_test::expect;

use super::{assert_archive_cpio_parity, assert_archive_rpm_parity};

#[test]
fn cpio_detector_recognizes_new_ascii_crc_and_trailing_zero_headers() {
    let elisp_form = r##"(mapcar
 (lambda (contents)
   (with-temp-buffer
     (set-buffer-multibyte nil)
     (insert contents)
     (goto-char (point-max))
     (list
      (archive-cpio-find-type)
      (point)
      (= (point-min) 1))))
 (list
  (concat
   "070701"
   (make-string 104 ?0))
  (concat
   "070702"
   (make-string 104 ?0))
  (make-string 128 0)))"##;
    let expect = expect!["OK ((cpio 1 t) (cpio 1 t) (cpio 1 t))"];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn cpio_detector_rejects_old_ascii_binary_truncated_and_prefixed_inputs() {
    let elisp_form = r##"(mapcar
 (lambda (contents)
   (with-temp-buffer
     (set-buffer-multibyte nil)
     (insert contents)
     (archive-cpio-find-type)))
 (list
  (concat "070707" (make-string 104 ?0))
  (concat "070701" (make-string 103 ?0))
  (concat "x070701" (make-string 104 ?0))
  ""
  "plain text"))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn cpio_detector_widens_narrowed_archive_and_leaves_point_at_buffer_start() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert
   (concat "070701" (make-string 104 ?0) "payload"))
  (narrow-to-region 20 40)
  (goto-char 30)
  (list
   (point-min)
   (point-max)
   (archive-cpio-find-type)
   (point)
   (point-min)
   (point-max)))"##;
    let expect = expect!["OK (20 40 cpio 1 1 118)"];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn rpm_detector_requires_the_complete_six_byte_lead_signature() {
    let elisp_form = r##"(mapcar
 (lambda (bytes)
   (with-temp-buffer
     (set-buffer-multibyte nil)
     (insert (apply #'unibyte-string bytes))
     (archive-rpm-find-type)))
 '((237 171 238 219 3 0)
   (237 171 238 219 3 0 255 1)
   (237 171 238 219 3)
   (237 171 238 219 4 0)
   (237 171 238 218 3 0)
   (0 237 171 238 219 3 0)))"##;
    let expect = expect!["OK (rpm rpm nil nil nil nil)"];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn rpm_detector_widens_and_rewinds_a_narrowed_binary_buffer() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert
   (apply #'unibyte-string
          '(237 171 238 219 3 0 10 20 30 40)))
  (narrow-to-region 7 11)
  (goto-char 9)
  (list
   (point-min)
   (point-max)
   (archive-rpm-find-type)
   (point)
   (point-min)
   (point-max)))"##;
    let expect = expect!["OK (7 11 rpm 1 1 11)"];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn archive_find_type_advice_selects_cpio_rpm_and_preserves_builtin_tar_detection() {
    let elisp_form = r##"(mapcar
 (lambda (contents)
   (with-temp-buffer
     (set-buffer-multibyte nil)
     (insert contents)
     (condition-case error-data
         (archive-find-type)
       (error
        (list (car error-data)
              (error-message-string
               error-data))))))
 (list
  (concat
   "070701" (make-string 104 ?0))
  (apply #'unibyte-string
         '(237 171 238 219 3 0))
  (concat
   (make-string 257 0)
   "ustar"
   (make-string 255 0))
  "ordinary text"))"##;
    let expect = expect![[
        r#"OK (cpio rpm (error "Buffer format not recognized") (error "Buffer format not recognized"))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn rpm_magic_mode_entry_matches_only_binary_version_three_leads() {
    let elisp_form = r##"(let ((entry
       (seq-find
        (lambda (candidate)
          (eq (cdr candidate) 'archive-mode))
        magic-mode-alist)))
  (list
   (car entry)
   (mapcar
    (lambda (bytes)
      (string-match-p
       (car entry)
       (apply #'unibyte-string bytes)))
    '((237 171 238 219 3 0)
      (237 171 238 219 3 0 99)
      (237 171 238 219 4 0)
      (0 237 171 238 219 3 0)))))"##;
    let expect = expect![[r#"OK ("��������\3\0" (0 0 nil 1))"#]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn automatic_mode_dispatch_prefers_rpm_magic_over_an_unrelated_filename() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (setq buffer-file-name
        (expand-file-name
         "release.payload"
         (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
  (insert
   (apply #'unibyte-string
          '(237 171 238 219 3 0)))
  (insert (make-string 128 0))
  (let (activated)
    (cl-letf
        (((symbol-function 'archive-mode)
          (lambda ()
            (setq activated
                  (list 'archive-mode
                        (point)
                        buffer-file-name)))))
      (set-auto-mode)
      activated)))"##;
    let expect = expect![[r#"OK (archive-mode 135 "[ORACLE-SANDBOX]/release.payload")"#]];
    assert_archive_rpm_parity(elisp_form, expect);
}
