use expect_test::expect;

use super::assert_arscript_mode_parity;

#[test]
fn nested_real_header_and_stroke_records_indent_as_a_complete_practical_document() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Events>\n"
   "<StrokeEvent>\n"
   "<StrokeHeader>\n"
   "<EventPt>\n"
   "Wait: 0.018s Loc: (10, 20)\n"
   "</EventPt>\n"
   "<Recorded> Yes </Recorded>\n"
   "</StrokeHeader>\n"
   "</StrokeEvent>\n"
   "</Events>\n")
  (arscript-mode)
  (setq-local tab-width 3)
  (indent-region (point-min) (point-max))
  (list
   (buffer-string)
   (mapcar
    (lambda (line)
      (goto-char (point-min))
      (forward-line (1- line))
      (current-indentation))
    (number-sequence 1 10))))"##;
    let expect = expect![[
        r#"OK ("<Events>\n   <StrokeEvent>\n      <StrokeHeader>\n         <EventPt>\n            Wait: 0.018s Loc: (10, 20)\n         </EventPt>\n         <Recorded> Yes </Recorded>\n      </StrokeHeader>\n   </StrokeEvent>\n</Events>\n" (0 3 6 9 12 9 9 6 3 0))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn first_line_is_always_flushed_to_column_zero_and_point_tracks_removed_indent() {
    let elisp_form = r##"(with-temp-buffer
  (insert "        <Version>\n")
  (arscript-mode)
  (goto-char (point-min))
  (forward-char 11)
  (let ((before (point)))
    (arscript-indent-line)
    (list
     before
     (point)
     (current-indentation)
     (buffer-string))))"##;
    let expect = expect![[r#"OK (12 1 0 "<Version>\n")"#]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn closing_tag_uses_only_the_previous_line_indent_minus_tab_width_and_clamps_zero() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (insert (car case) "\n        </Header>\n")
     (arscript-mode)
     (setq-local tab-width (cdr case))
     (goto-char (point-min))
     (forward-line)
     (arscript-indent-line)
     (list
      case
      (current-indentation)
      (buffer-string))))
 '(("      Painting Name: \"Willow\"" . 4)
   ("  Painting Name: \"Willow\"" . 4)
   ("          Painting Name: \"Willow\"" . 3)))"##;
    let expect = expect![[
        r#"OK ((("      Painting Name: \"Willow\"" . 4) 2 "      Painting Name: \"Willow\"\n  </Header>\n") (("  Painting Name: \"Willow\"" . 4) 0 "  Painting Name: \"Willow\"\n</Header>\n") (("          Painting Name: \"Willow\"" . 3) 7 "          Painting Name: \"Willow\"\n       </Header>\n"))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn line_after_an_opening_tag_adds_exactly_one_configured_tab_width() {
    let elisp_form = r##"(mapcar
 (lambda (width)
   (with-temp-buffer
     (insert "  <Header>\nPainting Name: \"Willow\"\n")
     (arscript-mode)
     (setq-local tab-width width)
     (goto-char (point-min))
     (forward-line)
     (arscript-indent-line)
     (list
      width
      (current-indentation)
      (buffer-string))))
 '(2 4 7))"##;
    let expect = expect![[
        r#"OK ((2 4 "  <Header>\n    Painting Name: \"Willow\"\n") (4 6 "  <Header>\n      Painting Name: \"Willow\"\n") (7 9 "  <Header>\n         Painting Name: \"Willow\"\n"))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn backward_scan_crosses_blank_comments_and_plain_fields_to_find_nearest_tag() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Header>\n"
   "  Painting Name: \"Willow\"\n"
   "\n"
   "    // metadata remains inside header\n"
   "Painting Width: 2456\n")
  (arscript-mode)
  (setq-local tab-width 4)
  (goto-char (point-max))
  (forward-line -1)
  (arscript-indent-line)
  (list
   (current-indentation)
   (line-number-at-pos)
   (buffer-string)))"##;
    let expect = expect![[
        r#"OK (4 5 "<Header>\n  Painting Name: \"Willow\"\n\n    // metadata remains inside header\n    Painting Width: 2456\n")"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn line_after_a_closing_tag_reuses_that_line_indentation_for_the_next_sibling() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "    </Header>\n"
   "            <StartupFeatures>\n")
  (arscript-mode)
  (setq-local tab-width 4)
  (goto-char (point-min))
  (forward-line)
  (arscript-indent-line)
  (list
   (current-indentation)
   (buffer-string)))"##;
    let expect = expect![[r#"OK (4 "    </Header>\n    <StartupFeatures>\n")"#]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn inline_opening_and_closing_tags_control_following_lines_despite_surrounding_text() {
    let elisp_form = r##"(mapcar
 (lambda (previous)
   (with-temp-buffer
     (insert "      " previous "\nnext record\n")
     (arscript-mode)
     (setq-local tab-width 4)
     (goto-char (point-min))
     (forward-line)
     (arscript-indent-line)
     (list
      previous
      (current-indentation)
      (buffer-string))))
 '("prefix <StrokeEvent> suffix"
   "payload </StrokeEvent> trailing"
   "<Recorded> Yes </Recorded>"))"##;
    let expect = expect![[
        r#"OK (("prefix <StrokeEvent> suffix" 10 "      prefix <StrokeEvent> suffix\n          next record\n") ("payload </StrokeEvent> trailing" 6 "      payload </StrokeEvent> trailing\n      next record\n") ("<Recorded> Yes </Recorded>" 6 "      <Recorded> Yes </Recorded>\n      next record\n"))"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn self_closing_tags_are_treated_as_openers_and_indent_the_following_record() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<ReferenceImage path=\"willow.png\" />\n"
   "EvType: Command CommandID: LoadReferenceImage\n")
  (arscript-mode)
  (setq-local tab-width 5)
  (goto-char (point-min))
  (forward-line)
  (arscript-indent-line)
  (list
   (current-indentation)
   (buffer-string)))"##;
    let expect = expect![[
        r#"OK (5 "<ReferenceImage path=\"willow.png\" />\n     EvType: Command CommandID: LoadReferenceImage\n")"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn buffers_without_any_prior_tag_preserve_existing_indentation_instead_of_inventing_structure() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "Painting Name: \"Willow\"\n"
   "       Painting Width: 2456\n"
   "   Painting Height: 2206\n")
  (arscript-mode)
  (let (results)
    (dolist (line '(2 3))
      (goto-char (point-min))
      (forward-line (1- line))
      (let ((before (current-indentation)))
        (arscript-indent-line)
        (push
         (list
          line before
          (current-indentation)
          (point))
         results)))
    (list
     (nreverse results)
     (buffer-string))))"##;
    let expect = expect![[
        r#"OK (((2 7 7 32) (3 3 3 56)) "Painting Name: \"Willow\"\n       Painting Width: 2456\n   Painting Height: 2206\n")"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn repeated_indentation_is_idempotent_for_a_mixed_realistic_event_buffer() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "<Events>\n"
   " <StrokeEvent>\n"
   "      <StrokeHeader>\n"
   "Wait: 0.000s Loc: (1086.56, 559.258)\n"
   " </StrokeHeader>\n"
   "</StrokeEvent>\n"
   "EvType: Command CommandID: Undo\n"
   "</Events>\n")
  (arscript-mode)
  (setq-local tab-width 4)
  (indent-region (point-min) (point-max))
  (let ((once (buffer-string)))
    (indent-region (point-min) (point-max))
    (list
     once
     (buffer-string)
     (equal once (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("<Events>\n    <StrokeEvent>\n        <StrokeHeader>\n            Wait: 0.000s Loc: (1086.56, 559.258)\n        </StrokeHeader>\n    </StrokeEvent>\n    EvType: Command CommandID: Undo\n</Events>\n" "<Events>\n    <StrokeEvent>\n        <StrokeHeader>\n            Wait: 0.000s Loc: (1086.56, 559.258)\n        </StrokeHeader>\n    </StrokeEvent>\n    EvType: Command CommandID: Undo\n</Events>\n" t)"#
    ]];
    assert_arscript_mode_parity(elisp_form, expect);
}

#[test]
fn newline_and_indent_uses_the_mode_indenter_in_a_live_tag_editing_workflow() {
    let elisp_form = r##"(with-temp-buffer
  (arscript-mode)
  (setq-local tab-width 2)
  (insert "<Header>")
  (newline-and-indent)
  (insert "Painting Name: \"Willow\"")
  (newline-and-indent)
  (insert "</Header>")
  (arscript-indent-line)
  (list
   (buffer-string)
   (point)
   (current-indentation)))"##;
    let expect = expect![[r#"OK ("<Header>\n  Painting Name: \"Willow\"\n</Header>" 36 0)"#]];
    assert_arscript_mode_parity(elisp_form, expect);
}
