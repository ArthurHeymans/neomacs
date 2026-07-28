use expect_test::expect;

use super::assert_aa_edit_mode_parity;

#[test]
fn aa_edit_mode_opens_a_real_shift_jis_mlt_file_as_a_ready_to_edit_art_buffer() {
    let elisp_form = r##"(let* ((path
        (aa-edit-test-write-mlt "aa gallery/opening scene.mlt" "[SPLIT]\n"))
       (digest (aa-edit-test-file-sha256 path))
       (buffer (find-file-noselect path)))
  (unwind-protect
      (with-current-buffer buffer
        (list
         (file-name-nondirectory (buffer-file-name))
         major-mode
         mode-name
         (derived-mode-p 'text-mode)
         buffer-file-coding-system
         enable-multibyte-characters
         (local-variable-p 'page-delimiter)
         page-delimiter
         buffer-face-mode
         buffer-face-mode-face
         (local-variable-p 'buffer-face-mode-face)
         (buffer-size)
         (buffer-substring-no-properties (point-min) (point-max))
         (point)
         (buffer-modified-p)
         (get-text-property (point-min) 'charset)
         (get-text-property (- (point-max) 1) 'charset)
         (save-excursion
           (goto-char (point-min))
           (search-forward "[SPLIT]")
           (get-text-property (match-beginning 0) 'charset))
         (equal digest (aa-edit-test-file-sha256 path))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ("opening scene.mlt" aa-edit-mode "（´д｀）" text-mode japanese-shift-jis-unix t t "^\\[SPLIT]" t navi2ch-mona16-face t 62 "　　　（´д｀）\n　＿ノ　　ヽ、＿\n[SPLIT]\nやる夫「ＡＡだお」\n　　∧＿∧\n　（　´∀｀）\n[SPLIT]\nおわり\n" 1 nil japanese-jisx0208 japanese-jisx0208 japanese-jisx0208 t)"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_page_commands_walk_the_split_separated_art_panels() {
    let elisp_form = r##"(let ((buffer
       (find-file-noselect
        (aa-edit-test-write-mlt "aa gallery/panels.mlt" "[SPLIT]\n"))))
  (unwind-protect
      (with-current-buffer buffer
        (let (positions pages)
          (goto-char (point-min))
          (push (point) positions)
          (forward-page 1)
          (push (point) positions)
          (forward-page 1)
          (push (point) positions)
          (forward-page 1)
          (push (point) positions)
          (backward-page 1)
          (push (point) positions)
          (backward-page 2)
          (push (point) positions)
          (dolist (start (list (point-min) 40 (point-max)))
            (save-restriction
              (save-excursion
                (goto-char start)
                (narrow-to-page)
                (push (list (point-min)
                            (point-max)
                            (buffer-substring-no-properties
                             (point-min)
                             (point-max)))
                      pages))))
          (list (nreverse positions)
                (nreverse pages)
                (buffer-size)
                (point-min)
                (point-max)
                (buffer-modified-p))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((1 26 58 63 58 1) ((1 19 "　　　（´д｀）\n　＿ノ　　ヽ、＿\n") (27 51 "やる夫「ＡＡだお」\n　　∧＿∧\n　（　´∀｀）\n") (59 63 "おわり\n")) 62 1 63 nil)"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_saves_an_appended_panel_back_as_shift_jis_bytes() {
    let elisp_form = r##"(let* ((path (aa-edit-test-write-mlt "saving/story.mlt" "[SPLIT]\n"))
       (original (aa-edit-test-file-sha256 path))
       (buffer (find-file-noselect path)))
  (unwind-protect
      (with-current-buffer buffer
        (goto-char (point-max))
        (insert "[SPLIT]\nおまけ（´・ω・｀）\n")
        (let ((modified-before-save (buffer-modified-p)))
          (save-buffer)
          (list
           modified-before-save
           (buffer-modified-p)
           buffer-file-coding-system
           (buffer-size)
           (equal original (aa-edit-test-file-sha256 path))
           (equal (aa-edit-test-file-bytes path)
                  (encode-coding-string
                   (buffer-substring-no-properties (point-min) (point-max))
                   'japanese-shift-jis
                   t))
           (length (aa-edit-test-file-bytes path))
           (aa-edit-test-directory-listing "saving")
           (with-temp-buffer
             (insert-file-contents path)
             (list buffer-file-coding-system
                   (buffer-substring-no-properties (point-min) (point-max)))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK (t nil japanese-shift-jis-unix 81 nil t 131 (("story.mlt" . 131) ("story.mlt~" . 102)) (japanese-shift-jis-unix "　　　（´д｀）\n　＿ノ　　ヽ、＿\n[SPLIT]\nやる夫「ＡＡだお」\n　　∧＿∧\n　（　´∀｀）\n[SPLIT]\nおわり\n[SPLIT]\nおまけ（´・ω・｀）\n"))"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_honours_a_customized_delimiter_pattern_for_page_navigation() {
    let elisp_form = r##"(let* ((text
        (concat "　（＾ω＾）\n"
                "=====\n"
                "本文に[SPLIT]と書いても区切らない\n"
                "=====\n"
                "おしまい\n"))
       (path (aa-edit-test-write-file "custom/scene.mlt" text))
       (aa-edit-delimiter-pattern "^=====$")
       (buffer (find-file-noselect path)))
  (unwind-protect
      (with-current-buffer buffer
        (let (positions)
          (goto-char (point-min))
          (push (point) positions)
          (forward-page 1)
          (push (point) positions)
          (forward-page 1)
          (push (point) positions)
          (list
           page-delimiter
           aa-edit-delimiter-pattern
           aa-edit-mlt-delimiter-regexp
           (local-variable-p 'page-delimiter)
           (nreverse positions)
           (save-restriction
             (save-excursion
               (goto-char (point-min))
               (narrow-to-page)
               (buffer-substring-no-properties (point-min) (point-max))))
           (save-restriction
             (save-excursion
               (goto-char (point-max))
               (narrow-to-page)
               (buffer-substring-no-properties (point-min) (point-max))))
           (buffer-size))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ("^=====$" "^=====$" "^\\[SPLIT]" t (1 13 40) "　（＾ω＾）\n" "おしまい\n" 45)"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_claims_mlt_files_on_disk_and_leaves_other_suffixes_alone() {
    let elisp_form = r##"(list
 (assoc "\\.mlt\\'" auto-mode-alist)
 auto-mode-case-fold
 (mapcar
  (lambda (name)
    (let ((buffer
           (find-file-noselect
            (aa-edit-test-write-file name "　（´д｀）\n"))))
      (unwind-protect
          (with-current-buffer buffer
            (list (file-name-nondirectory (buffer-file-name))
                  major-mode
                  mode-name
                  (local-variable-p 'page-delimiter)
                  buffer-face-mode))
        (kill-buffer buffer))))
  '("routing/scene.mlt"
    "routing/SCENE.MLT"
    "routing/scene.mlt.txt"
    "routing/mlt")))"##;
    let expect = expect![[
        r#"OK (("\\.mlt\\'" . aa-edit-mode) t (("scene.mlt" aa-edit-mode "（´д｀）" t t) ("SCENE.MLT" aa-edit-mode "（´д｀）" t t) ("scene.mlt.txt" text-mode "Text" nil nil) ("mlt" fundamental-mode "Fundamental" nil nil)))"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_follows_the_navi2ch_mona_face_configuration_of_the_reader() {
    let elisp_form = r##"(let ((path (aa-edit-test-write-file "faces/scene.mlt" "　（´д｀）\n")))
  (cl-flet ((open-face (setting)
              (let ((navi2ch-mona-face-variable setting))
                (let ((buffer (find-file-noselect path)))
                  (unwind-protect
                      (with-current-buffer buffer
                        (list buffer-face-mode
                              buffer-face-mode-face
                              (local-variable-p 'buffer-face-mode-face)
                              major-mode))
                    (kill-buffer buffer))))))
    (list
     (default-value 'navi2ch-mona-face-variable)
     (custom-variable-p 'navi2ch-mona-face-variable)
     (open-face t)
     (open-face 'my-own-aa-face)
     (open-face '(:family "IPAMonaPGothic" :height 120))
     (default-value 'navi2ch-mona-face-variable))))"##;
    let expect = expect![[
        r#"OK (t (t) (t navi2ch-mona16-face t aa-edit-mode) (t my-own-aa-face t aa-edit-mode) (t (:family "IPAMonaPGothic" :height 120) t aa-edit-mode) t)"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}
