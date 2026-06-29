//! Per-face *face-id* matrix (all GNU faces).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_id_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'abbrev-table-name)",
        expect_test::expect![[r#""OK 68""#]],
    );
}

#[test]
fn div_face_id_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'blink-matching-paren-offscreen)",
        expect_test::expect![[r#""OK 74""#]],
    );
}

#[test]
fn div_face_id_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'bold)",
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_face_id_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'bold-italic)",
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_face_id_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'border)",
        expect_test::expect![[r#""OK 45""#]],
    );
}

#[test]
fn div_face_id_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'buffer-menu-buffer)",
        expect_test::expect![[r#""OK 126""#]],
    );
}

#[test]
fn div_face_id_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'button)",
        expect_test::expect![[r#""OK 67""#]],
    );
}

#[test]
fn div_face_id_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'child-frame-border)",
        expect_test::expect![[r#""OK 40""#]],
    );
}

#[test]
fn div_face_id_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'completions-annotations)",
        expect_test::expect![[r#""OK 77""#]],
    );
}

#[test]
fn div_face_id_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'completions-common-part)",
        expect_test::expect![[r#""OK 80""#]],
    );
}

#[test]
fn div_face_id_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'completions-first-difference)",
        expect_test::expect![[r#""OK 79""#]],
    );
}

#[test]
fn div_face_id_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'completions-group-separator)",
        expect_test::expect![[r#""OK 76""#]],
    );
}

#[test]
fn div_face_id_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'completions-group-title)",
        expect_test::expect![[r#""OK 75""#]],
    );
}

#[test]
fn div_face_id_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'completions-highlight)",
        expect_test::expect![[r#""OK 78""#]],
    );
}

#[test]
fn div_face_id_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'confusingly-reordered)",
        expect_test::expect![[r#""OK 70""#]],
    );
}

#[test]
fn div_face_id_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'cursor)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_face_id_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'default)",
        expect_test::expect![[r#""OK 0""#]],
    );
}

#[test]
fn div_face_id_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'eldoc-highlight-function-argument)",
        expect_test::expect![[r#""OK 184""#]],
    );
}

#[test]
fn div_face_id_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-ampersand)",
        expect_test::expect![[r#""OK 157""#]],
    );
}

#[test]
fn div_face_id_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-binding-variable)",
        expect_test::expect![[r#""OK 144""#]],
    );
}

#[test]
fn div_face_id_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-bound-variable)",
        expect_test::expect![[r#""OK 145""#]],
    );
}

#[test]
fn div_face_id_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-charset)",
        expect_test::expect![[r#""OK 169""#]],
    );
}

#[test]
fn div_face_id_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-coding)",
        expect_test::expect![[r#""OK 167""#]],
    );
}

#[test]
fn div_face_id_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-completion-category)",
        expect_test::expect![[r#""OK 171""#]],
    );
}

#[test]
fn div_face_id_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-completion-category-definition)",
        expect_test::expect![[r#""OK 172""#]],
    );
}

#[test]
fn div_face_id_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-condition)",
        expect_test::expect![[r#""OK 130""#]],
    );
}

#[test]
fn div_face_id_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-constant)",
        expect_test::expect![[r#""OK 158""#]],
    );
}

#[test]
fn div_face_id_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defcharset)",
        expect_test::expect![[r#""OK 170""#]],
    );
}

#[test]
fn div_face_id_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defcoding)",
        expect_test::expect![[r#""OK 168""#]],
    );
}

#[test]
fn div_face_id_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defface)",
        expect_test::expect![[r#""OK 162""#]],
    );
}

#[test]
fn div_face_id_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-deficon)",
        expect_test::expect![[r#""OK 164""#]],
    );
}

#[test]
fn div_face_id_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defmacro)",
        expect_test::expect![[r#""OK 160""#]],
    );
}

#[test]
fn div_face_id_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defoclosure)",
        expect_test::expect![[r#""OK 166""#]],
    );
}

#[test]
fn div_face_id_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defun)",
        expect_test::expect![[r#""OK 159""#]],
    );
}

#[test]
fn div_face_id_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-defvar)",
        expect_test::expect![[r#""OK 161""#]],
    );
}

#[test]
fn div_face_id_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-face)",
        expect_test::expect![[r#""OK 132""#]],
    );
}

#[test]
fn div_face_id_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-feature)",
        expect_test::expect![[r#""OK 141""#]],
    );
}

#[test]
fn div_face_id_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-free-variable)",
        expect_test::expect![[r#""OK 128""#]],
    );
}

#[test]
fn div_face_id_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-function)",
        expect_test::expect![[r#""OK 135""#]],
    );
}

#[test]
fn div_face_id_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-function-property-declaration)",
        expect_test::expect![[r#""OK 150""#]],
    );
}

#[test]
fn div_face_id_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-group)",
        expect_test::expect![[r#""OK 155""#]],
    );
}

#[test]
fn div_face_id_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-icon)",
        expect_test::expect![[r#""OK 163""#]],
    );
}

#[test]
fn div_face_id_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-macro)",
        expect_test::expect![[r#""OK 138""#]],
    );
}

#[test]
fn div_face_id_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-major-mode-name)",
        expect_test::expect![[r#""OK 131""#]],
    );
}

#[test]
fn div_face_id_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-nnoo-backend)",
        expect_test::expect![[r#""OK 156""#]],
    );
}

#[test]
fn div_face_id_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-non-local-exit)",
        expect_test::expect![[r#""OK 136""#]],
    );
}

#[test]
fn div_face_id_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-oclosure)",
        expect_test::expect![[r#""OK 165""#]],
    );
}

#[test]
fn div_face_id_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-rx)",
        expect_test::expect![[r#""OK 142""#]],
    );
}

#[test]
fn div_face_id_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-shadowed-variable)",
        expect_test::expect![[r#""OK 147""#]],
    );
}

#[test]
fn div_face_id_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-shadowing-variable)",
        expect_test::expect![[r#""OK 146""#]],
    );
}

#[test]
fn div_face_id_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-shorthand-font-lock-face)",
        expect_test::expect![[r#""OK 183""#]],
    );
}

#[test]
fn div_face_id_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-slot)",
        expect_test::expect![[r#""OK 152""#]],
    );
}

#[test]
fn div_face_id_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-special-form)",
        expect_test::expect![[r#""OK 139""#]],
    );
}

#[test]
fn div_face_id_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-special-variable-declaration)",
        expect_test::expect![[r#""OK 129""#]],
    );
}

#[test]
fn div_face_id_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-symbol-at-mouse)",
        expect_test::expect![[r#""OK 127""#]],
    );
}

#[test]
fn div_face_id_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-symbol-role)",
        expect_test::expect![[r#""OK 133""#]],
    );
}

#[test]
fn div_face_id_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-symbol-role-definition)",
        expect_test::expect![[r#""OK 134""#]],
    );
}

#[test]
fn div_face_id_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-theme)",
        expect_test::expect![[r#""OK 143""#]],
    );
}

#[test]
fn div_face_id_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-thing)",
        expect_test::expect![[r#""OK 151""#]],
    );
}

#[test]
fn div_face_id_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-throw-tag)",
        expect_test::expect![[r#""OK 140""#]],
    );
}

#[test]
fn div_face_id_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-type)",
        expect_test::expect![[r#""OK 154""#]],
    );
}

#[test]
fn div_face_id_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-unknown-call)",
        expect_test::expect![[r#""OK 137""#]],
    );
}

#[test]
fn div_face_id_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-variable-at-point)",
        expect_test::expect![[r#""OK 148""#]],
    );
}

#[test]
fn div_face_id_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-warning-type)",
        expect_test::expect![[r#""OK 149""#]],
    );
}

#[test]
fn div_face_id_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'elisp-widget-type)",
        expect_test::expect![[r#""OK 153""#]],
    );
}

#[test]
fn div_face_id_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'error)",
        expect_test::expect![[r#""OK 57""#]],
    );
}

#[test]
fn div_face_id_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'escape-glyph)",
        expect_test::expect![[r#""OK 21""#]],
    );
}

#[test]
fn div_face_id_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'file-name-shadow)",
        expect_test::expect![[r#""OK 116""#]],
    );
}

#[test]
fn div_face_id_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'fill-column-indicator)",
        expect_test::expect![[r#""OK 20""#]],
    );
}

#[test]
fn div_face_id_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'fixed-pitch)",
        expect_test::expect![[r#""OK 5""#]],
    );
}

#[test]
fn div_face_id_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'fixed-pitch-serif)",
        expect_test::expect![[r#""OK 6""#]],
    );
}

#[test]
fn div_face_id_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-bracket-face)",
        expect_test::expect![[r#""OK 107""#]],
    );
}

#[test]
fn div_face_id_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-builtin-face)",
        expect_test::expect![[r#""OK 88""#]],
    );
}

#[test]
fn div_face_id_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-comment-delimiter-face)",
        expect_test::expect![[r#""OK 83""#]],
    );
}

#[test]
fn div_face_id_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-comment-face)",
        expect_test::expect![[r#""OK 82""#]],
    );
}

#[test]
fn div_face_id_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-constant-face)",
        expect_test::expect![[r#""OK 94""#]],
    );
}

#[test]
fn div_face_id_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-delimiter-face)",
        expect_test::expect![[r#""OK 108""#]],
    );
}

#[test]
fn div_face_id_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-doc-face)",
        expect_test::expect![[r#""OK 85""#]],
    );
}

#[test]
fn div_face_id_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-doc-markup-face)",
        expect_test::expect![[r#""OK 86""#]],
    );
}

#[test]
fn div_face_id_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-escape-face)",
        expect_test::expect![[r#""OK 101""#]],
    );
}

#[test]
fn div_face_id_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-function-call-face)",
        expect_test::expect![[r#""OK 90""#]],
    );
}

#[test]
fn div_face_id_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-function-name-face)",
        expect_test::expect![[r#""OK 89""#]],
    );
}

#[test]
fn div_face_id_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-keyword-face)",
        expect_test::expect![[r#""OK 87""#]],
    );
}

#[test]
fn div_face_id_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-misc-punctuation-face)",
        expect_test::expect![[r#""OK 109""#]],
    );
}

#[test]
fn div_face_id_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-negation-char-face)",
        expect_test::expect![[r#""OK 96""#]],
    );
}

#[test]
fn div_face_id_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-number-face)",
        expect_test::expect![[r#""OK 102""#]],
    );
}

#[test]
fn div_face_id_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-operator-face)",
        expect_test::expect![[r#""OK 103""#]],
    );
}

#[test]
fn div_face_id_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-preprocessor-face)",
        expect_test::expect![[r#""OK 97""#]],
    );
}

#[test]
fn div_face_id_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-property-name-face)",
        expect_test::expect![[r#""OK 104""#]],
    );
}

#[test]
fn div_face_id_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-property-use-face)",
        expect_test::expect![[r#""OK 105""#]],
    );
}

#[test]
fn div_face_id_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-punctuation-face)",
        expect_test::expect![[r#""OK 106""#]],
    );
}

#[test]
fn div_face_id_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-regexp-face)",
        expect_test::expect![[r#""OK 98""#]],
    );
}

#[test]
fn div_face_id_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-regexp-grouping-backslash)",
        expect_test::expect![[r#""OK 99""#]],
    );
}

#[test]
fn div_face_id_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-regexp-grouping-construct)",
        expect_test::expect![[r#""OK 100""#]],
    );
}

#[test]
fn div_face_id_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-string-face)",
        expect_test::expect![[r#""OK 84""#]],
    );
}

#[test]
fn div_face_id_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-type-face)",
        expect_test::expect![[r#""OK 93""#]],
    );
}

#[test]
fn div_face_id_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-variable-name-face)",
        expect_test::expect![[r#""OK 91""#]],
    );
}

#[test]
fn div_face_id_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-variable-use-face)",
        expect_test::expect![[r#""OK 92""#]],
    );
}

#[test]
fn div_face_id_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'font-lock-warning-face)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_face_id_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'fringe)",
        expect_test::expect![[r#""OK 43""#]],
    );
}

#[test]
fn div_face_id_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'glyphless-char)",
        expect_test::expect![[r#""OK 56""#]],
    );
}

#[test]
fn div_face_id_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'header-line)",
        expect_test::expect![[r#""OK 31""#]],
    );
}

#[test]
fn div_face_id_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'header-line-active)",
        expect_test::expect![[r#""OK 33""#]],
    );
}

#[test]
fn div_face_id_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'header-line-highlight)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_face_id_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'header-line-inactive)",
        expect_test::expect![[r#""OK 34""#]],
    );
}

#[test]
fn div_face_id_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'help-argument-name)",
        expect_test::expect![[r#""OK 54""#]],
    );
}

#[test]
fn div_face_id_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'help-for-help-header)",
        expect_test::expect![[r#""OK 69""#]],
    );
}

#[test]
fn div_face_id_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'help-key-binding)",
        expect_test::expect![[r#""OK 55""#]],
    );
}

#[test]
fn div_face_id_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'highlight)",
        expect_test::expect![[r#""OK 12""#]],
    );
}

#[test]
fn div_face_id_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'homoglyph)",
        expect_test::expect![[r#""OK 22""#]],
    );
}

#[test]
fn div_face_id_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'internal-border)",
        expect_test::expect![[r#""OK 39""#]],
    );
}

#[test]
fn div_face_id_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'isearch)",
        expect_test::expect![[r#""OK 111""#]],
    );
}

#[test]
fn div_face_id_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'isearch-fail)",
        expect_test::expect![[r#""OK 112""#]],
    );
}

#[test]
fn div_face_id_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'isearch-group-1)",
        expect_test::expect![[r#""OK 114""#]],
    );
}

#[test]
fn div_face_id_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'isearch-group-2)",
        expect_test::expect![[r#""OK 115""#]],
    );
}

#[test]
fn div_face_id_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'italic)",
        expect_test::expect![[r#""OK 2""#]],
    );
}

#[test]
fn div_face_id_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'lazy-highlight)",
        expect_test::expect![[r#""OK 113""#]],
    );
}

#[test]
fn div_face_id_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'line-number)",
        expect_test::expect![[r#""OK 16""#]],
    );
}

#[test]
fn div_face_id_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'line-number-current-line)",
        expect_test::expect![[r#""OK 17""#]],
    );
}

#[test]
fn div_face_id_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'line-number-major-tick)",
        expect_test::expect![[r#""OK 18""#]],
    );
}

#[test]
fn div_face_id_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'line-number-minor-tick)",
        expect_test::expect![[r#""OK 19""#]],
    );
}

#[test]
fn div_face_id_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'link)",
        expect_test::expect![[r#""OK 10""#]],
    );
}

#[test]
fn div_face_id_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'link-visited)",
        expect_test::expect![[r#""OK 11""#]],
    );
}

#[test]
fn div_face_id_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'margin)",
        expect_test::expect![[r#""OK 42""#]],
    );
}

#[test]
fn div_face_id_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'match)",
        expect_test::expect![[r#""OK 124""#]],
    );
}

#[test]
fn div_face_id_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'menu)",
        expect_test::expect![[r#""OK 53""#]],
    );
}

#[test]
fn div_face_id_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'minibuffer-nonselected)",
        expect_test::expect![[r#""OK 81""#]],
    );
}

#[test]
fn div_face_id_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'minibuffer-prompt)",
        expect_test::expect![[r#""OK 41""#]],
    );
}

#[test]
fn div_face_id_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mode-line)",
        expect_test::expect![[r#""OK 25""#]],
    );
}

#[test]
fn div_face_id_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mode-line-active)",
        expect_test::expect![[r#""OK 26""#]],
    );
}

#[test]
fn div_face_id_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mode-line-buffer-id)",
        expect_test::expect![[r#""OK 30""#]],
    );
}

#[test]
fn div_face_id_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mode-line-emphasis)",
        expect_test::expect![[r#""OK 29""#]],
    );
}

#[test]
fn div_face_id_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mode-line-highlight)",
        expect_test::expect![[r#""OK 28""#]],
    );
}

#[test]
fn div_face_id_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mode-line-inactive)",
        expect_test::expect![[r#""OK 27""#]],
    );
}

#[test]
fn div_face_id_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mouse)",
        expect_test::expect![[r#""OK 47""#]],
    );
}

#[test]
fn div_face_id_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'mouse-drag-and-drop-region)",
        expect_test::expect![[r#""OK 110""#]],
    );
}

#[test]
fn div_face_id_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'next-error)",
        expect_test::expect![[r#""OK 71""#]],
    );
}

#[test]
fn div_face_id_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'next-error-message)",
        expect_test::expect![[r#""OK 72""#]],
    );
}

#[test]
fn div_face_id_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'nobreak-hyphen)",
        expect_test::expect![[r#""OK 24""#]],
    );
}

#[test]
fn div_face_id_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'nobreak-space)",
        expect_test::expect![[r#""OK 23""#]],
    );
}

#[test]
fn div_face_id_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'query-replace)",
        expect_test::expect![[r#""OK 123""#]],
    );
}

#[test]
fn div_face_id_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'read-multiple-choice-face)",
        expect_test::expect![[r#""OK 60""#]],
    );
}

#[test]
fn div_face_id_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'region)",
        expect_test::expect![[r#""OK 13""#]],
    );
}

#[test]
fn div_face_id_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'scroll-bar)",
        expect_test::expect![[r#""OK 44""#]],
    );
}

#[test]
fn div_face_id_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'secondary-selection)",
        expect_test::expect![[r#""OK 14""#]],
    );
}

#[test]
fn div_face_id_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'separator-line)",
        expect_test::expect![[r#""OK 73""#]],
    );
}

#[test]
fn div_face_id_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'shadow)",
        expect_test::expect![[r#""OK 9""#]],
    );
}

#[test]
fn div_face_id_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'show-paren-match)",
        expect_test::expect![[r#""OK 64""#]],
    );
}

#[test]
fn div_face_id_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'show-paren-match-expression)",
        expect_test::expect![[r#""OK 65""#]],
    );
}

#[test]
fn div_face_id_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'show-paren-mismatch)",
        expect_test::expect![[r#""OK 66""#]],
    );
}

#[test]
fn div_face_id_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'success)",
        expect_test::expect![[r#""OK 59""#]],
    );
}

#[test]
fn div_face_id_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar)",
        expect_test::expect![[r#""OK 49""#]],
    );
}

#[test]
fn div_face_id_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar-tab)",
        expect_test::expect![[r#""OK 117""#]],
    );
}

#[test]
fn div_face_id_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar-tab-group-current)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_face_id_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar-tab-group-inactive)",
        expect_test::expect![[r#""OK 120""#]],
    );
}

#[test]
fn div_face_id_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar-tab-highlight)",
        expect_test::expect![[r#""OK 122""#]],
    );
}

#[test]
fn div_face_id_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar-tab-inactive)",
        expect_test::expect![[r#""OK 118""#]],
    );
}

#[test]
fn div_face_id_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-bar-tab-ungrouped)",
        expect_test::expect![[r#""OK 121""#]],
    );
}

#[test]
fn div_face_id_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-line)",
        expect_test::expect![[r#""OK 50""#]],
    );
}

#[test]
fn div_face_id_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-line-active)",
        expect_test::expect![[r#""OK 51""#]],
    );
}

#[test]
fn div_face_id_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tab-line-inactive)",
        expect_test::expect![[r#""OK 52""#]],
    );
}

#[test]
fn div_face_id_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tabulated-list-fake-header)",
        expect_test::expect![[r#""OK 125""#]],
    );
}

#[test]
fn div_face_id_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tool-bar)",
        expect_test::expect![[r#""OK 48""#]],
    );
}

#[test]
fn div_face_id_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tooltip)",
        expect_test::expect![[r#""OK 185""#]],
    );
}

#[test]
fn div_face_id_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'trailing-whitespace)",
        expect_test::expect![[r#""OK 15""#]],
    );
}

#[test]
fn div_face_id_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tty-menu-disabled-face)",
        expect_test::expect![[r#""OK 62""#]],
    );
}

#[test]
fn div_face_id_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tty-menu-enabled-face)",
        expect_test::expect![[r#""OK 61""#]],
    );
}

#[test]
fn div_face_id_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'tty-menu-selected-face)",
        expect_test::expect![[r#""OK 63""#]],
    );
}

#[test]
fn div_face_id_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'underline)",
        expect_test::expect![[r#""OK 4""#]],
    );
}

#[test]
fn div_face_id_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'variable-pitch)",
        expect_test::expect![[r#""OK 7""#]],
    );
}

#[test]
fn div_face_id_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'variable-pitch-text)",
        expect_test::expect![[r#""OK 8""#]],
    );
}

#[test]
fn div_face_id_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-conflict-state)",
        expect_test::expect![[r#""OK 178""#]],
    );
}

#[test]
fn div_face_id_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-edited-state)",
        expect_test::expect![[r#""OK 181""#]],
    );
}

#[test]
fn div_face_id_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-ignored-state)",
        expect_test::expect![[r#""OK 182""#]],
    );
}

#[test]
fn div_face_id_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-locally-added-state)",
        expect_test::expect![[r#""OK 177""#]],
    );
}

#[test]
fn div_face_id_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-locked-state)",
        expect_test::expect![[r#""OK 176""#]],
    );
}

#[test]
fn div_face_id_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-missing-state)",
        expect_test::expect![[r#""OK 180""#]],
    );
}

#[test]
fn div_face_id_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-needs-update-state)",
        expect_test::expect![[r#""OK 175""#]],
    );
}

#[test]
fn div_face_id_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-removed-state)",
        expect_test::expect![[r#""OK 179""#]],
    );
}

#[test]
fn div_face_id_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-state-base)",
        expect_test::expect![[r#""OK 173""#]],
    );
}

#[test]
fn div_face_id_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vc-up-to-date-state)",
        expect_test::expect![[r#""OK 174""#]],
    );
}

#[test]
fn div_face_id_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'vertical-border)",
        expect_test::expect![[r#""OK 35""#]],
    );
}

#[test]
fn div_face_id_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'warning)",
        expect_test::expect![[r#""OK 58""#]],
    );
}

#[test]
fn div_face_id_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'window-divider)",
        expect_test::expect![[r#""OK 36""#]],
    );
}

#[test]
fn div_face_id_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'window-divider-first-pixel)",
        expect_test::expect![[r#""OK 37""#]],
    );
}

#[test]
fn div_face_id_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-id 'window-divider-last-pixel)",
        expect_test::expect![[r#""OK 38""#]],
    );
}
