//! Divergence tests: face attributes, font specs, display tables, and rendering.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_face_attributes_complete() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (let ((attrs (face-all-attributes 'default (selected-frame))))
    (list (plist-member attrs :family)
          (plist-member attrs :height)
          (plist-member attrs :weight)
          (plist-member attrs :slant)
          (plist-member attrs :foreground)
          (plist-member attrs :background)
          (plist-get attrs :family)
          (stringp (or (plist-get attrs :family) ""))
          (plist-get attrs :foreground)
          (or (stringp (plist-get attrs :foreground))
              (null (plist-get attrs :foreground)))
          (facep 'default)
          (facep 'bold)
          (facep 'italic)
          (facep 'underline)
          (not (facep 'nonexistent-face-xxx))))) "#,
    );
}

#[test]
fn divergence_face_all_attributes_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (make-face 'test-faac-xxx)
  (set-face-attribute 'test-faac-xxx nil
                      :foreground "red"
                      :background "blue"
                      :weight 'bold
                      :slant 'italic
                      :underline t
                      :overline t
                      :strike-through t
                      :box nil
                      :inherit 'default)
  (let ((attrs (face-all-attributes 'test-faac-xxx (selected-frame))))
    (list (facep 'test-faac-xxx)
          (equal (plist-get attrs :foreground) "red")
          (equal (plist-get attrs :background) "blue")
          (eq (plist-get attrs :weight) 'bold)
          (eq (plist-get attrs :slant) 'italic)
          (plist-get attrs :underline)
          (plist-get attrs :overline)
          (plist-get attrs :strike-through)
          (eq (plist-get attrs :inherit) 'default)))) "#,
    );
}

#[test]
fn divergence_face_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (make-face 'test-fdoc-xxx)
  (set-face-documentation 'test-fdoc-xxx "Test face for divergence tests")
  (list (facep 'test-fdoc-xxx)
        (face-documentation 'test-fdoc-xxx)
        (string= (face-documentation 'test-fdoc-xxx)
                 "Test face for divergence tests")
        (face-documentation 'default)
        (stringp (or (face-documentation 'default) "")))) "#,
    );
}

#[test]
fn divergence_face_inheritance_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (make-face 'test-fic-base-xxx)
  (make-face 'test-fic-mid-xxx)
  (make-face 'test-fic-top-xxx)
  (set-face-attribute 'test-fic-base-xxx nil
                      :foreground "green" :weight 'bold)
  (set-face-attribute 'test-fic-mid-xxx nil
                      :foreground "yellow" :inherit 'test-fic-base-xxx)
  (set-face-attribute 'test-fic-top-xxx nil
                      :background "cyan" :inherit 'test-fic-mid-xxx)
  (list (plist-get (face-all-attributes 'test-fic-base-xxx (selected-frame)) :foreground)
        (equal (plist-get (face-all-attributes 'test-fic-base-xxx (selected-frame)) :foreground)
               "green")
        (eq (plist-get (face-all-attributes 'test-fic-base-xxx (selected-frame)) :weight) 'bold)
        (eq (plist-get (face-all-attributes 'test-fic-mid-xxx (selected-frame)) :inherit)
            'test-fic-base-xxx)
        (eq (plist-get (face-all-attributes 'test-fic-top-xxx (selected-frame)) :inherit)
            'test-fic-mid-xxx)
        (equal (plist-get (face-all-attributes 'test-fic-top-xxx (selected-frame)) :background)
               "cyan"))) "#,
    );
}

#[test]
fn divergence_display_table_setup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (let ((dt (make-display-table)))
    (aset dt ?\t [?\^ ?I])
    (aset dt ?\n [?\$ ?\n])
    (setq buffer-display-table dt)
    (list (display-table-p dt)
          (eq buffer-display-table dt)
          (aref dt ?\t)
          (equal (aref dt ?\t) [?\^ ?I])
          (aref dt ?\n)
          (equal (aref dt ?\n) [?\$ ?\n])
          (length dt)
          (= (length dt) 256)
          (setq buffer-display-table nil)
          (null buffer-display-table)))) "#,
    );
}

#[test]
fn divergence_face_list_and_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (make-face 'test-fls1-xxx)
  (make-face 'test-fls2-xxx)
  (make-face 'test-fls3-xxx)
  (let ((all-faces (face-list)))
    (list (and (memq 'test-fls1-xxx all-faces) t)
          (and (memq 'test-fls2-xxx all-faces) t)
          (and (memq 'test-fls3-xxx all-faces) t)
          (and (memq 'default all-faces) t)
          (and (memq 'bold all-faces) t)
          (> (length all-faces) 10)
          (facep 'test-fls1-xxx)
          (facep 'test-fls2-xxx)
          (facep 'test-fls3-xxx)
          (not (facep 'nonexistent-face-xxx))))) "#,
    );
}

#[test]
fn divergence_face_color_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
          (make-face 'test-fcv-xxx)\n\
          (set-face-attribute 'test-fcv-xxx nil\n\
                              :foreground \"red\" :background \"blue\")\n\
          (let ((fg (color-values \"red\"))\n\
                (bg (color-values \"blue\")))\n\
            (list (or (null fg) (listp fg))\n\
                  (or (null fg) (= (length fg) 3))\n\
                  (or (null fg) (= (car fg) 65535))\n\
                  (or (null bg) (listp bg))\n\
                  (or (null bg) (= (length bg) 3))\n\
                  (or (null bg) (= (nth 2 bg) 65535))\n\
                  (plist-get (face-all-attributes 'test-fcv-xxx\n\
                                                   (selected-frame))\n\
                             :foreground)))) ",
    );
}

#[test]
fn divergence_font_spec_creation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (let ((fs (font-spec :family "Monospace" :weight 'normal :slant 'normal
                        :size 12)))
    (list (fontp fs)
          (font-get fs :family)
          (equal (font-get fs :family) "Monospace")
          (font-get fs :weight)
          (eq (font-get fs :weight) 'normal)
          (font-get fs :slant)
          (eq (font-get fs :slant) 'normal)
          (font-get fs :size)
          (equal (font-get fs :size) 12)
          (font-spec-p fs)))) "#,
    );
}

#[test]
fn divergence_font_spec_spacing_gnu_codes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (font-get (font-spec :spacing 'p) :spacing)
  (font-get (font-spec :spacing 'd) :spacing)
  (font-get (font-spec :spacing 'm) :spacing)
  (font-get (font-spec :spacing 'c) :spacing)
  (font-get (font-spec :spacing 'P) :spacing)
  (font-get (font-spec :spacing 'D) :spacing)
  (font-get (font-spec :spacing 'M) :spacing)
  (font-get (font-spec :spacing 'C) :spacing)
  (font-get (font-spec :spacing 1) :spacing)
  (font-get (font-spec :spacing 109) :spacing)
  (mapcar (lambda (spacing)
            (list spacing
                  (font-get (font-spec :spacing spacing) :spacing)
                  (font-xlfd-name (font-spec :spacing spacing))))
          '(0 1 89 90 91 99 100 101 109 110))
  (mapcar (lambda (spacing)
            (condition-case err
                (font-spec :spacing spacing)
              (error (list 'error (car err)))))
          '(proportional mono charcell dual pp))
  (condition-case err
      (font-spec :spacing 111)
    (error (list 'error (car err))))) "#,
    );
}

#[test]
fn divergence_face_remapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (make-face 'test-fremap-xxx)
  (set-face-attribute 'test-fremap-xxx nil :foreground "purple")
  (face-remap-add-relative 'test-fremap-xxx 'bold)
  (let ((remaps (face-remap-remove-relative 'test-fremap-xxx 'bold)))
    (list (facep 'test-fremap-xxx)
          (equal (plist-get (face-all-attributes 'test-fremap-xxx (selected-frame)) :foreground)
                 "purple")
          (face-remap-add-relative 'default :height 200)
          (face-remap-remove-relative 'default :height 200)))) "#,
    );
}

#[test]
fn divergence_glyphless_char_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (let ((table (make-glyphless-char-display-table)))
    (set-char-table-range table ?\x00 'thin-space)
    (set-char-table-range table ?\x01 'empty-box)
    (list (char-table-p table)
          (eq (char-table-range table ?\x00) 'thin-space)
          (eq (char-table-range table ?\x01) 'empty-box)
          (char-table-range table ?\x41)
          (null (char-table-range table ?\x41))
          (boundp 'glyphless-char-display-control)
          (listp glyphless-char-display-control)))) "#,
    );
}
