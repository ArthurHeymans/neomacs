use expect_test::expect;

use super::{
    assert_aozora_view_autoload_parity, assert_aozora_view_parity, assert_aozora_view_signal_parity,
};

#[test]
fn installed_descriptor_source_feature_and_data_payload_match_exact_melpa_transaction() {
    let elisp_form = r##"(let* ((descriptor
                          (cadr
                           (assq
                            'aozora-view
                            package-alist)))
                         (source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
                         (data
                          (expand-file-name
                           "aozora_gaiji_chuki.txt"
                           (file-name-directory
                            source))))
                     (with-temp-buffer
                       (insert-file-contents-literally
                        data)
                       (list
                        (featurep 'aozora-view)
                        (package-desc-name descriptor)
                        (package-version-join
                         (package-desc-version descriptor))
                        (package-desc-reqs descriptor)
                        (package-desc-summary descriptor)
                        (file-name-nondirectory
                         (symbol-file
                          'aozora-view-draw
                          'defun))
                        (file-name-nondirectory source)
                        (buffer-size)
                        (secure-hash
                         'sha256
                         (current-buffer)))))"##;
    let expect = expect![[
        r#"OK (t aozora-view "20140310.1317" nil "Aozora Bunko text Emacs viewer." "aozora-view.el" "aozora-view.el" 634656 "8348d617290decd1296e03a685e08048cc68cd69aa243d5a9972ac5bb312fddc")"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_only_the_public_view_command_before_source_load() {
    let elisp_form = r##"(list
                      (featurep 'aozora-view)
                      (featurep
                       'aozora-view-autoloads)
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (fboundp symbol)
                          (and
                           (fboundp symbol)
                           (autoloadp
                            (symbol-function
                             symbol)))))
                       '(aozora-view
                         aozora-view-mode
                         aozora-view-draw
                         aozora-view-bookmark
                         aozora-arrange-fill-lines))
                      (boundp
                       'aozora-fill-column)
                      (and
                       (member
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![
        "OK (nil t ((aozora-view t t) (aozora-view-mode nil nil) (aozora-view-draw nil nil) (aozora-view-bookmark nil nil) (aozora-arrange-fill-lines nil nil)) nil nil)"
    ];
    assert_aozora_view_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (copy-tree
                          (help-function-arglist
                           function t))
                         (commandp function)))
                      '(aozora-view-arrange-replace
                        aozora-view-buffer-width
                        aozora-view-arrange-fill-lines
                        aozora-view-cache-file
                        aozora-view-load-cache
                        aozora-view-save-cache
                        aozora-view-bookmark
                        aozora-view-restore-bookmark
                        aozora-view-suspend
                        aozora-view-traditional
                        aozora-view-redraw
                        aozora-view-draw
                        aozora-view
                        aozora-arrange-fill-lines
                        aozora-view-mode))"##;
    let expect = expect![
        "OK ((aozora-view-arrange-replace nil nil) (aozora-view-buffer-width (start end) nil) (aozora-view-arrange-fill-lines nil nil) (aozora-view-cache-file (file-name) nil) (aozora-view-load-cache (file-name) nil) (aozora-view-save-cache (file-name) nil) (aozora-view-bookmark (arg) t) (aozora-view-restore-bookmark nil t) (aozora-view-suspend nil t) (aozora-view-traditional nil t) (aozora-view-redraw nil t) (aozora-view-draw (text-buffer text-file-name) nil) (aozora-view nil t) (aozora-arrange-fill-lines (_entry) nil) (aozora-view-mode nil t))"
    ];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn viewer_configuration_buffer_locality_and_kenten_vocabulary_are_exact() {
    let elisp_form = r##"(list
                      aozora-fill-column
                      (file-name-nondirectory
                       (directory-file-name
                        aozora-view-cache-directory))
                      aozora-view-cache-ext
                      aozora-view-save-cache
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (local-variable-if-set-p
                           symbol)
                          (default-value symbol)))
                       '(aozora-view-text-buffer
                         aozora-view-text-file
                         aozora-view-buffer
                         aozora-view-bookmarks))
                      aozora-kenten-alist
                      aozora-kenten-regexp)"##;
    let expect = expect![[
        r#"OK (0.8 "aozora-view" ".cache.gz" prompt ((aozora-view-text-buffer t nil) (aozora-view-text-file t nil) (aozora-view-buffer t nil) (aozora-view-bookmarks nil nil)) (("傍点" . 65093) ("白ゴマ傍点" . 65094) ("丸傍点" . 9679) ("白丸傍点" . 9675) ("傍点（白丸）" . 9675) ("黒三角傍点" . 9650) ("白三角傍点" . 9651) ("二重丸傍点" . 9678) ("蛇の目傍点" . 9673)) "［＃「\\([^」]+?\\)」に\\(丸傍点\\|二重丸傍点\\|傍点\\(?:（白丸）\\)?\\|\\(?:白\\(?:ゴマ\\|三角\\|丸\\)\\|蛇の目\\|黒三角\\)傍点\\)］\n?")"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_inherits_view_mode_and_binds_every_viewer_operation() {
    let elisp_form = r##"(list
                      (eq
                       (keymap-parent
                        aozora-view-mode-map)
                       view-mode-map)
                      (mapcar
                       (lambda (key)
                         (cons
                          key
                          (lookup-key
                           aozora-view-mode-map
                           key)))
                       '("b" "," "q" "t" "l" "SPC" "DEL"))
                      (get
                       'aozora-view-mode
                       'derived-mode-parent)
                      (get
                       'aozora-view-mode
                       'mode-class))"##;
    let expect = expect![[
        r#"OK (t (("b" . aozora-view-bookmark) ("," . aozora-view-restore-bookmark) ("q" . aozora-view-suspend) ("t" . aozora-view-traditional) ("l" . aozora-view-redraw) ("SPC" . 1) ("DEL" . 1)) text-mode nil)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn entering_view_mode_surfaces_the_removed_toggle_read_only_dependency() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "第一行\n第二行")
                     (set-buffer-modified-p nil)
                     (aozora-view-mode))"##;
    let expect = expect!["ERR (void-function toggle-read-only)"];
    assert_aozora_view_signal_parity(elisp_form, expect);
}

#[test]
fn view_mode_semantics_after_compatibility_shim_are_read_only_and_non_destructive() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "第一行\n第二行")
                     (set-buffer-modified-p nil)
                     (cl-letf
                         (((symbol-function
                            'toggle-read-only)
                           (lambda
                             (&optional argument)
                             (setq buffer-read-only
                                   (if argument
                                       (> (prefix-numeric-value
                                           argument)
                                          0)
                                     (not buffer-read-only))))))
                       (aozora-view-mode)
                       (list
                        major-mode
                        mode-name
                        buffer-read-only
                        view-mode
                        line-spacing
                        (eq
                         (current-local-map)
                         aozora-view-mode-map)
                        (buffer-modified-p)
                        (buffer-string))))"##;
    let expect = expect![[r#"OK (aozora-view-mode "青空文庫" t t 0 t nil "第一行\n第二行")"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn gaiji_table_is_complete_and_maps_representative_real_annotations() {
    let elisp_form = r##"(let ((keys nil))
                     (maphash
                      (lambda (key _value)
                        (push key keys))
                      aozora-view-gaiji-table)
                     (setq keys
                           (sort keys #'string<))
                     (list
                      (hash-table-count
                       aozora-view-gaiji-table)
                      (car keys)
                      (car
                       (last keys))
                      (mapcar
                       (lambda (key)
                         (cons
                          key
                          (gethash
                           key
                           aozora-view-gaiji-table)))
                       '("「朽のつくり」"
                         "ローマ数字1"
                         "感嘆符二つ"
                         "疑問符一つ感嘆符二つ"
                         "濁点付き片仮名ワ"))))"##;
    let expect = expect![[
        r#"OK (8428 "2プラス" "［＃「ぞ」は底本では「濁点付き平仮名う」" (("「朽のつくり」" . "丂") ("ローマ数字1" . "Ⅰ") ("感嘆符二つ" . "‼") ("疑問符一つ感嘆符二つ") ("濁点付き片仮名ワ" . "ヷ")))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn accent_translation_table_handles_ligatures_strokes_and_combining_marks() {
    let elisp_form = r##"(progn
                     (require 'ucs-normalize)
                     (mapcar
                      (lambda (source)
                        (with-temp-buffer
                          (insert source)
                          (translate-region
                           (point-min)
                           (point-max)
                           'aozora-accent-table)
                          (ucs-normalize-NFC-region
                           (point-min)
                           (point-max))
                          (buffer-string)))
                      '("AE& ae& OE& oe&"
                        "A& a& S& O/ o/"
                        "A` E' I^ N~ U: C,"
                        "?!@ ??@")))"##;
    let expect = expect![[r#"OK ("Æ æ Œ œ" "Å å ß Ø ø" "À É Î Ñ Ü Ç" "?¡ ?¿")"#]];
    assert_aozora_view_parity(elisp_form, expect);
}
