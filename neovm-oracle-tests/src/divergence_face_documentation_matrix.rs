//! Per-face *face-documentation* matrix (all GNU faces).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_doc_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'abbrev-table-name)");
}

#[test]
fn div_face_doc_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'blink-matching-paren-offscreen)");
}

#[test]
fn div_face_doc_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'bold)");
}

#[test]
fn div_face_doc_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'bold-italic)");
}

#[test]
fn div_face_doc_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'border)");
}

#[test]
fn div_face_doc_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'buffer-menu-buffer)");
}

#[test]
fn div_face_doc_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'button)");
}

#[test]
fn div_face_doc_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'child-frame-border)");
}

#[test]
fn div_face_doc_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'completions-annotations)");
}

#[test]
fn div_face_doc_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'completions-common-part)");
}

#[test]
fn div_face_doc_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'completions-first-difference)");
}

#[test]
fn div_face_doc_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'completions-group-separator)");
}

#[test]
fn div_face_doc_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'completions-group-title)");
}

#[test]
fn div_face_doc_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'completions-highlight)");
}

#[test]
fn div_face_doc_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'confusingly-reordered)");
}

#[test]
fn div_face_doc_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'cursor)");
}

#[test]
fn div_face_doc_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'default)");
}

#[test]
fn div_face_doc_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'eldoc-highlight-function-argument)");
}

#[test]
fn div_face_doc_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-ampersand)");
}

#[test]
fn div_face_doc_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-binding-variable)");
}

#[test]
fn div_face_doc_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-bound-variable)");
}

#[test]
fn div_face_doc_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-charset)");
}

#[test]
fn div_face_doc_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-coding)");
}

#[test]
fn div_face_doc_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-completion-category)");
}

#[test]
fn div_face_doc_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-completion-category-definition)");
}

#[test]
fn div_face_doc_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-condition)");
}

#[test]
fn div_face_doc_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-constant)");
}

#[test]
fn div_face_doc_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defcharset)");
}

#[test]
fn div_face_doc_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defcoding)");
}

#[test]
fn div_face_doc_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defface)");
}

#[test]
fn div_face_doc_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-deficon)");
}

#[test]
fn div_face_doc_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defmacro)");
}

#[test]
fn div_face_doc_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defoclosure)");
}

#[test]
fn div_face_doc_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defun)");
}

#[test]
fn div_face_doc_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-defvar)");
}

#[test]
fn div_face_doc_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-face)");
}

#[test]
fn div_face_doc_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-feature)");
}

#[test]
fn div_face_doc_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-free-variable)");
}

#[test]
fn div_face_doc_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-function)");
}

#[test]
fn div_face_doc_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-function-property-declaration)");
}

#[test]
fn div_face_doc_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-group)");
}

#[test]
fn div_face_doc_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-icon)");
}

#[test]
fn div_face_doc_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-macro)");
}

#[test]
fn div_face_doc_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-major-mode-name)");
}

#[test]
fn div_face_doc_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-nnoo-backend)");
}

#[test]
fn div_face_doc_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-non-local-exit)");
}

#[test]
fn div_face_doc_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-oclosure)");
}

#[test]
fn div_face_doc_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-rx)");
}

#[test]
fn div_face_doc_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-shadowed-variable)");
}

#[test]
fn div_face_doc_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-shadowing-variable)");
}

#[test]
fn div_face_doc_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-shorthand-font-lock-face)");
}

#[test]
fn div_face_doc_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-slot)");
}

#[test]
fn div_face_doc_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-special-form)");
}

#[test]
fn div_face_doc_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-special-variable-declaration)");
}

#[test]
fn div_face_doc_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-symbol-at-mouse)");
}

#[test]
fn div_face_doc_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-symbol-role)");
}

#[test]
fn div_face_doc_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-symbol-role-definition)");
}

#[test]
fn div_face_doc_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-theme)");
}

#[test]
fn div_face_doc_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-thing)");
}

#[test]
fn div_face_doc_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-throw-tag)");
}

#[test]
fn div_face_doc_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-type)");
}

#[test]
fn div_face_doc_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-unknown-call)");
}

#[test]
fn div_face_doc_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-variable-at-point)");
}

#[test]
fn div_face_doc_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-warning-type)");
}

#[test]
fn div_face_doc_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'elisp-widget-type)");
}

#[test]
fn div_face_doc_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'error)");
}

#[test]
fn div_face_doc_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'escape-glyph)");
}

#[test]
fn div_face_doc_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'file-name-shadow)");
}

#[test]
fn div_face_doc_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'fill-column-indicator)");
}

#[test]
fn div_face_doc_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'fixed-pitch)");
}

#[test]
fn div_face_doc_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'fixed-pitch-serif)");
}

#[test]
fn div_face_doc_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-bracket-face)");
}

#[test]
fn div_face_doc_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-builtin-face)");
}

#[test]
fn div_face_doc_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-comment-delimiter-face)");
}

#[test]
fn div_face_doc_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-comment-face)");
}

#[test]
fn div_face_doc_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-constant-face)");
}

#[test]
fn div_face_doc_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-delimiter-face)");
}

#[test]
fn div_face_doc_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-doc-face)");
}

#[test]
fn div_face_doc_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-doc-markup-face)");
}

#[test]
fn div_face_doc_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-escape-face)");
}

#[test]
fn div_face_doc_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-function-call-face)");
}

#[test]
fn div_face_doc_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-function-name-face)");
}

#[test]
fn div_face_doc_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-keyword-face)");
}

#[test]
fn div_face_doc_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-misc-punctuation-face)");
}

#[test]
fn div_face_doc_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-negation-char-face)");
}

#[test]
fn div_face_doc_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-number-face)");
}

#[test]
fn div_face_doc_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-operator-face)");
}

#[test]
fn div_face_doc_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-preprocessor-face)");
}

#[test]
fn div_face_doc_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-property-name-face)");
}

#[test]
fn div_face_doc_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-property-use-face)");
}

#[test]
fn div_face_doc_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-punctuation-face)");
}

#[test]
fn div_face_doc_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-regexp-face)");
}

#[test]
fn div_face_doc_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-regexp-grouping-backslash)");
}

#[test]
fn div_face_doc_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-regexp-grouping-construct)");
}

#[test]
fn div_face_doc_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-string-face)");
}

#[test]
fn div_face_doc_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-type-face)");
}

#[test]
fn div_face_doc_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-variable-name-face)");
}

#[test]
fn div_face_doc_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-variable-use-face)");
}

#[test]
fn div_face_doc_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'font-lock-warning-face)");
}

#[test]
fn div_face_doc_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'fringe)");
}

#[test]
fn div_face_doc_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'glyphless-char)");
}

#[test]
fn div_face_doc_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'header-line)");
}

#[test]
fn div_face_doc_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'header-line-active)");
}

#[test]
fn div_face_doc_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'header-line-highlight)");
}

#[test]
fn div_face_doc_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'header-line-inactive)");
}

#[test]
fn div_face_doc_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'help-argument-name)");
}

#[test]
fn div_face_doc_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'help-for-help-header)");
}

#[test]
fn div_face_doc_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'help-key-binding)");
}

#[test]
fn div_face_doc_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'highlight)");
}

#[test]
fn div_face_doc_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'homoglyph)");
}

#[test]
fn div_face_doc_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'internal-border)");
}

#[test]
fn div_face_doc_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'isearch)");
}

#[test]
fn div_face_doc_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'isearch-fail)");
}

#[test]
fn div_face_doc_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'isearch-group-1)");
}

#[test]
fn div_face_doc_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'isearch-group-2)");
}

#[test]
fn div_face_doc_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'italic)");
}

#[test]
fn div_face_doc_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'lazy-highlight)");
}

#[test]
fn div_face_doc_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'line-number)");
}

#[test]
fn div_face_doc_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'line-number-current-line)");
}

#[test]
fn div_face_doc_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'line-number-major-tick)");
}

#[test]
fn div_face_doc_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'line-number-minor-tick)");
}

#[test]
fn div_face_doc_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'link)");
}

#[test]
fn div_face_doc_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'link-visited)");
}

#[test]
fn div_face_doc_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'margin)");
}

#[test]
fn div_face_doc_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'match)");
}

#[test]
fn div_face_doc_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'menu)");
}

#[test]
fn div_face_doc_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'minibuffer-nonselected)");
}

#[test]
fn div_face_doc_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'minibuffer-prompt)");
}

#[test]
fn div_face_doc_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mode-line)");
}

#[test]
fn div_face_doc_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mode-line-active)");
}

#[test]
fn div_face_doc_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mode-line-buffer-id)");
}

#[test]
fn div_face_doc_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mode-line-emphasis)");
}

#[test]
fn div_face_doc_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mode-line-highlight)");
}

#[test]
fn div_face_doc_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mode-line-inactive)");
}

#[test]
fn div_face_doc_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mouse)");
}

#[test]
fn div_face_doc_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'mouse-drag-and-drop-region)");
}

#[test]
fn div_face_doc_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'next-error)");
}

#[test]
fn div_face_doc_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'next-error-message)");
}

#[test]
fn div_face_doc_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'nobreak-hyphen)");
}

#[test]
fn div_face_doc_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'nobreak-space)");
}

#[test]
fn div_face_doc_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'query-replace)");
}

#[test]
fn div_face_doc_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'read-multiple-choice-face)");
}

#[test]
fn div_face_doc_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'region)");
}

#[test]
fn div_face_doc_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'scroll-bar)");
}

#[test]
fn div_face_doc_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'secondary-selection)");
}

#[test]
fn div_face_doc_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'separator-line)");
}

#[test]
fn div_face_doc_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'shadow)");
}

#[test]
fn div_face_doc_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'show-paren-match)");
}

#[test]
fn div_face_doc_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'show-paren-match-expression)");
}

#[test]
fn div_face_doc_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'show-paren-mismatch)");
}

#[test]
fn div_face_doc_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'success)");
}

#[test]
fn div_face_doc_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar)");
}

#[test]
fn div_face_doc_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar-tab)");
}

#[test]
fn div_face_doc_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar-tab-group-current)");
}

#[test]
fn div_face_doc_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar-tab-group-inactive)");
}

#[test]
fn div_face_doc_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar-tab-highlight)");
}

#[test]
fn div_face_doc_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar-tab-inactive)");
}

#[test]
fn div_face_doc_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-bar-tab-ungrouped)");
}

#[test]
fn div_face_doc_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-line)");
}

#[test]
fn div_face_doc_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-line-active)");
}

#[test]
fn div_face_doc_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tab-line-inactive)");
}

#[test]
fn div_face_doc_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tabulated-list-fake-header)");
}

#[test]
fn div_face_doc_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tool-bar)");
}

#[test]
fn div_face_doc_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tooltip)");
}

#[test]
fn div_face_doc_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'trailing-whitespace)");
}

#[test]
fn div_face_doc_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tty-menu-disabled-face)");
}

#[test]
fn div_face_doc_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tty-menu-enabled-face)");
}

#[test]
fn div_face_doc_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'tty-menu-selected-face)");
}

#[test]
fn div_face_doc_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'underline)");
}

#[test]
fn div_face_doc_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'variable-pitch)");
}

#[test]
fn div_face_doc_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'variable-pitch-text)");
}

#[test]
fn div_face_doc_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-conflict-state)");
}

#[test]
fn div_face_doc_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-edited-state)");
}

#[test]
fn div_face_doc_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-ignored-state)");
}

#[test]
fn div_face_doc_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-locally-added-state)");
}

#[test]
fn div_face_doc_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-locked-state)");
}

#[test]
fn div_face_doc_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-missing-state)");
}

#[test]
fn div_face_doc_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-needs-update-state)");
}

#[test]
fn div_face_doc_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-removed-state)");
}

#[test]
fn div_face_doc_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-state-base)");
}

#[test]
fn div_face_doc_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vc-up-to-date-state)");
}

#[test]
fn div_face_doc_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'vertical-border)");
}

#[test]
fn div_face_doc_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'warning)");
}

#[test]
fn div_face_doc_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'window-divider)");
}

#[test]
fn div_face_doc_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'window-divider-first-pixel)");
}

#[test]
fn div_face_doc_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-documentation 'window-divider-last-pixel)");
}
