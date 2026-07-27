use expect_test::expect;

use super::assert_archive_rpm_parity;

#[test]
fn summarizes_a_complete_gzip_rpm_into_metadata_and_practical_file_descriptors() {
    let elisp_form = r##"(cl-labels
    ((pad4
      (size)
      (make-string (% (- 4 (% size 4)) 4) 0))
     (cpio-entry
      (ino mode uid gid name contents)
      (let* ((data (string-as-unibyte contents))
             (name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino mode uid gid 1 1700000000 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data)))))
     (gzip
      (data)
      (with-temp-buffer
        (set-buffer-multibyte nil)
        (insert data)
        (unless
            (zerop
             (call-process-region
              (point-min) (point-max)
              "gzip" t t nil "-n" "-c"))
          (error "gzip fixture creation failed"))
        (buffer-string)))
     (rpm
      (payload)
      (let* ((fields
              '((1000 "neomacs-tools")
                (1001 "2.7.1")
                (1002 "4.fc42")
                (1004 "Practical Neomacs utilities")
                (1011 "Neomacs Project")
                (1014 "GPL-3.0-or-later")
                (1020 "https://neomacs.example/packages")
                (1021 "linux")
                (1022 "x86_64")
                (1124 "cpio")
                (1125 "gzip")))
             (offset 0)
             indexes data)
        (dolist (field fields)
          (push
           `((:tag . ,(car field))
             (:type . 6)
             (:offset . ,offset)
             (:count . 1))
           indexes)
          (setq data
                (concat data (cadr field) "\0")
                offset
                (+ offset 1 (length (cadr field)))))
        (setq indexes (nreverse indexes))
        (concat
         (apply #'unibyte-string
                '(237 171 238 219 3 0))
         (make-string 90 0)
         (bindat-pack
          archive-rpm--header-bindat-spec
          `((:magic . ,#x8eade8)
            (:version . 1)
            (:reserved . 0)
            (:n-index-entries . 0)
            (:data-len . 0)))
         (bindat-pack
          archive-rpm--header-bindat-spec
          `((:magic . ,#x8eade8)
            (:version . 1)
            (:reserved . 0)
            (:n-index-entries . ,(length indexes))
            (:data-len . ,(length data))))
         (mapconcat
          (lambda (entry)
            (bindat-pack
             archive-rpm--index-entry-bindat-spec
             entry))
          indexes "")
         data payload))))
  (let* ((cpio
          (concat
           (cpio-entry
            1 #o040755 0 0 "./usr/share/doc/neomacs-tools/" "")
           (cpio-entry
            2 #o100644 0 0
            "./usr/share/doc/neomacs-tools/README.md"
            "# Neomacs Tools\n\nInstalled from RPM.\n")
           (cpio-entry
            3 #o100755 0 0
            "./usr/bin/neomacs-helper"
            "#!/bin/sh\nexec neomacs \"$@\"\n")
           (cpio-entry
            0 0 0 0 "TRAILER!!!" "")))
         (rpm-bytes (rpm (gzip cpio)))
         (raw-size (length rpm-bytes))
         files)
    (with-temp-buffer
      (set-buffer-multibyte nil)
      (insert rpm-bytes)
      (goto-char (point-min))
      (setq files (archive-rpm-summarize))
      (list
       raw-size
       (buffer-substring
        (point-min)
        (save-excursion
          (goto-char (point-min))
          (search-forward
           "M   Filemode")
          (line-beginning-position)))
       (mapcar
        (lambda (file)
          (list
           (archive--file-desc-ext-file-name file)
           (archive--file-desc-mode file)
           (archive--file-desc-size file)))
        (append files nil))
       (> (buffer-size) raw-size)))))"##;
    let expect = expect![[
        r#"OK (665 "Name:         neomacs-tools\nVersion:      2.7.1\nRelease:      4.fc42\nSummary:      Practical Neomacs utilities\nVendor:       Neomacs Project\nLicense:      GPL-3.0-or-later\nURL:          https://neomacs.example/packages\nOS:           linux\nArchitecture: x86_64\nFormat:       cpio\nCompression:  gzip\n\n" (("./usr/share/doc/neomacs-tools/" 16877 0) ("./usr/share/doc/neomacs-tools/README.md" 33188 37) ("./usr/bin/neomacs-helper" 33261 28)) t)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn extracts_a_nested_text_file_from_a_complete_rpm_on_a_fixed_sandbox_path() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (cpio-entry
      (ino mode name contents)
      (let* ((data (string-as-unibyte contents))
             (name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino mode 0 0 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data)))))
     (gzip
      (data)
      (with-temp-buffer
        (set-buffer-multibyte nil)
        (insert data)
        (unless
            (zerop
             (call-process-region
              (point-min) (point-max)
              "gzip" t t nil "-n" "-c"))
          (error "gzip fixture creation failed"))
        (buffer-string)))
     (rpm
      (payload)
      (let* ((fields
              '((1000 "workflow-demo")
                (1001 "1.0")
                (1002 "1")
                (1022 "noarch")
                (1124 "cpio")
                (1125 "gzip")))
             (offset 0) indexes data)
        (dolist (field fields)
          (push
           `((:tag . ,(car field)) (:type . 6)
             (:offset . ,offset) (:count . 1))
           indexes)
          (setq data
                (concat data (cadr field) "\0")
                offset (+ offset (length (cadr field)) 1)))
        (setq indexes (nreverse indexes))
        (concat
         (apply #'unibyte-string
                '(237 171 238 219 3 0))
         (make-string 90 0)
         (bindat-pack
          archive-rpm--header-bindat-spec
          `((:magic . ,#x8eade8) (:version . 1)
            (:reserved . 0) (:n-index-entries . 0)
            (:data-len . 0)))
         (bindat-pack
          archive-rpm--header-bindat-spec
          `((:magic . ,#x8eade8) (:version . 1)
            (:reserved . 0)
            (:n-index-entries . ,(length indexes))
            (:data-len . ,(length data))))
         (mapconcat
          (lambda (entry)
            (bindat-pack
             archive-rpm--index-entry-bindat-spec entry))
          indexes "")
         data payload))))
  (let* ((path
          (expand-file-name
           "archive-rpm-text-workflow.rpm"
           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
         (target
          "./usr/share/doc/workflow-demo/guide.txt")
         (contents
          "Install\n=======\n\n1. Run neomacs.\n2. Open project.\n")
         (cpio
          (concat
           (cpio-entry 1 #o100644 target contents)
           (cpio-entry 0 0 "TRAILER!!!" "")))
         buffer)
    (unwind-protect
        (progn
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (insert (rpm (gzip cpio)))
            (write-region
             (point-min) (point-max) path nil 'silent))
          (with-temp-buffer
            (let ((found
                   (archive-rpm-extract path target)))
              (setq buffer
                    (find-buffer-visiting path))
              (list found
                    (buffer-string)
                    (secure-hash
                     'sha256 (current-buffer))
                    (and buffer
                         (buffer-local-value
                          'major-mode buffer))))))
      (when (buffer-live-p buffer)
        (kill-buffer buffer))
      (when (file-exists-p path)
        (delete-file path)))))"##;
    let expect = expect![[
        r#"OK (t "Install\n=======\n\n1. Run neomacs.\n2. Open project.\n" "9015661ecd489b44ee1e1d294e9c9bc3047f7f0f31e786b57935532a9bb6de4c" archive-mode)"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn extracts_high_bytes_and_nuls_from_an_rpm_without_text_conversion() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (cpio-entry
      (name contents)
      (let* ((data (string-as-unibyte contents))
             (name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               1 #o100600 0 0 1 0 (length data)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                data (pad4 (length data)))))
     (gzip
      (data)
      (with-temp-buffer
        (set-buffer-multibyte nil)
        (insert data)
        (call-process-region
         (point-min) (point-max)
         "gzip" t t nil "-n" "-c")
        (buffer-string)))
     (rpm
      (payload)
      (let* ((data "cpio\0gzip\0")
             (indexes
              '(((:tag . 1124) (:type . 6)
                 (:offset . 0) (:count . 1))
                ((:tag . 1125) (:type . 6)
                 (:offset . 5) (:count . 1)))))
        (concat
         (apply #'unibyte-string
                '(237 171 238 219 3 0))
         (make-string 90 0)
         (bindat-pack
          archive-rpm--header-bindat-spec
          `((:magic . ,#x8eade8) (:version . 1)
            (:reserved . 0) (:n-index-entries . 0)
            (:data-len . 0)))
         (bindat-pack
          archive-rpm--header-bindat-spec
          `((:magic . ,#x8eade8) (:version . 1)
            (:reserved . 0) (:n-index-entries . 2)
            (:data-len . ,(length data))))
         (mapconcat
          (lambda (entry)
            (bindat-pack
             archive-rpm--index-entry-bindat-spec entry))
          indexes "")
         data payload))))
  (let* ((path
          (expand-file-name
           "archive-rpm-binary-workflow.rpm"
           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
         (name "./var/lib/demo/data.bin")
         (binary
          (apply #'unibyte-string
                 '(0 1 2 10 13 31 32 127 128 129
                   191 200 254 255)))
         (cpio
          (concat
           (cpio-entry name binary)
           (cpio-entry "TRAILER!!!" "")))
         visiting)
    (unwind-protect
        (progn
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (insert (rpm (gzip cpio)))
            (write-region
             (point-min) (point-max) path nil 'silent))
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (let ((found
                   (archive-rpm-extract path name)))
              (setq visiting
                    (find-buffer-visiting path))
              (list
               found
               (string-to-list (buffer-string))
               (equal binary (buffer-string))
               enable-multibyte-characters))))
      (when (buffer-live-p visiting)
        (kill-buffer visiting))
      (when (file-exists-p path)
        (delete-file path)))))"##;
    let expect = expect!["OK (t (0 1 2 10 13 31 32 127 128 129 191 200 254 255) t nil)"];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn missing_member_from_a_valid_rpm_returns_nil_and_keeps_destination_unchanged() {
    let elisp_form = r##"(cl-labels
    ((pad4 (size)
       (make-string (% (- 4 (% size 4)) 4) 0))
     (entry
      (name contents mode ino)
      (let* ((name-field (concat name "\0"))
             (header
              (format
               "070701%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x%08x"
               ino mode 0 0 1 0 (length contents)
               0 0 0 0 (length name-field) 0)))
        (concat header name-field
                (pad4 (+ 110 (length name-field)))
                contents
                (pad4 (length contents)))))
     (gzip
      (data)
      (with-temp-buffer
        (set-buffer-multibyte nil)
        (insert data)
        (call-process-region
         (point-min) (point-max)
         "gzip" t t nil "-n" "-c")
        (buffer-string))))
  (let* ((metadata "cpio\0gzip\0")
         (indexes
          '(((:tag . 1124) (:type . 6)
             (:offset . 0) (:count . 1))
            ((:tag . 1125) (:type . 6)
             (:offset . 5) (:count . 1))))
         (cpio
          (concat
           (entry "./present.txt" "present" #o100644 1)
           (entry "TRAILER!!!" "" 0 0)))
         (rpm
          (concat
           (apply #'unibyte-string
                  '(237 171 238 219 3 0))
           (make-string 90 0)
           (bindat-pack
            archive-rpm--header-bindat-spec
            `((:magic . ,#x8eade8) (:version . 1)
              (:reserved . 0) (:n-index-entries . 0)
              (:data-len . 0)))
           (bindat-pack
            archive-rpm--header-bindat-spec
            `((:magic . ,#x8eade8) (:version . 1)
              (:reserved . 0) (:n-index-entries . 2)
              (:data-len . ,(length metadata))))
           (mapconcat
            (lambda (index)
              (bindat-pack
               archive-rpm--index-entry-bindat-spec
               index))
            indexes "")
           metadata (gzip cpio)))
         (path
          (expand-file-name
           "archive-rpm-missing-member.rpm"
           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
         visiting)
    (unwind-protect
        (progn
          (with-temp-buffer
            (set-buffer-multibyte nil)
            (insert rpm)
            (write-region
             (point-min) (point-max) path nil 'silent))
          (with-temp-buffer
            (insert "destination sentinel")
            (goto-char 12)
            (let ((found
                   (archive-rpm-extract
                    path "./absent.txt")))
              (setq visiting
                    (find-buffer-visiting path))
              (list found
                    (point)
                    (buffer-string)))))
      (when (buffer-live-p visiting)
        (kill-buffer visiting))
      (when (file-exists-p path)
        (delete-file path)))))"##;
    let expect = expect![[r#"OK (nil 12 "destination sentinel")"#]];
    assert_archive_rpm_parity(elisp_form, expect);
}

#[test]
fn extraction_finds_embedded_lead_after_archive_mode_has_prepended_a_summary() {
    let elisp_form = r##"(let ((archive
       (generate-new-buffer " *rpm summarized source*"))
      (destination
       (generate-new-buffer " *rpm summarized destination*"))
      calls)
  (unwind-protect
      (progn
        (with-current-buffer archive
          (set-buffer-multibyte nil)
          (insert
           "Name:      demo\n\n"
           "M   Filemode   Length File\n"
           (apply #'unibyte-string
                  '(237 171 238 219 3 0))
           (make-string 90 0)
           (apply #'unibyte-string
                  '(142 173 232 1))
           (make-string 12 0)
           (apply #'unibyte-string
                  '(142 173 232 1))
           (make-string 12 0)
           "compressed")
          (goto-char (point-min)))
        (cl-letf
            (((symbol-function 'find-file-noselect)
              (lambda (_path) archive))
             ((symbol-function
               'archive-rpm--parse-header)
              (lambda (align)
                (push
                 (list align (point))
                 calls)
                nil))
             ((symbol-function
               'archive-rpm--decompress-payload)
              (lambda (payload entries)
                (push
                 (list 'decompress payload entries)
                 calls)
                (insert "decompressed-cpio")))
             ((symbol-function
               'archive-cpio-extract-from-buffer)
              (lambda (name source dest)
                (push
                 (list 'extract name
                       (buffer-string)
                       (eq dest destination))
                 calls)
                'extracted)))
          (with-current-buffer destination
            (list
             (archive-rpm-extract
              "ignored.rpm" "./file.txt")
             (nreverse calls)))))
    (kill-buffer archive)
    (kill-buffer destination)))"##;
    let expect = expect![[
        r#"OK (extracted ((t 141) (nil 141) (decompress "������\1\0\0\0\0\0\0\0\0\0\0\0\0������\1\0\0\0\0\0\0\0\0\0\0\0\0compressed" nil) (extract "./file.txt" "decompressed-cpio" t)))"#
    ]];
    assert_archive_rpm_parity(elisp_form, expect);
}
