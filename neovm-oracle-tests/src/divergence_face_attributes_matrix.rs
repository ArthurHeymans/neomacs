//! Per-face *face-all-attributes* matrix (all GNU faces).
//!
//! One focused #[test] per face in `(face-list)`: query face-all-attributes
//! against the selected frame. The divergence root cause is the `:inherit`
//! plist cell: Neomacs emits `(:inherit)` (improper) vs GNU's
//! `(:inherit . unspecified)`. Each face surfaces its own divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_attr_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'abbrev-table-name (selected-frame))");
}

#[test]
fn div_face_attr_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'blink-matching-paren-offscreen (selected-frame))");
}

#[test]
fn div_face_attr_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'bold (selected-frame))");
}

#[test]
fn div_face_attr_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'bold-italic (selected-frame))");
}

#[test]
fn div_face_attr_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'border (selected-frame))");
}

#[test]
fn div_face_attr_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'buffer-menu-buffer (selected-frame))");
}

#[test]
fn div_face_attr_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'button (selected-frame))");
}

#[test]
fn div_face_attr_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'child-frame-border (selected-frame))");
}

#[test]
fn div_face_attr_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'completions-annotations (selected-frame))");
}

#[test]
fn div_face_attr_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'completions-common-part (selected-frame))");
}

#[test]
fn div_face_attr_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'completions-first-difference (selected-frame))");
}

#[test]
fn div_face_attr_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'completions-group-separator (selected-frame))");
}

#[test]
fn div_face_attr_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'completions-group-title (selected-frame))");
}

#[test]
fn div_face_attr_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'completions-highlight (selected-frame))");
}

#[test]
fn div_face_attr_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'confusingly-reordered (selected-frame))");
}

#[test]
fn div_face_attr_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'cursor (selected-frame))");
}

#[test]
fn div_face_attr_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'default (selected-frame))");
}

#[test]
fn div_face_attr_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'eldoc-highlight-function-argument (selected-frame))");
}

#[test]
fn div_face_attr_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-ampersand (selected-frame))");
}

#[test]
fn div_face_attr_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-binding-variable (selected-frame))");
}

#[test]
fn div_face_attr_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-bound-variable (selected-frame))");
}

#[test]
fn div_face_attr_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-charset (selected-frame))");
}

#[test]
fn div_face_attr_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-coding (selected-frame))");
}

#[test]
fn div_face_attr_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-completion-category (selected-frame))");
}

#[test]
fn div_face_attr_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-completion-category-definition (selected-frame))");
}

#[test]
fn div_face_attr_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-condition (selected-frame))");
}

#[test]
fn div_face_attr_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-constant (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defcharset (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defcoding (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defface (selected-frame))");
}

#[test]
fn div_face_attr_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-deficon (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defmacro (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defoclosure (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defun (selected-frame))");
}

#[test]
fn div_face_attr_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-defvar (selected-frame))");
}

#[test]
fn div_face_attr_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-face (selected-frame))");
}

#[test]
fn div_face_attr_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-feature (selected-frame))");
}

#[test]
fn div_face_attr_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-free-variable (selected-frame))");
}

#[test]
fn div_face_attr_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-function (selected-frame))");
}

#[test]
fn div_face_attr_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-function-property-declaration (selected-frame))");
}

#[test]
fn div_face_attr_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-group (selected-frame))");
}

#[test]
fn div_face_attr_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-icon (selected-frame))");
}

#[test]
fn div_face_attr_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-macro (selected-frame))");
}

#[test]
fn div_face_attr_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-major-mode-name (selected-frame))");
}

#[test]
fn div_face_attr_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-nnoo-backend (selected-frame))");
}

#[test]
fn div_face_attr_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-non-local-exit (selected-frame))");
}

#[test]
fn div_face_attr_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-oclosure (selected-frame))");
}

#[test]
fn div_face_attr_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-rx (selected-frame))");
}

#[test]
fn div_face_attr_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-shadowed-variable (selected-frame))");
}

#[test]
fn div_face_attr_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-shadowing-variable (selected-frame))");
}

#[test]
fn div_face_attr_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-shorthand-font-lock-face (selected-frame))");
}

#[test]
fn div_face_attr_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-slot (selected-frame))");
}

#[test]
fn div_face_attr_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-special-form (selected-frame))");
}

#[test]
fn div_face_attr_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-special-variable-declaration (selected-frame))");
}

#[test]
fn div_face_attr_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-symbol-at-mouse (selected-frame))");
}

#[test]
fn div_face_attr_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-symbol-role (selected-frame))");
}

#[test]
fn div_face_attr_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-symbol-role-definition (selected-frame))");
}

#[test]
fn div_face_attr_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-theme (selected-frame))");
}

#[test]
fn div_face_attr_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-thing (selected-frame))");
}

#[test]
fn div_face_attr_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-throw-tag (selected-frame))");
}

#[test]
fn div_face_attr_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-type (selected-frame))");
}

#[test]
fn div_face_attr_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-unknown-call (selected-frame))");
}

#[test]
fn div_face_attr_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-variable-at-point (selected-frame))");
}

#[test]
fn div_face_attr_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-warning-type (selected-frame))");
}

#[test]
fn div_face_attr_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'elisp-widget-type (selected-frame))");
}

#[test]
fn div_face_attr_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'error (selected-frame))");
}

#[test]
fn div_face_attr_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'escape-glyph (selected-frame))");
}

#[test]
fn div_face_attr_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'file-name-shadow (selected-frame))");
}

#[test]
fn div_face_attr_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'fill-column-indicator (selected-frame))");
}

#[test]
fn div_face_attr_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'fixed-pitch (selected-frame))");
}

#[test]
fn div_face_attr_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'fixed-pitch-serif (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-bracket-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-builtin-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-comment-delimiter-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-comment-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-constant-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-delimiter-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-doc-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-doc-markup-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-escape-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-function-call-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-function-name-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-keyword-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-misc-punctuation-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-negation-char-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-number-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-operator-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-preprocessor-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-property-name-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-property-use-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-punctuation-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-regexp-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-regexp-grouping-backslash (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-regexp-grouping-construct (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-string-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-type-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-variable-name-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-variable-use-face (selected-frame))");
}

#[test]
fn div_face_attr_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'font-lock-warning-face (selected-frame))");
}

#[test]
fn div_face_attr_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'fringe (selected-frame))");
}

#[test]
fn div_face_attr_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'glyphless-char (selected-frame))");
}

#[test]
fn div_face_attr_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'header-line (selected-frame))");
}

#[test]
fn div_face_attr_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'header-line-active (selected-frame))");
}

#[test]
fn div_face_attr_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'header-line-highlight (selected-frame))");
}

#[test]
fn div_face_attr_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'header-line-inactive (selected-frame))");
}

#[test]
fn div_face_attr_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'help-argument-name (selected-frame))");
}

#[test]
fn div_face_attr_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'help-for-help-header (selected-frame))");
}

#[test]
fn div_face_attr_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'help-key-binding (selected-frame))");
}

#[test]
fn div_face_attr_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'highlight (selected-frame))");
}

#[test]
fn div_face_attr_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'homoglyph (selected-frame))");
}

#[test]
fn div_face_attr_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'internal-border (selected-frame))");
}

#[test]
fn div_face_attr_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'isearch (selected-frame))");
}

#[test]
fn div_face_attr_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'isearch-fail (selected-frame))");
}

#[test]
fn div_face_attr_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'isearch-group-1 (selected-frame))");
}

#[test]
fn div_face_attr_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'isearch-group-2 (selected-frame))");
}

#[test]
fn div_face_attr_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'italic (selected-frame))");
}

#[test]
fn div_face_attr_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'lazy-highlight (selected-frame))");
}

#[test]
fn div_face_attr_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'line-number (selected-frame))");
}

#[test]
fn div_face_attr_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'line-number-current-line (selected-frame))");
}

#[test]
fn div_face_attr_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'line-number-major-tick (selected-frame))");
}

#[test]
fn div_face_attr_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'line-number-minor-tick (selected-frame))");
}

#[test]
fn div_face_attr_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'link (selected-frame))");
}

#[test]
fn div_face_attr_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'link-visited (selected-frame))");
}

#[test]
fn div_face_attr_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'margin (selected-frame))");
}

#[test]
fn div_face_attr_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'match (selected-frame))");
}

#[test]
fn div_face_attr_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'menu (selected-frame))");
}

#[test]
fn div_face_attr_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'minibuffer-nonselected (selected-frame))");
}

#[test]
fn div_face_attr_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'minibuffer-prompt (selected-frame))");
}

#[test]
fn div_face_attr_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mode-line (selected-frame))");
}

#[test]
fn div_face_attr_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mode-line-active (selected-frame))");
}

#[test]
fn div_face_attr_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mode-line-buffer-id (selected-frame))");
}

#[test]
fn div_face_attr_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mode-line-emphasis (selected-frame))");
}

#[test]
fn div_face_attr_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mode-line-highlight (selected-frame))");
}

#[test]
fn div_face_attr_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mode-line-inactive (selected-frame))");
}

#[test]
fn div_face_attr_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mouse (selected-frame))");
}

#[test]
fn div_face_attr_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'mouse-drag-and-drop-region (selected-frame))");
}

#[test]
fn div_face_attr_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'next-error (selected-frame))");
}

#[test]
fn div_face_attr_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'next-error-message (selected-frame))");
}

#[test]
fn div_face_attr_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'nobreak-hyphen (selected-frame))");
}

#[test]
fn div_face_attr_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'nobreak-space (selected-frame))");
}

#[test]
fn div_face_attr_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'query-replace (selected-frame))");
}

#[test]
fn div_face_attr_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'read-multiple-choice-face (selected-frame))");
}

#[test]
fn div_face_attr_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'region (selected-frame))");
}

#[test]
fn div_face_attr_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'scroll-bar (selected-frame))");
}

#[test]
fn div_face_attr_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'secondary-selection (selected-frame))");
}

#[test]
fn div_face_attr_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'separator-line (selected-frame))");
}

#[test]
fn div_face_attr_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'shadow (selected-frame))");
}

#[test]
fn div_face_attr_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'show-paren-match (selected-frame))");
}

#[test]
fn div_face_attr_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'show-paren-match-expression (selected-frame))");
}

#[test]
fn div_face_attr_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'show-paren-mismatch (selected-frame))");
}

#[test]
fn div_face_attr_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'success (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar-tab (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar-tab-group-current (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar-tab-group-inactive (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar-tab-highlight (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar-tab-inactive (selected-frame))");
}

#[test]
fn div_face_attr_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-bar-tab-ungrouped (selected-frame))");
}

#[test]
fn div_face_attr_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-line (selected-frame))");
}

#[test]
fn div_face_attr_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-line-active (selected-frame))");
}

#[test]
fn div_face_attr_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tab-line-inactive (selected-frame))");
}

#[test]
fn div_face_attr_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tabulated-list-fake-header (selected-frame))");
}

#[test]
fn div_face_attr_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tool-bar (selected-frame))");
}

#[test]
fn div_face_attr_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tooltip (selected-frame))");
}

#[test]
fn div_face_attr_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'trailing-whitespace (selected-frame))");
}

#[test]
fn div_face_attr_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tty-menu-disabled-face (selected-frame))");
}

#[test]
fn div_face_attr_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tty-menu-enabled-face (selected-frame))");
}

#[test]
fn div_face_attr_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'tty-menu-selected-face (selected-frame))");
}

#[test]
fn div_face_attr_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'underline (selected-frame))");
}

#[test]
fn div_face_attr_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'variable-pitch (selected-frame))");
}

#[test]
fn div_face_attr_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'variable-pitch-text (selected-frame))");
}

#[test]
fn div_face_attr_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-conflict-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-edited-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-ignored-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-locally-added-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-locked-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-missing-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-needs-update-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-removed-state (selected-frame))");
}

#[test]
fn div_face_attr_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-state-base (selected-frame))");
}

#[test]
fn div_face_attr_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vc-up-to-date-state (selected-frame))");
}

#[test]
fn div_face_attr_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'vertical-border (selected-frame))");
}

#[test]
fn div_face_attr_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'warning (selected-frame))");
}

#[test]
fn div_face_attr_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'window-divider (selected-frame))");
}

#[test]
fn div_face_attr_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'window-divider-first-pixel (selected-frame))");
}

#[test]
fn div_face_attr_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-all-attributes 'window-divider-last-pixel (selected-frame))");
}
