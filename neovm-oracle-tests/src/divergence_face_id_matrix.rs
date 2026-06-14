//! Per-face *face-id* matrix (all GNU faces).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_id_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'abbrev-table-name)");
}

#[test]
fn div_face_id_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'blink-matching-paren-offscreen)");
}

#[test]
fn div_face_id_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'bold)");
}

#[test]
fn div_face_id_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'bold-italic)");
}

#[test]
fn div_face_id_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'border)");
}

#[test]
fn div_face_id_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'buffer-menu-buffer)");
}

#[test]
fn div_face_id_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'button)");
}

#[test]
fn div_face_id_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'child-frame-border)");
}

#[test]
fn div_face_id_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'completions-annotations)");
}

#[test]
fn div_face_id_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'completions-common-part)");
}

#[test]
fn div_face_id_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'completions-first-difference)");
}

#[test]
fn div_face_id_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'completions-group-separator)");
}

#[test]
fn div_face_id_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'completions-group-title)");
}

#[test]
fn div_face_id_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'completions-highlight)");
}

#[test]
fn div_face_id_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'confusingly-reordered)");
}

#[test]
fn div_face_id_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'cursor)");
}

#[test]
fn div_face_id_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'default)");
}

#[test]
fn div_face_id_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'eldoc-highlight-function-argument)");
}

#[test]
fn div_face_id_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-ampersand)");
}

#[test]
fn div_face_id_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-binding-variable)");
}

#[test]
fn div_face_id_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-bound-variable)");
}

#[test]
fn div_face_id_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-charset)");
}

#[test]
fn div_face_id_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-coding)");
}

#[test]
fn div_face_id_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-completion-category)");
}

#[test]
fn div_face_id_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-completion-category-definition)");
}

#[test]
fn div_face_id_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-condition)");
}

#[test]
fn div_face_id_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-constant)");
}

#[test]
fn div_face_id_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defcharset)");
}

#[test]
fn div_face_id_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defcoding)");
}

#[test]
fn div_face_id_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defface)");
}

#[test]
fn div_face_id_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-deficon)");
}

#[test]
fn div_face_id_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defmacro)");
}

#[test]
fn div_face_id_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defoclosure)");
}

#[test]
fn div_face_id_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defun)");
}

#[test]
fn div_face_id_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-defvar)");
}

#[test]
fn div_face_id_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-face)");
}

#[test]
fn div_face_id_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-feature)");
}

#[test]
fn div_face_id_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-free-variable)");
}

#[test]
fn div_face_id_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-function)");
}

#[test]
fn div_face_id_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-function-property-declaration)");
}

#[test]
fn div_face_id_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-group)");
}

#[test]
fn div_face_id_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-icon)");
}

#[test]
fn div_face_id_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-macro)");
}

#[test]
fn div_face_id_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-major-mode-name)");
}

#[test]
fn div_face_id_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-nnoo-backend)");
}

#[test]
fn div_face_id_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-non-local-exit)");
}

#[test]
fn div_face_id_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-oclosure)");
}

#[test]
fn div_face_id_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-rx)");
}

#[test]
fn div_face_id_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-shadowed-variable)");
}

#[test]
fn div_face_id_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-shadowing-variable)");
}

#[test]
fn div_face_id_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-shorthand-font-lock-face)");
}

#[test]
fn div_face_id_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-slot)");
}

#[test]
fn div_face_id_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-special-form)");
}

#[test]
fn div_face_id_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-special-variable-declaration)");
}

#[test]
fn div_face_id_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-symbol-at-mouse)");
}

#[test]
fn div_face_id_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-symbol-role)");
}

#[test]
fn div_face_id_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-symbol-role-definition)");
}

#[test]
fn div_face_id_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-theme)");
}

#[test]
fn div_face_id_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-thing)");
}

#[test]
fn div_face_id_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-throw-tag)");
}

#[test]
fn div_face_id_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-type)");
}

#[test]
fn div_face_id_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-unknown-call)");
}

#[test]
fn div_face_id_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-variable-at-point)");
}

#[test]
fn div_face_id_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-warning-type)");
}

#[test]
fn div_face_id_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'elisp-widget-type)");
}

#[test]
fn div_face_id_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'error)");
}

#[test]
fn div_face_id_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'escape-glyph)");
}

#[test]
fn div_face_id_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'file-name-shadow)");
}

#[test]
fn div_face_id_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'fill-column-indicator)");
}

#[test]
fn div_face_id_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'fixed-pitch)");
}

#[test]
fn div_face_id_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'fixed-pitch-serif)");
}

#[test]
fn div_face_id_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-bracket-face)");
}

#[test]
fn div_face_id_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-builtin-face)");
}

#[test]
fn div_face_id_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-comment-delimiter-face)");
}

#[test]
fn div_face_id_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-comment-face)");
}

#[test]
fn div_face_id_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-constant-face)");
}

#[test]
fn div_face_id_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-delimiter-face)");
}

#[test]
fn div_face_id_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-doc-face)");
}

#[test]
fn div_face_id_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-doc-markup-face)");
}

#[test]
fn div_face_id_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-escape-face)");
}

#[test]
fn div_face_id_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-function-call-face)");
}

#[test]
fn div_face_id_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-function-name-face)");
}

#[test]
fn div_face_id_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-keyword-face)");
}

#[test]
fn div_face_id_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-misc-punctuation-face)");
}

#[test]
fn div_face_id_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-negation-char-face)");
}

#[test]
fn div_face_id_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-number-face)");
}

#[test]
fn div_face_id_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-operator-face)");
}

#[test]
fn div_face_id_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-preprocessor-face)");
}

#[test]
fn div_face_id_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-property-name-face)");
}

#[test]
fn div_face_id_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-property-use-face)");
}

#[test]
fn div_face_id_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-punctuation-face)");
}

#[test]
fn div_face_id_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-regexp-face)");
}

#[test]
fn div_face_id_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-regexp-grouping-backslash)");
}

#[test]
fn div_face_id_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-regexp-grouping-construct)");
}

#[test]
fn div_face_id_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-string-face)");
}

#[test]
fn div_face_id_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-type-face)");
}

#[test]
fn div_face_id_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-variable-name-face)");
}

#[test]
fn div_face_id_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-variable-use-face)");
}

#[test]
fn div_face_id_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'font-lock-warning-face)");
}

#[test]
fn div_face_id_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'fringe)");
}

#[test]
fn div_face_id_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'glyphless-char)");
}

#[test]
fn div_face_id_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'header-line)");
}

#[test]
fn div_face_id_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'header-line-active)");
}

#[test]
fn div_face_id_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'header-line-highlight)");
}

#[test]
fn div_face_id_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'header-line-inactive)");
}

#[test]
fn div_face_id_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'help-argument-name)");
}

#[test]
fn div_face_id_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'help-for-help-header)");
}

#[test]
fn div_face_id_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'help-key-binding)");
}

#[test]
fn div_face_id_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'highlight)");
}

#[test]
fn div_face_id_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'homoglyph)");
}

#[test]
fn div_face_id_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'internal-border)");
}

#[test]
fn div_face_id_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'isearch)");
}

#[test]
fn div_face_id_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'isearch-fail)");
}

#[test]
fn div_face_id_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'isearch-group-1)");
}

#[test]
fn div_face_id_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'isearch-group-2)");
}

#[test]
fn div_face_id_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'italic)");
}

#[test]
fn div_face_id_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'lazy-highlight)");
}

#[test]
fn div_face_id_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'line-number)");
}

#[test]
fn div_face_id_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'line-number-current-line)");
}

#[test]
fn div_face_id_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'line-number-major-tick)");
}

#[test]
fn div_face_id_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'line-number-minor-tick)");
}

#[test]
fn div_face_id_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'link)");
}

#[test]
fn div_face_id_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'link-visited)");
}

#[test]
fn div_face_id_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'margin)");
}

#[test]
fn div_face_id_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'match)");
}

#[test]
fn div_face_id_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'menu)");
}

#[test]
fn div_face_id_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'minibuffer-nonselected)");
}

#[test]
fn div_face_id_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'minibuffer-prompt)");
}

#[test]
fn div_face_id_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mode-line)");
}

#[test]
fn div_face_id_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mode-line-active)");
}

#[test]
fn div_face_id_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mode-line-buffer-id)");
}

#[test]
fn div_face_id_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mode-line-emphasis)");
}

#[test]
fn div_face_id_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mode-line-highlight)");
}

#[test]
fn div_face_id_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mode-line-inactive)");
}

#[test]
fn div_face_id_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mouse)");
}

#[test]
fn div_face_id_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'mouse-drag-and-drop-region)");
}

#[test]
fn div_face_id_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'next-error)");
}

#[test]
fn div_face_id_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'next-error-message)");
}

#[test]
fn div_face_id_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'nobreak-hyphen)");
}

#[test]
fn div_face_id_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'nobreak-space)");
}

#[test]
fn div_face_id_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'query-replace)");
}

#[test]
fn div_face_id_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'read-multiple-choice-face)");
}

#[test]
fn div_face_id_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'region)");
}

#[test]
fn div_face_id_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'scroll-bar)");
}

#[test]
fn div_face_id_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'secondary-selection)");
}

#[test]
fn div_face_id_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'separator-line)");
}

#[test]
fn div_face_id_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'shadow)");
}

#[test]
fn div_face_id_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'show-paren-match)");
}

#[test]
fn div_face_id_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'show-paren-match-expression)");
}

#[test]
fn div_face_id_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'show-paren-mismatch)");
}

#[test]
fn div_face_id_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'success)");
}

#[test]
fn div_face_id_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar)");
}

#[test]
fn div_face_id_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar-tab)");
}

#[test]
fn div_face_id_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar-tab-group-current)");
}

#[test]
fn div_face_id_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar-tab-group-inactive)");
}

#[test]
fn div_face_id_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar-tab-highlight)");
}

#[test]
fn div_face_id_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar-tab-inactive)");
}

#[test]
fn div_face_id_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-bar-tab-ungrouped)");
}

#[test]
fn div_face_id_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-line)");
}

#[test]
fn div_face_id_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-line-active)");
}

#[test]
fn div_face_id_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tab-line-inactive)");
}

#[test]
fn div_face_id_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tabulated-list-fake-header)");
}

#[test]
fn div_face_id_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tool-bar)");
}

#[test]
fn div_face_id_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tooltip)");
}

#[test]
fn div_face_id_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'trailing-whitespace)");
}

#[test]
fn div_face_id_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tty-menu-disabled-face)");
}

#[test]
fn div_face_id_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tty-menu-enabled-face)");
}

#[test]
fn div_face_id_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'tty-menu-selected-face)");
}

#[test]
fn div_face_id_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'underline)");
}

#[test]
fn div_face_id_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'variable-pitch)");
}

#[test]
fn div_face_id_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'variable-pitch-text)");
}

#[test]
fn div_face_id_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-conflict-state)");
}

#[test]
fn div_face_id_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-edited-state)");
}

#[test]
fn div_face_id_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-ignored-state)");
}

#[test]
fn div_face_id_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-locally-added-state)");
}

#[test]
fn div_face_id_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-locked-state)");
}

#[test]
fn div_face_id_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-missing-state)");
}

#[test]
fn div_face_id_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-needs-update-state)");
}

#[test]
fn div_face_id_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-removed-state)");
}

#[test]
fn div_face_id_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-state-base)");
}

#[test]
fn div_face_id_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vc-up-to-date-state)");
}

#[test]
fn div_face_id_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'vertical-border)");
}

#[test]
fn div_face_id_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'warning)");
}

#[test]
fn div_face_id_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'window-divider)");
}

#[test]
fn div_face_id_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'window-divider-first-pixel)");
}

#[test]
fn div_face_id_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-id 'window-divider-last-pixel)");
}
