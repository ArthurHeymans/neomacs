use expect_test::expect;

use super::assert_all_the_icons_ibuffer_parity;

#[test]
fn all_the_icons_ibuffer_loads_with_real_dependency_and_registers_ibuffer_columns() {
    let elisp_form = r##"(list
 (featurep 'all-the-icons)
 (featurep 'all-the-icons-ibuffer)
 (mapcar
  (lambda (column)
    (let ((function (intern (format "ibuffer-make-column-%s" column))))
      (list column
            (cond
             ((functionp function) 'function)
             ((assq function ibuffer-inline-columns) 'inline)
             (t 'missing))
            (get function 'ibuffer-column-name)
            (functionp (get function 'ibuffer-column-summarizer)))))
  '(icon size-h mode+ filename-and-process+))
 (list
  (package-installed-p 'all-the-icons)
  (package-installed-p 'all-the-icons-ibuffer)
  (package-version-join
   (package-desc-version
    (cadr (assq 'all-the-icons package-alist))))
  (package-version-join
   (package-desc-version
    (cadr (assq 'all-the-icons-ibuffer package-alist))))))"##;
    let expect = expect![[
        r#"OK (t t ((icon inline "" nil) (size-h inline "Size" t) (mode+ inline "Mode" nil) (filename-and-process+ function "Filename/Process" t)) (t t "20250527.927" "20230503.1625"))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ibuffer_defaults_describe_the_real_ibuffer_layout() {
    let elisp_form = r##"(list
 all-the-icons-ibuffer-icon
 all-the-icons-ibuffer-color-icon
 all-the-icons-ibuffer-icon-size
 all-the-icons-ibuffer-icon-v-adjust
 all-the-icons-ibuffer-human-readable-size
 all-the-icons-ibuffer-display-predicate
 all-the-icons-ibuffer-formats)"##;
    let expect = expect![[
        r#"OK (t t 1.0 0.0 t display-graphic-p ((mark modified read-only locked " " (icon 2 2) (name 18 18 :left :elide) " " (size-h 9 -1 :right) " " (mode+ 16 16 :left :elide) " " filename-and-process+) (mark " " (name 16 -1) " " filename)))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ibuffer_custom_options_retain_types_groups_and_safe_values() {
    let elisp_form = r##"(mapcar
 (lambda (variable)
   (list variable
         (get variable 'custom-type)
         (get variable 'custom-group)
         (default-boundp variable)
         (default-value variable)))
 '(all-the-icons-ibuffer-display-predicate
   all-the-icons-ibuffer-icon
   all-the-icons-ibuffer-color-icon
   all-the-icons-ibuffer-icon-size
   all-the-icons-ibuffer-icon-v-adjust
   all-the-icons-ibuffer-human-readable-size))"##;
    let expect = expect![
        "OK ((all-the-icons-ibuffer-display-predicate boolean nil t display-graphic-p) (all-the-icons-ibuffer-icon boolean nil t t) (all-the-icons-ibuffer-color-icon boolean nil t t) (all-the-icons-ibuffer-icon-size float nil t 1.0) (all-the-icons-ibuffer-icon-v-adjust float nil t 0.0) (all-the-icons-ibuffer-human-readable-size boolean nil t t))"
    ];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ibuffer_column_metadata_drives_headers_faces_and_summaries() {
    let elisp_form = r##"(mapcar
 (lambda (name)
   (let ((column (intern (format "ibuffer-make-column-%s" name))))
     (list name
           (get column 'ibuffer-column-name)
           (get column 'header-mouse-map)
           (functionp (get column 'ibuffer-column-summarizer))
           (get column 'ibuffer-column-summary))))
 '(icon size-h mode+ filename-and-process+))"##;
    let expect = expect![[
        r#"OK ((icon "" nil nil nil) (size-h "Size" (keymap (mouse-1 . ibuffer-do-sort-by-size)) t nil) (mode+ "Mode" (keymap (mouse-1 . ibuffer-do-sort-by-major-mode)) nil nil) (filename-and-process+ "Filename/Process" (keymap (mouse-1 . ibuffer-do-sort-by-filename/process)) t nil))"#
    ]];
    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}
