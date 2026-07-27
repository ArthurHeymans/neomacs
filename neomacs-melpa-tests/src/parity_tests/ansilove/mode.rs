use expect_test::expect;

use super::assert_ansilove_parity;

#[test]
fn editable_command_restores_a_practical_text_buffer_to_fundamental_editing_state() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-read-only t)
  (let (messages)
    (cl-letf (((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push (apply #'format format-string arguments) messages))))
      (let ((result (ansilove-turn-to-editable-mode)))
        (insert "editable text")
        (list
         result
         major-mode
         mode-name
         buffer-read-only
         (buffer-string)
         (nreverse messages))))))"##;
    let expect = expect![[
        r#"OK (#1=("Warning: Entered editable mode.") fundamental-mode "Fundamental" nil "editable text" #1#)"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn major_mode_installs_read_only_state_map_and_hook_for_a_real_ansi_document() {
    let elisp_form = r##"(with-temp-buffer
  (insert "\e[31mRED\e[0m\n\e[1;34mBOLD BLUE\e[0m")
  (let ((ansilove-mode-hook
         (list
          (lambda ()
            (setq-local ansilove-hook-observation
                        (list major-mode
                              buffer-read-only
                              (buffer-string))))))
        messages)
    (cl-letf (((symbol-function 'image-type-available-p)
               (lambda (type) (eq type 'imagemagick)))
              ((symbol-function 'display-images-p) (lambda () t))
              ((symbol-function 'ansilove--check-executable) (lambda () t))
              ((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push (apply #'format format-string arguments) messages))))
      (ansilove-mode)
      (list
       major-mode
       mode-name
       buffer-read-only
       (eq (current-local-map) ansilove-mode-map)
       ansilove-hook-observation
       (mapcar
        (lambda (key) (lookup-key (current-local-map) (kbd key)))
        '("a" "e" "q" "?"))
       (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (ansilove-mode "ansilove" t t (ansilove-mode t "\33[31mRED\33[0m\n\33[1;34mBOLD BLUE\33[0m") (ansilove ansilove-turn-to-editable-mode quit-window describe-mode) ("Press the \"a\" key to view this buffer as a PNG image."))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn major_mode_reports_each_missing_runtime_capability_in_stable_order() {
    let elisp_form = r##"(with-temp-buffer
  (let ((ansilove-executable "/missing/ansilove")
        messages)
    (cl-letf (((symbol-function 'image-type-available-p) (lambda (_type) nil))
              ((symbol-function 'display-images-p) (lambda () nil))
              ((symbol-function 'ansilove--check-executable) (lambda () nil))
              ((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (push (apply #'format format-string arguments) messages))))
      (ansilove-mode)
      (list
       major-mode
       buffer-read-only
       (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (ansilove-mode t ("Press the \"a\" key to view this buffer as a PNG image." "Warning: ImageMagick support is missing from this version of Emacs." "Warning: Currently used display does not support displaying images." "Warning: The required executable /missing/ansilove is unusable!"))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn every_supported_filename_selects_ansilove_mode_through_auto_mode_detection() {
    let elisp_form = r##"(let (results)
  (dolist (extension ansilove-supported-file-extensions)
    (with-temp-buffer
      (setq buffer-file-name
            (expand-file-name
             (format "practical-art.%s" extension)
             temporary-file-directory))
      (cl-letf (((symbol-function 'image-type-available-p) (lambda (_type) t))
                ((symbol-function 'display-images-p) (lambda () t))
                ((symbol-function 'ansilove--check-executable) (lambda () t))
                ((symbol-function 'message) (lambda (&rest _arguments) nil)))
        (set-auto-mode)
        (push
         (list extension
               major-mode
               mode-name
               buffer-read-only
               (eq (current-local-map) ansilove-mode-map))
         results))))
  (nreverse results))"##;
    let expect = expect![[
        r#"OK (("adf" ansilove-mode "ansilove" t t) ("ans" ansilove-mode "ansilove" t t) ("bin" ansilove-mode "ansilove" t t) ("idf" ansilove-mode "ansilove" t t) ("pcb" ansilove-mode "ansilove" t t) ("tnd" ansilove-mode "ansilove" t t) ("xb" ansilove-mode "ansilove" t t))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}
