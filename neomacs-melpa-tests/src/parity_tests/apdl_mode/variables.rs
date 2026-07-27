use expect_test::expect;

use super::assert_apdl_mode_parity;

#[test]
fn variable_discovery_finds_assignments_and_command_outputs_in_source_order() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "length = 120\n"
   "width = length / 3\n"
   "*dim,stress,array,10\n"
   "*get,node_count,node,0,count\n"
   "cm,support_nodes,node\n"
   "*do,index,1,node_count\n"
   "  force_index = index * 100\n"
   "*enddo\n")
  (apdl-find-user-variables)
  (list apdl-user-variables
        apdl-user-variable-regexp
        (mapcar
         (lambda (name)
           (and (string-match-p apdl-user-variable-regexp name) t))
         '("length" "WIDTH" "stress" "node_count" "support_nodes"
           "index" "force_index" "missing"))))"##;
    let expect = expect![[
        r#"OK ((("length" 1) ("width" 2) ("stress" 3) ("node_count" 4) ("support_nodes" 5) ("index" 6) ("force_index" 7)) "\\_<\\(force_index\\|index\\|length\\|node_count\\|s\\(?:\\(?:tres\\|upport_node\\)s\\)\\|width\\)\\_>" (t t t t t t t nil))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn variable_discovery_rejects_comments_strings_formats_duplicates_and_invalid_names() {
    let elisp_form = r##"(with-temp-buffer
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "! ignored_comment = 1\n"
   "/title,ignored_title = 2\n"
   "*msg,info\n"
   "ignored_format = %g\n"
   "valid_name = 3\n"
   "VALID_NAME = 4\n"
   "_reserved = 5\n"
   "thirty_two_character_variable_123 = 6\n"
   "too_long_variable_name_over_32_chars = 7\n"
   "*dim,table_values,table,5\n"
   "*dim,TABLE_VALUES,array,5\n")
  (apdl-find-user-variables)
  (list apdl-user-variables apdl-user-variable-regexp))"##;
    let expect = expect![[
        r#"OK ((("valid_name" 5) ("_reserved" 7) ("table_values" 10)) "\\_<\\(_reserved\\|table_values\\|valid_name\\)\\_>")"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn dynamic_variable_hooks_refresh_the_regexp_after_practical_buffer_edits() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (apdl-add-variable-hooks)
    (insert "radius = 10\narea = acos(-1) * radius ** 2\n")
    (let ((first
           (list apdl-user-variables apdl-user-variable-regexp
                 (memq #'apdl-find-user-variables
                       after-change-functions)
                 (memq #'apdl-update-parameter-help
                       post-command-hook))))
      (goto-char (point-min))
      (insert "*get,node_total,node,0,count\n")
      (list first apdl-user-variables apdl-user-variable-regexp))))"##;
    let expect = expect![[
        r#"OK (((("radius" 1) ("area" 2)) "\\_<\\(area\\|radius\\)\\_>" (apdl-find-user-variables t) (apdl-update-parameter-help t)) (("node_total" 1) ("radius" 2) ("area" 3)) "\\_<\\(area\\|node_total\\|radius\\)\\_>")"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn variable_display_builds_a_read_only_navigable_definition_report() {
    let elisp_form = r##"(with-temp-buffer
  (rename-buffer "production-model.mac" t)
  (set-syntax-table apdl-mode-syntax-table)
  (insert
   "length = 120\n"
   "width = length / 3\n"
   "*dim,stress,array,10\n"
   "stress(1) = 42\n")
  (let (displayed)
    (cl-letf (((symbol-function 'display-buffer)
               (lambda (buffer-or-name &optional _action)
                 (setq displayed
                       (if (bufferp buffer-or-name)
                           (buffer-name buffer-or-name)
                         buffer-or-name))
                 (get-buffer buffer-or-name))))
      (apdl-display-variables nil)
      (with-current-buffer "*APDL-variables*"
        (let ((button (next-button (point-min) t))
              buttons)
          (while button
            (push
             (list
              (button-label button)
              (marker-position (button-get button 'action))
              (buffer-name
               (marker-buffer (button-get button 'action))))
             buttons)
            (setq button (next-button (button-end button))))
          (list
           displayed
           buffer-read-only
           (buffer-substring-no-properties (point-min) (point-max))
           (nreverse buttons)))))))"##;
    let expect = expect![[
        r#"OK ("*APDL-variables*" t "-*- APDL variables of production-model.mac click with mouse-2 -*-\n Line | Definition\n    1 | length = 120\n    2 | width = length / 3\n    3 | *dim,stress,array,10\n" (("1" 1 "production-model.mac") ("2" 14 "production-model.mac") ("3" 33 "production-model.mac")))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn helper_functions_preserve_case_insensitive_duplicates_sorting_and_line_access() {
    let elisp_form = r##"(let ((buffer (get-buffer-create " *apdl-lines*")))
  (unwind-protect
      (progn
        (with-current-buffer buffer
          (erase-buffer)
          (insert "  first = 1\nsecond = 2\nthird = 3\n"))
        (list
         (apdl-asterisk-regexp "*get")
         (apdl-asterisk-regexp "nsel")
         (apdl-string-length-predicate "longer" "tiny")
         (apdl-string-length-predicate "x" "long")
         (apdl-find-duplicate-p
          "WIDTH" '(("length" 1) ("width" 2)))
         (apdl-find-duplicate-p
          "height" '(("length" 1) ("width" 2)))
         (apdl-copy-buffer-line buffer 1)
         (apdl-copy-buffer-line buffer 3)
         (let ((marker (apdl-buffer-line-marker buffer 2)))
           (list (marker-position marker)
                 (buffer-name (marker-buffer marker))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ("\\*get" "nsel" nil t "width" nil "first = 1" "third = 3" (13 " *apdl-lines*"))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}
