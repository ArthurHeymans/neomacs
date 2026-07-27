use expect_test::expect;

use super::assert_arscript_mode_parity;

#[test]
fn realistic_project_header_fontifies_multiword_fields_tags_values_and_comments() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "// Project metadata\n"
   "<Header>\n"
   "Painting Name: \"Willow\"\n"
   "Painting Width: 2456\n"
   "Painting Height: 2206\n"
   "Painting DPI: 200\n"
   "Mask Edge Map Width: 1280\n"
   "Mask Edge Map Height: 800\n"
   "Author Name: \"Ada\"\n"
   "Script Feature Flags: 0x000000034\n"
   "</Header>\n")
  (arscript-mode)
  (font-lock-ensure)
  (let ((position (point-min))
        runs)
    (while (< position (point-max))
      (let* ((face (get-text-property position 'face))
             (next
              (or
               (next-single-property-change
                position 'face nil (point-max))
               (point-max))))
        (when face
          (push
           (list
            (buffer-substring-no-properties
             position next)
            face)
           runs))
        (setq position next)))
    (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("// Project metadata" font-lock-comment-face) ("<Header>" font-lock-type-face) ("Painting Name" font-lock-keyword-face) ("\"Willow\"" font-lock-string-face) ("Painting Width" font-lock-keyword-face) ("2456" font-lock-constant-face) ("Painting Height" font-lock-keyword-face) ("2206" font-lock-constant-face) ("Painting DPI" font-lock-keyword-face) ("200" font-lock-constant-face) ("Mask Edge Map Width" font-lock-keyword-face) ("1280" font-lock-constant-face) ("Mask Edge Map Height" font-lock-keyword-face) ("800" font-lock-constant-face) ("Author Name" font-lock-keyword-face) ("\"Ada\"" font-lock-string-face) ("Script Feature Flags" font-lock-keyword-face) ("0x000000034" font-lock-string-face) ("</Header>" font-lock-type-face))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn practical_event_commands_fontify_command_fields_and_both_colour_spellings() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "EvType: Command CommandID: CID_SetClearCanvas ParamType: flag Value: { true }\n"
   "EvType: Command CommandID: SetForeColour ParamType: Pixel Value: { 0x0FF7386A0 }\n"
   "EvType: Command CommandID: SetForeColor ParamType: Pixel Value: { 0x0ffcca38f }\n"
   "EvType: Command CommandID: CanvasXForm\n"
   "EvType: Command CommandID: ReferenceImageXForm\n"
   "EvType: Command CommandID: SetToolProperty ParamType: ToolProp\n")
  (arscript-mode)
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (list needle
           (get-text-property
            (match-beginning 0) 'face)))
   '("EvType"
     "Command"
     "CommandID"
     "CID_SetClearCanvas"
     "ParamType"
     "flag"
     "Value"
     "true"
     "SetForeColour"
     "SetForeColor"
     "Pixel"
     "0x0FF7386A0"
     "0x0ffcca38f"
     "CanvasXForm"
     "ReferenceImageXForm"
     "SetToolProperty"
     "ToolProp")))"##;
    let expect = expect![[
        r#"OK (("EvType" font-lock-keyword-face) ("Command" font-lock-constant-face) ("CommandID" font-lock-keyword-face) ("CID_SetClearCanvas" font-lock-constant-face) ("ParamType" font-lock-keyword-face) ("flag" font-lock-constant-face) ("Value" font-lock-keyword-face) ("true" font-lock-constant-face) ("SetForeColour" font-lock-constant-face) ("SetForeColor" font-lock-constant-face) ("Pixel" font-lock-constant-face) ("0x0FF7386A0" font-lock-string-face) ("0x0ffcca38f" font-lock-string-face) ("CanvasXForm" font-lock-constant-face) ("ReferenceImageXForm" font-lock-constant-face) ("SetToolProperty" font-lock-constant-face) ("ToolProp" font-lock-constant-face))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn all_declared_command_constants_are_case_folded_and_require_word_boundaries() {
    let elisp_form = r##"(with-temp-buffer
  (let ((constants
         '("SetForeColour" "SetForeColor" "Pixel" "Command" "Yes" "No"
           "CanvasXForm" "ReferenceImageXForm" "SetMetallicValue"
           "Undo" "SetToolProperty" "ToolProp" "CID_SetClearCanvas"
           "CID_ToolSelect" "CID_SetSpecificToolPressure"
           "CID_ToggleSpecificLayerVisible" "CID_MergeSpecificLayerDown"
           "CID_DuplicateSpecificLayer" "CID_SetSpecificLayerBlend"
           "LayerXForm" "CID_SelectSpecificLayer"
           "CID_SetSpecificLayerOpacity" "LayerProp"
           "ExportLayer" "Canvas Reset All"
           "LoadReferenceImage" "ToolPreset" "flag" "true")))
    (dolist (constant constants)
      (insert constant "\n"))
    (insert
     "prefixSetForeColourSuffix\n"
     "command toolpreset TRUE yes no\n")
    (arscript-mode)
    (font-lock-ensure)
    (list
     (mapcar
      (lambda (constant)
        (goto-char (point-min))
        (search-forward constant)
        (list
         constant
         (get-text-property
          (match-beginning 0) 'face)))
      constants)
     (progn
       (goto-char (point-min))
       (search-forward "prefixSetForeColourSuffix")
       (get-text-property
        (+ (match-beginning 0) 6) 'face))
     (mapcar
      (lambda (variant)
        (goto-char (point-min))
        (search-forward variant)
        (list
         variant
         (get-text-property
          (match-beginning 0) 'face)))
      '("command" "toolpreset" "TRUE" "yes" "no")))))"##;
    let expect = expect![[
        r#"OK ((("SetForeColour" font-lock-constant-face) ("SetForeColor" font-lock-constant-face) ("Pixel" font-lock-constant-face) ("Command" font-lock-constant-face) ("Yes" font-lock-constant-face) ("No" font-lock-constant-face) ("CanvasXForm" font-lock-constant-face) ("ReferenceImageXForm" font-lock-constant-face) ("SetMetallicValue" font-lock-constant-face) ("Undo" font-lock-constant-face) ("SetToolProperty" font-lock-constant-face) ("ToolProp" font-lock-constant-face) ("CID_SetClearCanvas" font-lock-constant-face) ("CID_ToolSelect" font-lock-constant-face) ("CID_SetSpecificToolPressure" font-lock-constant-face) ("CID_ToggleSpecificLayerVisible" font-lock-constant-face) ("CID_MergeSpecificLayerDown" font-lock-constant-face) ("CID_DuplicateSpecificLayer" font-lock-constant-face) ("CID_SetSpecificLayerBlend" font-lock-constant-face) ("LayerXForm" font-lock-constant-face) ("CID_SelectSpecificLayer" font-lock-constant-face) ("CID_SetSpecificLayerOpacity" font-lock-constant-face) ("LayerProp" font-lock-constant-face) ("ExportLayer" font-lock-constant-face) ("Canvas Reset All" font-lock-constant-face) ("LoadReferenceImage" font-lock-constant-face) ("ToolPreset" font-lock-constant-face) ("flag" font-lock-constant-face) ("true" font-lock-constant-face)) nil (("command" font-lock-constant-face) ("toolpreset" font-lock-constant-face) ("TRUE" font-lock-constant-face) ("yes" font-lock-constant-face) ("no" font-lock-constant-face)))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn all_declared_field_keywords_are_case_folded_and_require_word_boundaries() {
    let elisp_form = r##"(with-temp-buffer
  (let ((keywords
         '("EvType" "CommandID" "ParamType" "Value" "Count"
           "Idx" "Channels" "Path" "Script Startup Features"
           "Reference Image" "ArtRage Version" "ArtRage Build"
           "Professional Edition" "Script Version"
           "Painting Name" "Painting Width" "Painting Height"
           "Painting DPI" "Mask Edge Map Width"
           "Mask Edge Map Height" "Author Name" "Script Name"
           "Comment" "Script Type" "Script Feature Flags"
           "Tool Data")))
    (dolist (keyword keywords)
      (insert keyword ":\n"))
    (insert "Painting Names: x\nevtype: Command\n")
    (arscript-mode)
    (font-lock-ensure)
    (list
     (mapcar
      (lambda (keyword)
        (goto-char (point-min))
        (search-forward (concat keyword ":"))
        (list
         keyword
         (get-text-property
          (match-beginning 0) 'face)))
      keywords)
     (progn
       (goto-char (point-min))
       (search-forward "Painting Names")
       (get-text-property
        (match-beginning 0) 'face))
     (progn
       (goto-char (point-min))
       (search-forward "evtype")
       (get-text-property
        (match-beginning 0) 'face)))))"##;
    let expect = expect![[
        r#"OK ((("EvType" font-lock-keyword-face) ("CommandID" font-lock-keyword-face) ("ParamType" font-lock-keyword-face) ("Value" font-lock-keyword-face) ("Count" font-lock-keyword-face) ("Idx" font-lock-keyword-face) ("Channels" font-lock-keyword-face) ("Path" font-lock-keyword-face) ("Script Startup Features" font-lock-keyword-face) ("Reference Image" font-lock-keyword-face) ("ArtRage Version" font-lock-keyword-face) ("ArtRage Build" font-lock-keyword-face) ("Professional Edition" font-lock-keyword-face) ("Script Version" font-lock-keyword-face) ("Painting Name" font-lock-keyword-face) ("Painting Width" font-lock-keyword-face) ("Painting Height" font-lock-keyword-face) ("Painting DPI" font-lock-keyword-face) ("Mask Edge Map Width" font-lock-keyword-face) ("Mask Edge Map Height" font-lock-keyword-face) ("Author Name" font-lock-keyword-face) ("Script Name" font-lock-keyword-face) ("Comment" font-lock-keyword-face) ("Script Type" font-lock-keyword-face) ("Script Feature Flags" font-lock-keyword-face) ("Tool Data" font-lock-keyword-face)) nil font-lock-keyword-face)"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn plain_tags_and_brush_names_fontify_while_attribute_tags_and_plural_words_do_not() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<StrokeEvent id=\"7\">\n"
   "  <StrokeHeader>\n"
   "    <EventPt> Wait: 0.018s </EventPt>\n"
   "  </StrokeHeader>\n"
   "</StrokeEvent>\n"
   "ToolID: 4900 (Oil Brush)\n"
   "ToolID: 4906 (Eraser)\n"
   "ToolID: 4900 (Oil Paint)\n"
   "Oil Brushes Erasers Oil Painter\n")
  (arscript-mode)
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (list
      needle
      (get-text-property
       (match-beginning 0) 'face)))
   '("<StrokeEvent id=\"7\">"
     "<StrokeHeader>"
     "<EventPt>"
     "</EventPt>"
     "</StrokeHeader>"
     "</StrokeEvent>"
     "Oil Brush"
     "Eraser"
     "Oil Paint"
     "Oil Brushes"
     "Erasers"
     "Oil Painter")))"##;
    let expect = expect![[
        r#"OK (("<StrokeEvent id=\"7\">" nil) ("<StrokeHeader>" font-lock-type-face) ("<EventPt>" font-lock-type-face) ("</EventPt>" font-lock-type-face) ("</StrokeHeader>" font-lock-type-face) ("</StrokeEvent>" font-lock-type-face) ("Oil Brush" font-lock-string-face) ("Eraser" font-lock-string-face) ("Oil Paint" font-lock-string-face) ("Oil Brushes" nil) ("Erasers" nil) ("Oil Painter" nil))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn coordinate_patterns_fontify_labels_and_signed_decimal_components_but_not_delimiters() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "Loc: (1086.56, 591.216)\n"
   "Dr: (-0.706138, -0.708074)\n"
   "Hd: (0.708074, -0.706138)\n"
   "Off: (537, 307)\n"
   "Size: (320, 236)\n"
   "Scale: (1.25, -2.5)\n")
  (arscript-mode)
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (let ((start (match-beginning 0)))
       (list
        needle
        (mapcar
         (lambda (offset)
           (list
            offset
            (char-after (+ start offset))
            (get-text-property
             (+ start offset) 'face)))
         (number-sequence
          0 (1- (length needle)))))))
   '("Loc: (1086.56, 591.216)"
     "Dr: (-0.706138, -0.708074)"
     "Hd: (0.708074, -0.706138)"
     "Off: (537, 307)"
     "Size: (320, 236)"
     "Scale: (1.25, -2.5)")))"##;
    let expect = expect![[
        r#"OK (("Loc: (1086.56, 591.216)" ((0 76 font-lock-keyword-face) (1 111 font-lock-keyword-face) (2 99 font-lock-keyword-face) (3 58 font-lock-keyword-face) (4 32 nil) (5 40 nil) (6 49 font-lock-constant-face) (7 48 font-lock-constant-face) (8 56 font-lock-constant-face) (9 54 font-lock-constant-face) (10 46 font-lock-constant-face) (11 53 font-lock-constant-face) (12 54 font-lock-constant-face) (13 44 nil) (14 32 nil) (15 53 font-lock-constant-face) (16 57 font-lock-constant-face) (17 49 font-lock-constant-face) (18 46 font-lock-constant-face) (19 50 font-lock-constant-face) (20 49 font-lock-constant-face) (21 54 font-lock-constant-face) (22 41 nil))) ("Dr: (-0.706138, -0.708074)" ((0 68 font-lock-keyword-face) (1 114 font-lock-keyword-face) (2 58 font-lock-keyword-face) (3 32 nil) (4 40 nil) (5 45 font-lock-constant-face) (6 48 font-lock-constant-face) (7 46 font-lock-constant-face) (8 55 font-lock-constant-face) (9 48 font-lock-constant-face) (10 54 font-lock-constant-face) (11 49 font-lock-constant-face) (12 51 font-lock-constant-face) (13 56 font-lock-constant-face) (14 44 nil) (15 32 nil) (16 45 font-lock-constant-face) (17 48 font-lock-constant-face) (18 46 font-lock-constant-face) (19 55 font-lock-constant-face) (20 48 font-lock-constant-face) (21 56 font-lock-constant-face) (22 48 font-lock-constant-face) (23 55 font-lock-constant-face) (24 52 font-lock-constant-face) (25 41 nil))) ("Hd: (0.708074, -0.706138)" ((0 72 font-lock-keyword-face) (1 100 font-lock-keyword-face) (2 58 font-lock-keyword-face) (3 32 nil) (4 40 nil) (5 48 font-lock-constant-face) (6 46 font-lock-constant-face) (7 55 font-lock-constant-face) (8 48 font-lock-constant-face) (9 56 font-lock-constant-face) (10 48 font-lock-constant-face) (11 55 font-lock-constant-face) (12 52 font-lock-constant-face) (13 44 nil) (14 32 nil) (15 45 font-lock-constant-face) (16 48 font-lock-constant-face) (17 46 font-lock-constant-face) (18 55 font-lock-constant-face) (19 48 font-lock-constant-face) (20 54 font-lock-constant-face) (21 49 font-lock-constant-face) (22 51 font-lock-constant-face) (23 56 font-lock-constant-face) (24 41 nil))) ("Off: (537, 307)" ((0 79 font-lock-keyword-face) (1 102 font-lock-keyword-face) (2 102 font-lock-keyword-face) (3 58 font-lock-keyword-face) (4 32 nil) (5 40 nil) (6 53 font-lock-constant-face) (7 51 font-lock-constant-face) (8 55 font-lock-constant-face) (9 44 nil) (10 32 nil) (11 51 font-lock-constant-face) (12 48 font-lock-constant-face) (13 55 font-lock-constant-face) (14 41 nil))) ("Size: (320, 236)" ((0 83 font-lock-keyword-face) (1 105 font-lock-keyword-face) (2 122 font-lock-keyword-face) (3 101 font-lock-keyword-face) (4 58 font-lock-keyword-face) (5 32 nil) (6 40 nil) (7 51 font-lock-constant-face) (8 50 font-lock-constant-face) (9 48 font-lock-constant-face) (10 44 nil) (11 32 nil) (12 50 font-lock-constant-face) (13 51 font-lock-constant-face) (14 54 font-lock-constant-face) (15 41 nil))) ("Scale: (1.25, -2.5)" ((0 83 font-lock-keyword-face) (1 99 font-lock-keyword-face) (2 97 font-lock-keyword-face) (3 108 font-lock-keyword-face) (4 101 font-lock-keyword-face) (5 58 font-lock-keyword-face) (6 32 nil) (7 40 nil) (8 49 font-lock-constant-face) (9 46 font-lock-constant-face) (10 50 font-lock-constant-face) (11 53 font-lock-constant-face) (12 44 nil) (13 32 nil) (14 45 font-lock-constant-face) (15 50 font-lock-constant-face) (16 46 font-lock-constant-face) (17 53 font-lock-constant-face) (18 41 nil))))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn reverse_and_inversion_flags_greedily_fontify_the_remainder_of_each_event_line() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "Rv: NO\tIv: NO\n"
   "Rv: YES\tIv: NO\tPr: 0.25\n"
   "Iv: YES trailing words\n")
  (arscript-mode)
  (font-lock-ensure)
  (let ((position (point-min))
        runs)
    (while (< position (point-max))
      (let* ((face (get-text-property position 'face))
             (next
              (or
               (next-single-property-change
                position 'face nil (point-max))
               (point-max))))
        (when face
          (push
           (list
            (buffer-substring-no-properties
             position next)
            face)
           runs))
        (setq position next)))
    (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("Rv:" font-lock-string-face) ("NO\11Iv: NO" font-lock-constant-face) ("Rv:" font-lock-string-face) ("YES\11Iv: NO\11Pr: 0.25" font-lock-constant-face) ("Iv:" font-lock-string-face) ("YES trailing words" font-lock-constant-face))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn standalone_wait_and_scalar_fields_accept_spaces_but_not_tabs_or_literal_s_runs() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "Wait: 0.018s\n"
   "Pr: 0.237271\n"
   "Ti: 0.519182\n"
   "Ro: 1.25776\n"
   "Fw: 1\n"
   "Bt: 0\n"
   "Wait:\t0.018s\n"
   "Pr:\t0.237271\n"
   "Wait:ss0.018s\n"
   "Pr:ss0.237271\n")
  (arscript-mode)
  (font-lock-ensure)
  (mapcar
   (lambda (line)
     (goto-char (point-min))
     (forward-line line)
     (let ((start (point)))
       (list
        line
        (buffer-substring-no-properties
         start (line-end-position))
        (get-text-property start 'face)
        (get-text-property
         (max start (1- (line-end-position)))
         'face))))
   (number-sequence 0 9)))"##;
    let expect = expect![[
        r#"OK ((0 "Wait: 0.018s" font-lock-string-face font-lock-constant-face) (1 "Pr: 0.237271" font-lock-keyword-face font-lock-constant-face) (2 "Ti: 0.519182" font-lock-keyword-face font-lock-constant-face) (3 "Ro: 1.25776" font-lock-keyword-face font-lock-constant-face) (4 "Fw: 1" font-lock-keyword-face font-lock-constant-face) (5 "Bt: 0" font-lock-keyword-face font-lock-constant-face) (6 "Wait:\0110.018s" nil nil) (7 "Pr:\0110.237271" nil nil) (8 "Wait:ss0.018s" nil nil) (9 "Pr:ss0.237271" nil nil))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn hexadecimal_values_accept_mixed_case_digits_and_stop_at_nonhex_boundaries() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "0x0FF7386A0 0x0ffcca38f 0xDeadBEEF\n"
   "0x 0xG123 prefix0xABCsuffix 0XABC\n")
  (arscript-mode)
  (font-lock-ensure)
  (let ((position (point-min))
        runs)
    (while (< position (point-max))
      (let* ((face (get-text-property position 'face))
             (next
              (or
               (next-single-property-change
                position 'face nil (point-max))
               (point-max))))
        (when face
          (push
           (list
            (buffer-substring-no-properties
             position next)
            face)
           runs))
        (setq position next)))
    (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("0x0FF7386A0" font-lock-string-face) ("0x0ffcca38f" font-lock-string-face) ("0xDeadBEEF" font-lock-string-face) ("0xABC" font-lock-string-face))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn editing_an_event_then_refontifying_updates_the_precise_semantic_face_runs() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "EvType: Command CommandID: Undo\n"
   "Loc: (10, 20)\n")
  (arscript-mode)
  (font-lock-ensure)
  (goto-char (point-min))
  (search-forward "Undo")
  (replace-match "ExportLayer")
  (goto-char (point-max))
  (insert "ToolID: 4900 (Oil Brush)\n")
  (font-lock-flush)
  (font-lock-ensure)
  (let ((position (point-min))
        runs)
    (while (< position (point-max))
      (let* ((face (get-text-property position 'face))
             (next
              (or
               (next-single-property-change
                position 'face nil (point-max))
               (point-max))))
        (when face
          (push
           (list
            (buffer-substring-no-properties
             position next)
            face)
           runs))
        (setq position next)))
    (list
     (buffer-string)
     (nreverse runs))))"##;
    let expect = expect![[
        r#"OK (#("EvType: Command CommandID: ExportLayer\nLoc: (10, 20)\nToolID: 4900 (Oil Brush)\n" 0 6 (face font-lock-keyword-face) 8 15 (face font-lock-constant-face) 16 25 (face font-lock-keyword-face) 27 38 (face font-lock-constant-face) 39 43 (face font-lock-keyword-face) 45 47 (face font-lock-constant-face) 49 51 (face font-lock-constant-face) 53 60 (face font-lock-keyword-face) 61 65 (face font-lock-constant-face) 67 76 (face font-lock-string-face)) (("EvType" font-lock-keyword-face) ("Command" font-lock-constant-face) ("CommandID" font-lock-keyword-face) ("ExportLayer" font-lock-constant-face) ("Loc:" font-lock-keyword-face) ("10" font-lock-constant-face) ("20" font-lock-constant-face) ("ToolID:" font-lock-keyword-face) ("4900" font-lock-constant-face) ("Oil Brush" font-lock-string-face)))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}
