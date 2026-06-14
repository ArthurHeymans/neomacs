//! Per-face *face-attribute :slant* matrix.
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_slant_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'abbrev-table-name :slant)");
}

#[test]
fn div_face_slant_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'blink-matching-paren-offscreen :slant)");
}

#[test]
fn div_face_slant_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold :slant)");
}

#[test]
fn div_face_slant_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold-italic :slant)");
}

#[test]
fn div_face_slant_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'border :slant)");
}

#[test]
fn div_face_slant_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'buffer-menu-buffer :slant)");
}

#[test]
fn div_face_slant_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'button :slant)");
}

#[test]
fn div_face_slant_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'child-frame-border :slant)");
}

#[test]
fn div_face_slant_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-annotations :slant)");
}

#[test]
fn div_face_slant_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-common-part :slant)");
}

#[test]
fn div_face_slant_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-first-difference :slant)");
}

#[test]
fn div_face_slant_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-separator :slant)");
}

#[test]
fn div_face_slant_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-title :slant)");
}

#[test]
fn div_face_slant_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-highlight :slant)");
}

#[test]
fn div_face_slant_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'confusingly-reordered :slant)");
}

#[test]
fn div_face_slant_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'cursor :slant)");
}

#[test]
fn div_face_slant_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'default :slant)");
}

#[test]
fn div_face_slant_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'eldoc-highlight-function-argument :slant)");
}

#[test]
fn div_face_slant_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-ampersand :slant)");
}

#[test]
fn div_face_slant_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-binding-variable :slant)");
}

#[test]
fn div_face_slant_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-bound-variable :slant)");
}

#[test]
fn div_face_slant_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-charset :slant)");
}

#[test]
fn div_face_slant_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-coding :slant)");
}

#[test]
fn div_face_slant_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category :slant)");
}

#[test]
fn div_face_slant_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category-definition :slant)");
}

#[test]
fn div_face_slant_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-condition :slant)");
}

#[test]
fn div_face_slant_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-constant :slant)");
}

#[test]
fn div_face_slant_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcharset :slant)");
}

#[test]
fn div_face_slant_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcoding :slant)");
}

#[test]
fn div_face_slant_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defface :slant)");
}

#[test]
fn div_face_slant_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-deficon :slant)");
}

#[test]
fn div_face_slant_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defmacro :slant)");
}

#[test]
fn div_face_slant_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defoclosure :slant)");
}

#[test]
fn div_face_slant_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defun :slant)");
}

#[test]
fn div_face_slant_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defvar :slant)");
}

#[test]
fn div_face_slant_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-face :slant)");
}

#[test]
fn div_face_slant_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-feature :slant)");
}

#[test]
fn div_face_slant_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-free-variable :slant)");
}

#[test]
fn div_face_slant_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function :slant)");
}

#[test]
fn div_face_slant_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function-property-declaration :slant)");
}

#[test]
fn div_face_slant_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-group :slant)");
}

#[test]
fn div_face_slant_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-icon :slant)");
}

#[test]
fn div_face_slant_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-macro :slant)");
}

#[test]
fn div_face_slant_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-major-mode-name :slant)");
}

#[test]
fn div_face_slant_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-nnoo-backend :slant)");
}

#[test]
fn div_face_slant_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-non-local-exit :slant)");
}

#[test]
fn div_face_slant_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-oclosure :slant)");
}

#[test]
fn div_face_slant_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-rx :slant)");
}

#[test]
fn div_face_slant_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowed-variable :slant)");
}

#[test]
fn div_face_slant_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowing-variable :slant)");
}

#[test]
fn div_face_slant_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shorthand-font-lock-face :slant)");
}

#[test]
fn div_face_slant_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-slot :slant)");
}

#[test]
fn div_face_slant_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-form :slant)");
}

#[test]
fn div_face_slant_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-variable-declaration :slant)");
}

#[test]
fn div_face_slant_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-at-mouse :slant)");
}

#[test]
fn div_face_slant_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role :slant)");
}

#[test]
fn div_face_slant_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role-definition :slant)");
}

#[test]
fn div_face_slant_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-theme :slant)");
}

#[test]
fn div_face_slant_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-thing :slant)");
}

#[test]
fn div_face_slant_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-throw-tag :slant)");
}

#[test]
fn div_face_slant_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-type :slant)");
}

#[test]
fn div_face_slant_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-unknown-call :slant)");
}

#[test]
fn div_face_slant_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-variable-at-point :slant)");
}

#[test]
fn div_face_slant_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-warning-type :slant)");
}

#[test]
fn div_face_slant_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-widget-type :slant)");
}

#[test]
fn div_face_slant_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'error :slant)");
}

#[test]
fn div_face_slant_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'escape-glyph :slant)");
}

#[test]
fn div_face_slant_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'file-name-shadow :slant)");
}

#[test]
fn div_face_slant_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fill-column-indicator :slant)");
}

#[test]
fn div_face_slant_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch :slant)");
}

#[test]
fn div_face_slant_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch-serif :slant)");
}

#[test]
fn div_face_slant_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-bracket-face :slant)");
}

#[test]
fn div_face_slant_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-builtin-face :slant)");
}

#[test]
fn div_face_slant_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-delimiter-face :slant)");
}

#[test]
fn div_face_slant_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-face :slant)");
}

#[test]
fn div_face_slant_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-constant-face :slant)");
}

#[test]
fn div_face_slant_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-delimiter-face :slant)");
}

#[test]
fn div_face_slant_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-face :slant)");
}

#[test]
fn div_face_slant_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-markup-face :slant)");
}

#[test]
fn div_face_slant_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-escape-face :slant)");
}

#[test]
fn div_face_slant_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-call-face :slant)");
}

#[test]
fn div_face_slant_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-name-face :slant)");
}

#[test]
fn div_face_slant_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-keyword-face :slant)");
}

#[test]
fn div_face_slant_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-misc-punctuation-face :slant)");
}

#[test]
fn div_face_slant_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-negation-char-face :slant)");
}

#[test]
fn div_face_slant_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-number-face :slant)");
}

#[test]
fn div_face_slant_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-operator-face :slant)");
}

#[test]
fn div_face_slant_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-preprocessor-face :slant)");
}

#[test]
fn div_face_slant_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-name-face :slant)");
}

#[test]
fn div_face_slant_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-use-face :slant)");
}

#[test]
fn div_face_slant_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-punctuation-face :slant)");
}

#[test]
fn div_face_slant_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-face :slant)");
}

#[test]
fn div_face_slant_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-backslash :slant)");
}

#[test]
fn div_face_slant_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-construct :slant)");
}

#[test]
fn div_face_slant_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-string-face :slant)");
}

#[test]
fn div_face_slant_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-type-face :slant)");
}

#[test]
fn div_face_slant_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-name-face :slant)");
}

#[test]
fn div_face_slant_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-use-face :slant)");
}

#[test]
fn div_face_slant_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-warning-face :slant)");
}

#[test]
fn div_face_slant_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fringe :slant)");
}

#[test]
fn div_face_slant_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'glyphless-char :slant)");
}

#[test]
fn div_face_slant_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line :slant)");
}

#[test]
fn div_face_slant_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-active :slant)");
}

#[test]
fn div_face_slant_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-highlight :slant)");
}

#[test]
fn div_face_slant_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-inactive :slant)");
}

#[test]
fn div_face_slant_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-argument-name :slant)");
}

#[test]
fn div_face_slant_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-for-help-header :slant)");
}

#[test]
fn div_face_slant_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-key-binding :slant)");
}

#[test]
fn div_face_slant_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'highlight :slant)");
}

#[test]
fn div_face_slant_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'homoglyph :slant)");
}

#[test]
fn div_face_slant_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'internal-border :slant)");
}

#[test]
fn div_face_slant_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch :slant)");
}

#[test]
fn div_face_slant_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-fail :slant)");
}

#[test]
fn div_face_slant_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-1 :slant)");
}

#[test]
fn div_face_slant_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-2 :slant)");
}

#[test]
fn div_face_slant_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'italic :slant)");
}

#[test]
fn div_face_slant_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'lazy-highlight :slant)");
}

#[test]
fn div_face_slant_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number :slant)");
}

#[test]
fn div_face_slant_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-current-line :slant)");
}

#[test]
fn div_face_slant_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-major-tick :slant)");
}

#[test]
fn div_face_slant_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-minor-tick :slant)");
}

#[test]
fn div_face_slant_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link :slant)");
}

#[test]
fn div_face_slant_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link-visited :slant)");
}

#[test]
fn div_face_slant_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'margin :slant)");
}

#[test]
fn div_face_slant_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'match :slant)");
}

#[test]
fn div_face_slant_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'menu :slant)");
}

#[test]
fn div_face_slant_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-nonselected :slant)");
}

#[test]
fn div_face_slant_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-prompt :slant)");
}

#[test]
fn div_face_slant_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line :slant)");
}

#[test]
fn div_face_slant_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-active :slant)");
}

#[test]
fn div_face_slant_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-buffer-id :slant)");
}

#[test]
fn div_face_slant_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-emphasis :slant)");
}

#[test]
fn div_face_slant_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-highlight :slant)");
}

#[test]
fn div_face_slant_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-inactive :slant)");
}

#[test]
fn div_face_slant_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse :slant)");
}

#[test]
fn div_face_slant_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse-drag-and-drop-region :slant)");
}

#[test]
fn div_face_slant_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error :slant)");
}

#[test]
fn div_face_slant_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error-message :slant)");
}

#[test]
fn div_face_slant_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-hyphen :slant)");
}

#[test]
fn div_face_slant_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-space :slant)");
}

#[test]
fn div_face_slant_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'query-replace :slant)");
}

#[test]
fn div_face_slant_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'read-multiple-choice-face :slant)");
}

#[test]
fn div_face_slant_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'region :slant)");
}

#[test]
fn div_face_slant_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'scroll-bar :slant)");
}

#[test]
fn div_face_slant_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'secondary-selection :slant)");
}

#[test]
fn div_face_slant_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'separator-line :slant)");
}

#[test]
fn div_face_slant_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'shadow :slant)");
}

#[test]
fn div_face_slant_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match :slant)");
}

#[test]
fn div_face_slant_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match-expression :slant)");
}

#[test]
fn div_face_slant_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-mismatch :slant)");
}

#[test]
fn div_face_slant_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'success :slant)");
}

#[test]
fn div_face_slant_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar :slant)");
}

#[test]
fn div_face_slant_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab :slant)");
}

#[test]
fn div_face_slant_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-current :slant)");
}

#[test]
fn div_face_slant_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-inactive :slant)");
}

#[test]
fn div_face_slant_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-highlight :slant)");
}

#[test]
fn div_face_slant_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-inactive :slant)");
}

#[test]
fn div_face_slant_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-ungrouped :slant)");
}

#[test]
fn div_face_slant_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line :slant)");
}

#[test]
fn div_face_slant_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-active :slant)");
}

#[test]
fn div_face_slant_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-inactive :slant)");
}

#[test]
fn div_face_slant_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tabulated-list-fake-header :slant)");
}

#[test]
fn div_face_slant_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tool-bar :slant)");
}

#[test]
fn div_face_slant_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tooltip :slant)");
}

#[test]
fn div_face_slant_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'trailing-whitespace :slant)");
}

#[test]
fn div_face_slant_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-disabled-face :slant)");
}

#[test]
fn div_face_slant_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-enabled-face :slant)");
}

#[test]
fn div_face_slant_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-selected-face :slant)");
}

#[test]
fn div_face_slant_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'underline :slant)");
}

#[test]
fn div_face_slant_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch :slant)");
}

#[test]
fn div_face_slant_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch-text :slant)");
}

#[test]
fn div_face_slant_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-conflict-state :slant)");
}

#[test]
fn div_face_slant_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-edited-state :slant)");
}

#[test]
fn div_face_slant_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-ignored-state :slant)");
}

#[test]
fn div_face_slant_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locally-added-state :slant)");
}

#[test]
fn div_face_slant_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locked-state :slant)");
}

#[test]
fn div_face_slant_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-missing-state :slant)");
}

#[test]
fn div_face_slant_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-needs-update-state :slant)");
}

#[test]
fn div_face_slant_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-removed-state :slant)");
}

#[test]
fn div_face_slant_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-state-base :slant)");
}

#[test]
fn div_face_slant_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-up-to-date-state :slant)");
}

#[test]
fn div_face_slant_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vertical-border :slant)");
}

#[test]
fn div_face_slant_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'warning :slant)");
}

#[test]
fn div_face_slant_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider :slant)");
}

#[test]
fn div_face_slant_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-first-pixel :slant)");
}

#[test]
fn div_face_slant_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-last-pixel :slant)");
}
