use expect_test::expect;

use super::assert_arscript_mode_parity;

#[test]
fn comment_and_uncomment_region_round_trip_a_real_command_block_exactly() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FF7386A0 }\n"
   "EvType: Command CommandID: CID_SetClearCanvas ParamType: flag Value: { true }\n")
  (arscript-mode)
  (let ((original (buffer-string)))
    (comment-region (point-min) (point-max))
    (let ((commented (buffer-string)))
      (uncomment-region (point-min) (point-max))
      (list
       original
       commented
       (buffer-string)
       (equal original (buffer-string))))))"##;
    let expect = expect![[
        r#"OK ("EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FF7386A0 }\nEvType: Command CommandID: CID_SetClearCanvas ParamType: flag Value: { true }\n" "// EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FF7386A0 }\n// EvType: Command CommandID: CID_SetClearCanvas ParamType: flag Value: { true }\n" "EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FF7386A0 }\nEvType: Command CommandID: CID_SetClearCanvas ParamType: flag Value: { true }\n" t)"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn comment_region_handles_indented_blank_and_already_commented_lines_as_editor_commands_do() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "  Painting Name: \"Willow\"\n"
   "\n"
   "    // existing note\n"
   "  Painting Width: 2456\n")
  (arscript-mode)
  (comment-region (point-min) (point-max))
  (let ((once (buffer-string)))
    (comment-region (point-min) (point-max) 2)
    (list
     once
     (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("  // Painting Name: \"Willow\"\n\n  //   // existing note\n  // Painting Width: 2456\n" "  /// // Painting Name: \"Willow\"\n\n  /// //   // existing note\n  /// // Painting Width: 2456\n")"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn narrowing_indents_only_the_selected_nested_stroke_without_touching_siblings() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Events>\n"
   "<StrokeEvent>\n"
   "<StrokeHeader>\n"
   "Wait: 0.018s Loc: (10, 20)\n"
   "</StrokeHeader>\n"
   "</StrokeEvent>\n"
   "<StrokeEvent>\n"
   "untouched sibling\n"
   "</StrokeEvent>\n"
   "</Events>\n")
  (arscript-mode)
  (setq-local tab-width 2)
  (goto-char (point-min))
  (forward-line)
  (let ((start (point)))
    (forward-line 5)
    (save-restriction
      (narrow-to-region start (point))
      (indent-region (point-min) (point-max)))
    (list
     start
     (point)
     (buffer-string))))"##;
    let expect = expect![[
        r#"OK (10 115 "<Events>\n  <StrokeEvent>\n    <StrokeHeader>\n      Wait: 0.018s Loc: (10, 20)\n    </StrokeHeader>\n  </StrokeEvent>\n<StrokeEvent>\nuntouched sibling\n</StrokeEvent>\n</Events>\n")"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn representative_art_script_edit_combines_indentation_commenting_and_refontification() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Events>\n"
   "EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FFCCA38F }\n"
   "<StrokeEvent>\n"
   "<StrokeHeader>\n"
   "<EventPt> Wait: 0.018s Loc: (1086.56, 559.258) Pr: 0.156599 Rv: NO Iv: NO </EventPt>\n"
   "</StrokeHeader>\n"
   "</StrokeEvent>\n"
   "</Events>\n")
  (arscript-mode)
  (setq-local tab-width 2)
  (indent-region (point-min) (point-max))
  (goto-char (point-min))
  (forward-line)
  (comment-region
   (line-beginning-position)
   (line-beginning-position 2))
  (font-lock-ensure)
  (let ((needles
         '("<Events>"
           "// EvType"
           "EvType"
           "SetForeColour"
           "0x0FFCCA38F"
           "<StrokeEvent>"
           "<EventPt>"
           "Loc:"
           "1086.56"
           "Rv:"
           "NO"
           "</Events>")))
    (list
     (buffer-string)
     (mapcar
      (lambda (needle)
        (goto-char (point-min))
        (search-forward needle)
        (list
         needle
         (line-number-at-pos)
         (current-indentation)
         (get-text-property
          (match-beginning 0) 'face)))
      needles))))"##;
    let expect = expect![[
        r#"OK (#("<Events>\n  // EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FFCCA38F }\n  <StrokeEvent>\n    <StrokeHeader>\n      <EventPt> Wait: 0.018s Loc: (1086.56, 559.258) Pr: 0.156599 Rv: NO Iv: NO </EventPt>\n    </StrokeHeader>\n  </StrokeEvent>\n</Events>\n" 0 8 (face font-lock-type-face) 11 94 (face font-lock-comment-face) 97 110 (face font-lock-type-face) 115 129 (face font-lock-type-face) 136 145 (face font-lock-type-face) 146 151 (face font-lock-string-face) 152 158 (face font-lock-constant-face) 159 163 (face font-lock-keyword-face) 165 172 (face font-lock-constant-face) 174 181 (face font-lock-constant-face) 183 186 (face font-lock-keyword-face) 187 195 (face font-lock-constant-face) 196 199 (face font-lock-string-face) 210 220 (face font-lock-type-face) 225 240 (face font-lock-type-face) 243 257 (face font-lock-type-face) 258 267 (face font-lock-type-face)) (("<Events>" 1 0 font-lock-type-face) ("// EvType" 2 2 font-lock-comment-face) ("EvType" 2 2 font-lock-comment-face) ("SetForeColour" 2 2 font-lock-comment-face) ("0x0FFCCA38F" 2 2 font-lock-comment-face) ("<StrokeEvent>" 3 2 font-lock-type-face) ("<EventPt>" 5 6 font-lock-type-face) ("Loc:" 5 6 font-lock-keyword-face) ("1086.56" 5 6 font-lock-constant-face) ("Rv:" 5 6 font-lock-string-face) ("NO" 5 6 nil) ("</Events>" 8 0 font-lock-type-face)))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn switching_from_another_programming_mode_preserves_text_but_replaces_local_editing_state() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Header>\n"
   "Painting Name: \"Willow\"\n"
   "</Header>\n")
  (emacs-lisp-mode)
  (setq-local indent-tabs-mode t)
  (setq-local comment-start ";;;")
  (let ((before (buffer-string))
        (before-mode major-mode)
        (before-syntax (syntax-table)))
    (arscript-mode)
    (list
     before-mode
     major-mode
     (equal before (buffer-string))
     (eq before-syntax (syntax-table))
     indent-tabs-mode
     comment-start
     (eq indent-line-function
         #'arscript-indent-line))))"##;
    let expect = expect![[r#"OK (emacs-lisp-mode fundamental-mode t nil nil "//" t)"#]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn mode_activation_is_nonmutating_but_indent_region_marks_changed_content_modified() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Version>\n"
   "    ArtRage Version: ArtRage 3 4\n"
   "</Version>\n")
  (set-buffer-modified-p nil)
  (arscript-mode)
  (let ((after-mode (buffer-modified-p)))
    (indent-region (point-min) (point-max))
    (list
     after-mode
     (buffer-modified-p)
     (buffer-string))))"##;
    let expect =
        expect![[r#"OK (nil t "<Version>\n        ArtRage Version: ArtRage 3 4\n</Version>\n")"#]];
    assert_arscript_mode_parity(elisp_form, expect);
}
