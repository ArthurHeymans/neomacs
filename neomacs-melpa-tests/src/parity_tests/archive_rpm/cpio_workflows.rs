use expect_test::expect;

use super::{assert_archive_cpio_parity, assert_archive_cpio_signal_parity};

#[test]
fn extracts_text_binary_empty_and_nested_files_from_one_practical_archive() {
    let elisp_form = r##"(cl-labels
    ((pad4
      (size)
      (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (ino mode uid gid mtime name data)
      (let* ((bytes (string-as-unibyte data))
             (name-field
              (string-as-unibyte (concat name "\0")))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino mode uid gid 1 mtime (length bytes)
               0 0 0 0 (length name-field) 0)))
        (concat
         header name-field
         (pad4 (+ 110 (length name-field)))
         bytes (pad4 (length bytes)))))
     (trailer
      ()
      (entry 0 0 0 0 0 "TRAILER!!!" "")))
  (let* ((archive
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (insert
             (entry 1 #o100644 1000 1000 1700000000
                    "etc/app.conf"
                    "port=8080\nmode=prod\n")
             (entry 2 #o100600 0 0 1700000001
                    "var/lib/app/state.bin"
                    (unibyte-string 0 1 2 127 128 254 255))
             (entry 3 #o100644 1000 1000 1700000002
                    "empty" "")
             (trailer))
            (buffer-string)))
         (source (generate-new-buffer " *cpio practical source*"))
         results)
    (unwind-protect
        (progn
          (with-current-buffer source
            (set-buffer-multibyte nil)
            (insert archive)
            (goto-char (point-min)))
          (dolist (name
                   '("etc/app.conf"
                     "var/lib/app/state.bin"
                     "empty"
                     "missing"))
            (with-temp-buffer
              (set-buffer-multibyte nil)
              (let ((found
                     (archive-cpio-extract-from-buffer
                      name source (current-buffer))))
                (push
                 (list
                  name found
                  (string-to-list (buffer-string))
                  (secure-hash 'sha256 (current-buffer)))
                 results))))
          (nreverse results))
      (kill-buffer source))))"##;
    let expect = expect![[
        r#"OK (("etc/app.conf" t (112 111 114 116 61 56 48 56 48 10 109 111 100 101 61 112 114 111 100 10) "5b33e6a296131790110a6d3d6274889bc21469e2b72b0328da90ec237bc1f8e6") ("var/lib/app/state.bin" t (0 1 2 127 128 254 255) "7bb6463b30f9e301fed333cdf8960ca9497b602ccd8eeb46ae42693fdea15a4d") ("empty" t nil "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855") ("missing" nil nil "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"))"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn extraction_inserts_at_destination_point_without_erasing_existing_content() {
    let elisp_form = r##"(cl-labels
    ((pad4
      (size)
      (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (name data)
      (let* ((bytes (string-as-unibyte data))
             (name-field
              (string-as-unibyte (concat name "\0")))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               1 #o100644 42 84 1 0 (length bytes)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                bytes (pad4 (length bytes))))))
  (let ((source
         (generate-new-buffer " *cpio insertion source*")))
    (unwind-protect
        (progn
          (with-current-buffer source
            (set-buffer-multibyte nil)
            (insert
             (entry "report.txt"
                    "inserted payload"))
            (goto-char (point-min)))
          (with-temp-buffer
            (insert "before||after")
            (goto-char 8)
            (let ((result
                   (archive-cpio-extract-from-buffer
                    "report.txt" source (current-buffer))))
              (list result
                    (point)
                    (buffer-string)))))
      (kill-buffer source))))"##;
    let expect = expect![[r#"OK (t 24 "before|inserted payload|after")"#]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn duplicate_names_extract_the_first_archive_member_like_command_line_cpio() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (ino data)
      (let* ((name-field "duplicate.txt\0")
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino #o100644 0 0 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data))))))
  (let ((source
         (generate-new-buffer " *cpio duplicate source*")))
    (unwind-protect
        (progn
          (with-current-buffer source
            (set-buffer-multibyte nil)
            (insert (entry 1 "first")
                    (entry 2 "second"))
            (goto-char (point-min)))
          (with-temp-buffer
            (archive-cpio-extract-from-buffer
             "duplicate.txt" source (current-buffer))
            (buffer-string)))
      (kill-buffer source))))"##;
    let expect = expect![[r#"OK "first""#]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn extraction_widens_archive_mode_style_narrowing_and_restores_it_afterward() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (name data)
      (let* ((name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               77 #o100640 501 20 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data))))))
  (let ((source
         (generate-new-buffer " *cpio narrowed source*")))
    (unwind-protect
        (progn
          (with-current-buffer source
            (set-buffer-multibyte nil)
            (insert "visible summary\n\n"
                    (entry "secret.txt" "classified"))
            (narrow-to-region
             (point-min) (+ (point-min) 15)))
          (with-temp-buffer
            (let ((found
                   (archive-cpio-extract-from-buffer
                    "secret.txt" source (current-buffer))))
              (list
               found
               (buffer-string)
               (with-current-buffer source
                 (list (buffer-narrowed-p)
                       (point-min)
                       (point-max)))))))
      (kill-buffer source))))"##;
    let expect = expect![[r#"OK (t "classified" (t 1 16))"#]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn summarizes_regular_directory_symlink_and_fifo_members_with_exact_descriptors() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (ino mode uid gid name data)
      (let* ((bytes (string-as-unibyte data))
             (name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino mode uid gid 1 1700000000 (length bytes)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                bytes (pad4 (length bytes)))))
     (trailer () (entry 0 0 0 0 "TRAILER!!!" "")))
  (let ((display
         (generate-new-buffer " *cpio listing*")))
    (unwind-protect
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert
           (entry 1 #o040755 0 0 "usr/" "")
           (entry 2 #o100640 1000 100 "usr/report.txt"
                  "quarterly results\n")
           (entry 3 #o120777 1000 100 "latest"
                  "usr/report.txt")
           (entry 4 #o010644 42 43 "events.pipe" "")
           (trailer))
          (let ((files
                 (archive-cpio-summarize display)))
            (list
             (with-current-buffer display
               (buffer-string))
             (mapcar
              (lambda (file)
                (list
                 (archive--file-desc-ext-file-name file)
                 (archive--file-desc-int-file-name file)
                 (archive--file-desc-mode file)
                 (archive--file-desc-size file)
                 (archive--file-desc-pos file)))
              (append files nil)))))
      (kill-buffer display))))"##;
    let expect = expect![[
        r#"OK (#("M   Filemode   Length        UID/GID        File\n- ---------- -------- ---------- ---------- -----\n  drwxr-xr-x        0          0/0          usr/\n  -rw-r-----       18       1000/100        usr/report.txt\n  lrwxrwxrwx       14       1000/100        latest -> usr/report.txt\n  prw-r--r--        0         42/43         events.pipe\n" 143 147 (mouse-face highlight help-echo #1="mouse-2: extract this file into a buffer") 192 206 (mouse-face highlight help-echo #1#) 251 275 (mouse-face highlight help-echo #1#) 320 331 (mouse-face highlight help-echo #1#)) (("usr/" "usr/" 16877 0 117) ("usr/report.txt" "usr/report.txt" 33184 18 245) ("latest" "latest" 41471 14 385) ("events.pipe" "events.pipe" 4516 0 525)))"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn summarize_into_current_buffer_replaces_binary_with_a_human_listing() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (name data)
      (let* ((name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               9 #o100644 7 8 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data))))))
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert (entry "notes/today.md"
                   "# Today\n\nShip it.\n"))
    (let ((files (archive-cpio-summarize)))
      (list
       (buffer-string)
       (length files)
       (archive--file-desc-ext-file-name
        (aref files 0))
       (archive--file-desc-size
        (aref files 0))))))"##;
    let expect = expect![[
        r#"OK (#("M   Filemode   Length        UID/GID        File\n- ---------- -------- ---------- ---------- -----\n  -rw-r--r--       18          7/8          notes/today.md\n07070100000009000081a40000000700000008000000010000000000000012000000000000000000000000000000000000000f00000000notes/today.md\0\0\0\0# Today\n\nShip it.\n\0\0" 143 157 (mouse-face highlight help-echo "mouse-2: extract this file into a buffer")) 1 "notes/today.md" 18)"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn trailing_zero_padding_after_the_trailer_is_accepted_without_phantom_members() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (ino mode name data)
      (let* ((name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino mode 0 0 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data))))))
  (let ((display
         (generate-new-buffer " *cpio padded listing*")))
    (unwind-protect
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert
           (entry 1 #o100644 "one" "1")
           (entry 0 0 "TRAILER!!!" "")
           (make-string 64 0))
          (let ((files
                 (archive-cpio-summarize display)))
            (list
             (length files)
             (archive--file-desc-ext-file-name
              (aref files 0))
             (with-current-buffer display
               (count-lines
                (point-min) (point-max))))))
      (kill-buffer display))))"##;
    let expect = expect![[r#"OK (1 "one" 3)"#]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn archive_cpio_extract_reads_a_fixed_sandbox_file_and_preserves_binary_bytes() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (name data)
      (let* ((name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               31 #o100600 0 0 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data))))))
  (let* ((path
          (expand-file-name
           "archive-cpio-extract.cpio"
           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
         (payload
          (unibyte-string 0 10 13 127 128 200 255)))
    (unwind-protect
        (progn
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (insert (entry "raw/data.bin" payload))
            (write-region
             (point-min) (point-max) path nil 'silent))
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (let ((found
                   (archive-cpio-extract
                    path "raw/data.bin")))
              (list
               found
               (string-to-list (buffer-string))
               (secure-hash 'sha256
                            (current-buffer))))))
      (when (file-exists-p path)
        (delete-file path)))))"##;
    let expect = expect![[
        r#"OK (t (0 10 13 127 128 200 255) "eae0cda9efb66c7ecf667019b04ee679a8b6022e61a351bc2687017d1c666efe")"#
    ]];
    assert_archive_cpio_parity(elisp_form, expect);
}

#[test]
fn summarize_rejects_an_unrecognized_header_at_the_next_aligned_member() {
    let elisp_form = r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert
   (concat
    "070701"
    (make-string 88 ?0)
    "0000000400000000"
    "x\0\0\0"
    "BORK"
    (make-string 106 ?0)))
  (archive-cpio-summarize))"##;
    let expect = expect![[r#"ERR (error "Unknown mode 0")"#]];
    assert_archive_cpio_signal_parity(elisp_form, expect);
}

#[test]
fn summarize_rejects_a_member_with_unknown_posix_file_kind() {
    let elisp_form = r##"(let* ((name-field "mystery\0")
       (header
        (format
         "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
         1 #o160644 0 0 1 0 0
         0 0 0 0 (length name-field) 0)))
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert header name-field)
    (insert
     (make-string
      (% (- 4 (% (+ 110 (length name-field)) 4)) 4)
      0))
    (archive-cpio-summarize)))"##;
    let expect = expect![[r#"ERR (error "Unknown mode 57764")"#]];
    assert_archive_cpio_signal_parity(elisp_form, expect);
}
