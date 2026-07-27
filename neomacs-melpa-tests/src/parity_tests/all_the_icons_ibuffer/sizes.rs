use expect_test::expect;

use super::assert_all_the_icons_ibuffer_parity;

#[test]
fn file_size_parser_converts_real_iec_size_labels_across_all_supported_prefixes() {
    let elisp_form = r##"(mapcar
 (lambda (label)
   (list label
         (all-the-icons-ibuffer--file-size-human-readable-to-bytes label)))
 '("0" "999" "1k" "1.5k" "2M" "3.25G" "4T" "5P" "6E" "7Z" "8Y"))"##;
    let expect = expect![[
        r#"OK (("0" 0.0) ("999" 999.0) ("1k" 1024.0) ("1.5k" 1536.0) ("2M" 2097152.0) ("3.25G" 3489660928.0) ("4T" 4398046511104.0) ("5P" 5.62949953421312e+15) ("6E" 6.917529027641082e+18) ("7Z" 8.264141345021879e+21) ("8Y" 9.671406556917033e+24))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn file_size_parser_converts_si_labels_and_preserves_fractional_results() {
    let elisp_form = r##"(mapcar
 (lambda (label)
   (list label
         (all-the-icons-ibuffer--file-size-human-readable-to-bytes
          label 'si)))
 '("1kB" "1.25MB" "9.876GB" "0.5TB" "-2kB" "42"))"##;
    let expect = expect![[
        r#"OK (("1kB" 1000.0) ("1.25MB" 1250000.0) ("9.876GB" 9876000000.0) ("0.5TB" 500000000000.0) ("-2kB" -2000.0) ("42" 42.0))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn file_size_parser_honors_ambient_case_folding_for_practical_unit_labels() {
    let elisp_form = r##"(mapcar
 (lambda (fold)
   (let ((case-fold-search fold))
     (list
      fold
      (mapcar
       (lambda (label)
         (list
          label
          (all-the-icons-ibuffer--file-size-human-readable-to-bytes
           label)))
       '("1.2k" "1.2K" "1.2M" "1.2m"
         "15 kB" "2.5 MiB" "3GiB")))))
 '(t nil))"##;
    let expect = expect![[
        r#"OK ((t (("1.2k" 1228.8) ("1.2K" 1228.8) ("1.2M" 1258291.2) ("1.2m" 1258291.2) ("15 kB" 15360.0) ("2.5 MiB" 2621440.0) ("3GiB" 3221225472.0))) (nil (("1.2k" 1228.8) ("1.2K" 1.2) ("1.2M" 1258291.2) ("1.2m" 1.2) ("15 kB" 15360.0) ("2.5 MiB" 2621440.0) ("3GiB" 3221225472.0))))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn size_column_renders_real_buffer_sizes_in_human_and_exact_byte_modes() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "ati-size-workload")))
  (unwind-protect
      (mapcar
       (lambda (size)
         (with-current-buffer buffer
           (erase-buffer)
           (insert (make-string size ?x)))
         (mapcar
          (lambda (human)
            (let ((all-the-icons-ibuffer-human-readable-size human))
              (with-temp-buffer
                (funcall (ibuffer-compile-format '(size-h)) buffer ?\s)
                (let ((rendered (buffer-string)))
                  (list human rendered
                        (get-text-property
                         0 'font-lock-face rendered))))))
          '(t nil)))
       '(0 1 999 1024 1536 1048576))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (((t #("0" 0 1 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face) (nil #("0" 0 1 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face)) ((t #("1" 0 1 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face) (nil #("1" 0 1 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face)) ((t #("999" 0 3 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face) (nil #("999" 0 3 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face)) ((t #("1k" 0 2 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face) (nil #("1024" 0 4 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face)) ((t #("1.5k" 0 4 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face) (nil #("1536" 0 4 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face)) ((t #("1M" 0 2 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face) (nil #("1048576" 0 7 (font-lock-face all-the-icons-ibuffer-size-face)) all-the-icons-ibuffer-size-face)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn size_column_honors_ibuffer_width_and_right_alignment_in_real_rows() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "ati-size-alignment")))
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (insert (make-string 1536 ?x)))
        (mapcar
         (lambda (format)
           (with-temp-buffer
             (funcall (ibuffer-compile-format format) buffer ?\s)
             (list (buffer-string)
                   (length (buffer-string))
                   (get-text-property
                    (1- (point-max))
                    'font-lock-face
                    (buffer-string)))))
         '(((size-h 9 -1 :right))
           ((size-h 9 -1 :left))
           ("[" (size-h 12 12 :center) "]"))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((#("     1.5k" 5 9 (font-lock-face all-the-icons-ibuffer-size-face)) 9 nil) (#("1.5k     " 0 4 (font-lock-face all-the-icons-ibuffer-size-face)) 9 nil) (#("[    1.5k    ]" 5 9 (font-lock-face all-the-icons-ibuffer-size-face)) 14 nil))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn size_summarizer_totals_mixed_realistic_labels_in_human_and_exact_modes() {
    let elisp_form = r##"(let ((summarizer
       (get 'ibuffer-make-column-size-h 'ibuffer-column-summarizer))
      (labels '("512" "1k" "1.5M" "2G" "0" "3.25k")))
  (mapcar
   (lambda (human)
     (let ((all-the-icons-ibuffer-human-readable-size human))
       (list human
             (funcall summarizer (copy-sequence labels)))))
   '(t nil)))"##;
    let expect = expect![[r#"OK ((t "2G") (nil "2149061376"))"#]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}
