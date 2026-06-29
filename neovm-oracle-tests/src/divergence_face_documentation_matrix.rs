//! Per-face *face-documentation* matrix (all GNU faces).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_doc_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'abbrev-table-name)",
        expect_test::expect![[
            r#""OK \"Face used for displaying the abbrev table name in ‘edit-abbrevs-mode’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'blink-matching-paren-offscreen)",
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_face_doc_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'bold)",
        expect_test::expect![[r#""OK \"Basic bold face.\"""#]],
    );
}

#[test]
fn div_face_doc_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'bold-italic)",
        expect_test::expect![[r#""OK \"Basic bold-italic face.\"""#]],
    );
}

#[test]
fn div_face_doc_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'border)",
        expect_test::expect![[r#""OK \"Basic face for the frame border under X.\"""#]],
    );
}

#[test]
fn div_face_doc_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'buffer-menu-buffer)",
        expect_test::expect![[r#""OK \"Face for buffer names in the Buffer Menu.\"""#]],
    );
}

#[test]
fn div_face_doc_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'button)",
        expect_test::expect![[r#""OK \"Default face used for buttons.\"""#]],
    );
}

#[test]
fn div_face_doc_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'child-frame-border)",
        expect_test::expect![[
            r#""OK \"Basic face for the internal border of child frames.\nFor the internal border of non-child frames see ‘internal-border’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'completions-annotations)",
        expect_test::expect![[
            r#""OK \"Face to use for annotations in the *Completions* buffer.\nThis face is only used if the strings used for completions\ndoesn’t already specify a face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'completions-common-part)",
        expect_test::expect![[
            r#""OK \"Face for the parts of completions which matched the pattern.\nSee also the face ‘completions-first-difference’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'completions-first-difference)",
        expect_test::expect![[
            r#""OK \"Face for the first character after point in completions.\nSee also the face ‘completions-common-part’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'completions-group-separator)",
        expect_test::expect![[
            r#""OK \"Face used for the separator lines between the candidate groups.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'completions-group-title)",
        expect_test::expect![[
            r#""OK \"Face used for the title text of the candidate group headlines.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'completions-highlight)",
        expect_test::expect![[
            r#""OK \"Default face for highlighting the current completion candidate.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'confusingly-reordered)",
        expect_test::expect![[
            r#""OK \"Face for highlighting text that was bidi-reordered in confusing ways.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'cursor)",
        expect_test::expect![[
            r#""OK \"Basic face for the cursor color under X.\nCurrently, only the ‘:background’ attribute is meaningful; all\nother attributes are ignored.  The cursor foreground color is\ntaken from the background color of the underlying text.\n\nNote: Other faces cannot inherit from the cursor face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'default)",
        expect_test::expect![[r#""OK \"Basic default face.\"""#]],
    );
}

#[test]
fn div_face_doc_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'eldoc-highlight-function-argument)",
        expect_test::expect![[
            r#""OK \"Face used for the argument at point in a function’s argument list.\nNote that this face has no effect unless the ‘eldoc-documentation-strategy’\nhandles it explicitly.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-ampersand)",
        expect_test::expect![[
            r#""OK \"Face for highlighting argument list markers, such as ‘&optional’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-binding-variable)",
        expect_test::expect![[
            r#""OK \"Face for highlighting binding occurrences of variables in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-bound-variable)",
        expect_test::expect![[
            r#""OK \"Face for highlighting bound occurrences of variables in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-charset)",
        expect_test::expect![[
            r#""OK \"Face for highlighting charset names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-coding)",
        expect_test::expect![[
            r#""OK \"Face for highlighting coding system names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-completion-category)",
        expect_test::expect![[
            r#""OK \"Face for highlighting completion category names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-completion-category-definition)",
        expect_test::expect![[
            r#""OK \"Face for highlighting completion category definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-condition)",
        expect_test::expect![[
            r#""OK \"Face for highlighting ‘condition-case’ conditions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-constant)",
        expect_test::expect![[
            r#""OK \"Face for highlighting self-evaluating symbols in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defcharset)",
        expect_test::expect![[
            r#""OK \"Face for highlighting charset definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defcoding)",
        expect_test::expect![[
            r#""OK \"Face for highlighting coding system definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defface)",
        expect_test::expect![[
            r#""OK \"Face for highlighting face definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-deficon)",
        expect_test::expect![[
            r#""OK \"Face for highlighting icon definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defmacro)",
        expect_test::expect![[
            r#""OK \"Face for highlighting macro definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defoclosure)",
        expect_test::expect![[
            r#""OK \"Face for highlighting OClosure type definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defun)",
        expect_test::expect![[
            r#""OK \"Face for highlighting function definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-defvar)",
        expect_test::expect![[
            r#""OK \"Face for highlighting variable definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-face)",
        expect_test::expect![[r#""OK \"Face for highlighting face names in Emacs Lisp code.\"""#]],
    );
}

#[test]
fn div_face_doc_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-feature)",
        expect_test::expect![[
            r#""OK \"Face for highlighting feature names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-free-variable)",
        expect_test::expect![[
            r#""OK \"Face for highlighting free (special) variables in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-function)",
        expect_test::expect![[
            r#""OK \"Face for highlighting function calls in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-function-property-declaration)",
        expect_test::expect![[
            r#""OK \"Face for highlighting function/macro property declaration type names.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-group)",
        expect_test::expect![[
            r#""OK \"Face for highlighting customization group names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-icon)",
        expect_test::expect![[r#""OK \"Face for highlighting icon names in Emacs Lisp code.\"""#]],
    );
}

#[test]
fn div_face_doc_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-macro)",
        expect_test::expect![[r#""OK \"Face for highlighting macro calls in Emacs Lisp code.\"""#]],
    );
}

#[test]
fn div_face_doc_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-major-mode-name)",
        expect_test::expect![[
            r#""OK \"Face for highlighting major mode names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-nnoo-backend)",
        expect_test::expect![[
            r#""OK \"Face for highlighting ‘nnoo’ backend names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-non-local-exit)",
        expect_test::expect![[
            r#""OK \"Face for highlighting calls to functions that do not return.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-oclosure)",
        expect_test::expect![[
            r#""OK \"Face for highlighting OClosure type names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-rx)",
        expect_test::expect![[
            r#""OK \"Face for highlighting ‘rx’ constructs in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-shadowed-variable)",
        expect_test::expect![[
            r#""OK \"Face for highlighting special variables that are shadowed by a local binding.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-shadowing-variable)",
        expect_test::expect![[
            r#""OK \"Face for highlighting local bindings that shadow special variables.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-shorthand-font-lock-face)",
        expect_test::expect![[r#""OK \"Face for highlighting shorthands in Emacs Lisp.\"""#]],
    );
}

#[test]
fn div_face_doc_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-slot)",
        expect_test::expect![[r#""OK \"Face for highlighting EIEIO slot names.\"""#]],
    );
}

#[test]
fn div_face_doc_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-special-form)",
        expect_test::expect![[
            r#""OK \"Face for highlighting special forms in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-special-variable-declaration)",
        expect_test::expect![[
            r#""OK \"Face for highlighting free variable declarations in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-symbol-at-mouse)",
        expect_test::expect![[
            r#""OK \"Face for highlighting the symbol at mouse in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-symbol-role)",
        expect_test::expect![[
            r#""OK \"Face for highlighting symbol role names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-symbol-role-definition)",
        expect_test::expect![[
            r#""OK \"Face for highlighting symbol role definitions in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-theme)",
        expect_test::expect![[
            r#""OK \"Face for highlighting custom theme names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-thing)",
        expect_test::expect![[
            r#""OK \"Face for highlighting ‘thing-at-point’ \\\"thing\\\" names in Emacs Lisp.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-throw-tag)",
        expect_test::expect![[
            r#""OK \"Face for highlighting ‘catch’/‘throw’ tags in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-type)",
        expect_test::expect![[
            r#""OK \"Face for highlighting object type names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-unknown-call)",
        expect_test::expect![[
            r#""OK \"Face for highlighting unknown functions/macros in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-variable-at-point)",
        expect_test::expect![[
            r#""OK \"Face for highlighting (all occurrences of) the variable at point.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-warning-type)",
        expect_test::expect![[
            r#""OK \"Face for highlighting byte-compilation warning type names in Emacs Lisp.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'elisp-widget-type)",
        expect_test::expect![[
            r#""OK \"Face for highlighting widget type names in Emacs Lisp code.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'error)",
        expect_test::expect![[
            r#""OK \"Basic face used to highlight errors and to denote failure.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'escape-glyph)",
        expect_test::expect![[
            r#""OK \"Face for characters displayed as sequences using ‘^’ or ‘\\\\’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'file-name-shadow)",
        expect_test::expect![[r#""OK \"Face used by ‘file-name-shadow-mode’ for the shadow.\"""#]],
    );
}

#[test]
fn div_face_doc_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'fill-column-indicator)",
        expect_test::expect![[
            r#""OK \"Face for displaying fill column indicator.\nThis face is used when ‘display-fill-column-indicator-mode’ is\nnon-nil.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'fixed-pitch)",
        expect_test::expect![[r#""OK \"The basic fixed-pitch face.\"""#]],
    );
}

#[test]
fn div_face_doc_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'fixed-pitch-serif)",
        expect_test::expect![[r#""OK \"The basic fixed-pitch face with serifs.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-bracket-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight brackets, braces, and parens.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-builtin-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight builtins.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-comment-delimiter-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight comment delimiters.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-comment-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight comments.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-constant-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight constants and labels.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-delimiter-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight delimiters.\nWhat exactly is a delimiter depends on the major mode, but usually\nthese are characters like comma, colon, and semi-colon.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-doc-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight documentation embedded in program code.\nIt is typically used for special documentation comments or strings.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-doc-markup-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight embedded documentation mark-up.\nIt is meant for mark-up elements in text that uses ‘font-lock-doc-face’, such\nas the constructs of Haddock, Javadoc and similar systems.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-escape-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight escape sequences in strings.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-function-call-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight function calls.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-function-name-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight function names.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-keyword-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight keywords.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-misc-punctuation-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight miscellaneous punctuation.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-negation-char-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight easy to overlook negation.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-number-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight numbers.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-operator-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight operators.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-preprocessor-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight preprocessor directives.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-property-name-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight properties of an object.\nFor example, the declaration of fields in a struct.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-property-use-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight property references.\nFor example, property lookup of fields in a struct.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-punctuation-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight punctuation characters.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-regexp-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight regexp literals.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-regexp-grouping-backslash)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face for backslashes in Lisp regexp grouping constructs.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-regexp-grouping-construct)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight grouping constructs in Lisp regexps.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-string-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight strings.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-type-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight type and class names.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-variable-name-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight variable names.\"""#]],
    );
}

#[test]
fn div_face_doc_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-variable-use-face)",
        expect_test::expect![[
            r#""OK \"Font Lock mode face used to highlight variable references.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'font-lock-warning-face)",
        expect_test::expect![[r#""OK \"Font Lock mode face used to highlight warnings.\"""#]],
    );
}

#[test]
fn div_face_doc_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'fringe)",
        expect_test::expect![[
            r#""OK \"Basic face for the fringes to the left and right of windows under X.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'glyphless-char)",
        expect_test::expect![[
            r#""OK \"Face for displaying non-graphic characters (e.g. U+202A (LRE)).\nIt is used for characters of no fonts too.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'header-line)",
        expect_test::expect![[r#""OK \"Basic header-line face.\"""#]],
    );
}

#[test]
fn div_face_doc_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'header-line-active)",
        expect_test::expect![[
            r#""OK \"Face for the selected header line.\nThis inherits from the ‘header-line’ face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'header-line-highlight)",
        expect_test::expect![[r#""OK \"Basic header line face for highlighting.\"""#]],
    );
}

#[test]
fn div_face_doc_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'header-line-inactive)",
        expect_test::expect![[r#""OK \"Basic header line face for non-selected windows.\"""#]],
    );
}

#[test]
fn div_face_doc_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'help-argument-name)",
        expect_test::expect![[r#""OK \"Face to highlight argument names in *Help* buffers.\"""#]],
    );
}

#[test]
fn div_face_doc_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'help-for-help-header)",
        expect_test::expect![[r#""OK \"Face used for headers in the ‘help-for-help’ buffer.\"""#]],
    );
}

#[test]
fn div_face_doc_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'help-key-binding)",
        expect_test::expect![[
            r#""OK \"Face for keybindings in *Help* buffers.\n\nThis face is added by ‘substitute-command-keys’, which see.\n\nNote that this face will also be used for key bindings in\ntooltips.  This means that, for example, changing the :height of\nthis face will increase the height of any tooltip containing key\nbindings.  See also the face ‘tooltip’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'highlight)",
        expect_test::expect![[r#""OK \"Basic face for highlighting.\"""#]],
    );
}

#[test]
fn div_face_doc_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'homoglyph)",
        expect_test::expect![[r#""OK \"Face for lookalike characters.\"""#]],
    );
}

#[test]
fn div_face_doc_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'internal-border)",
        expect_test::expect![[
            r#""OK \"Basic face for the internal border.\nFor the internal border of child frames see ‘child-frame-border’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'isearch)",
        expect_test::expect![[r#""OK \"Face for highlighting Isearch matches.\"""#]],
    );
}

#[test]
fn div_face_doc_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'isearch-fail)",
        expect_test::expect![[
            r#""OK \"Face for highlighting failed part in Isearch echo-area message.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'isearch-group-1)",
        expect_test::expect![[r#""OK \"Face for highlighting Isearch the odd group matches.\"""#]],
    );
}

#[test]
fn div_face_doc_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'isearch-group-2)",
        expect_test::expect![[r#""OK \"Face for highlighting Isearch the even group matches.\"""#]],
    );
}

#[test]
fn div_face_doc_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'italic)",
        expect_test::expect![[r#""OK \"Basic italic face.\"""#]],
    );
}

#[test]
fn div_face_doc_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'lazy-highlight)",
        expect_test::expect![[
            r#""OK \"Face for lazy highlighting of matches other than the current one.\nUsed in Isearch when ‘isearch-lazy-highlight’ is non-nil,\nand in ‘query-replace’ when ‘query-replace-lazy-highlight’ is non-nil.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'line-number)",
        expect_test::expect![[
            r#""OK \"Face for displaying line numbers.\nThis face is used when ‘display-line-numbers’ is non-nil.\n\nIf you customize the font of this face, make sure it is a\nmonospaced font, otherwise line numbers will not line up,\nand text lines might move horizontally as you move through\nthe buffer.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'line-number-current-line)",
        expect_test::expect![[
            r#""OK \"Face for displaying the current line number.\nThis face is used when ‘display-line-numbers’ is non-nil.\n\nIf you customize the font of this face, make sure it is a\nmonospaced font, otherwise line numbers will not line up,\nand text lines might move horizontally as you move through\nthe buffer.  Similarly, making this face’s font different\nfrom that of the ‘line-number’ face could produce such\nunwanted effects.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'line-number-major-tick)",
        expect_test::expect![[
            r#""OK \"Face for highlighting \\\"major ticks\\\" (as in a ruler).\nWhen ‘display-line-numbers-major-tick’ is positive, highlight\nthe line numbers of lines which are a multiple of its value.\nThis face is used when ‘display-line-numbers’ is non-nil.\n\nIf you customize the font of this face, make sure it is a\nmonospaced font, otherwise line numbers will not line up,\nand text lines might move horizontally as you move through\nthe buffer.  Similarly, making this face’s font different\nfrom that of the ‘line-number’ face could produce such\nunwanted effects.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'line-number-minor-tick)",
        expect_test::expect![[
            r#""OK \"Face for highlighting \\\"minor ticks\\\" (as in a ruler).\nWhen ‘display-line-numbers-minor-tick’ is positive, highlight\nthe line numbers of lines which are a multiple of its value.\nThis face is used when ‘display-line-numbers’ is non-nil.\n\nIf you customize the font of this face, make sure it is a\nmonospaced font, otherwise line numbers will not line up,\nand text lines might move horizontally as you move through\nthe buffer.  Similarly, making this face’s font different\nfrom that of the ‘line-number’ face could produce such\nunwanted effects.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'link)",
        expect_test::expect![[r#""OK \"Basic face for unvisited links.\"""#]],
    );
}

#[test]
fn div_face_doc_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'link-visited)",
        expect_test::expect![[r#""OK \"Basic face for visited links.\"""#]],
    );
}

#[test]
fn div_face_doc_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'margin)",
        expect_test::expect![[
            r#""OK \"Basic face for window margins (both left and right).\nThis face is used to customize the appearance of the margin areas.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'match)",
        expect_test::expect![[r#""OK \"Face used to highlight matches permanently.\"""#]],
    );
}

#[test]
fn div_face_doc_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'menu)",
        expect_test::expect![[
            r#""OK \"Basic face for the font and colors of the menu bar and popup menus.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'minibuffer-nonselected)",
        expect_test::expect![[
            r#""OK \"Face for highlighting contents of non-selected minibuffer window.\nUsed by ‘minibuffer-nonselected-mode’ for the contents of the minibuffer\nwindow when the minibuffer remains active but its window is currently\nnot selected.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'minibuffer-prompt)",
        expect_test::expect![[
            r#""OK \"Face for minibuffer prompts.\nBy default, Emacs automatically adds this face to the value of\n‘minibuffer-prompt-properties’, which is a list of text properties\nused to display the prompt text.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mode-line)",
        expect_test::expect![[
            r#""OK \"Face for the mode lines as well as header lines.\nSee ‘mode-line-active’ and ‘mode-line-inactive’ for the faces\nused on mode lines.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mode-line-active)",
        expect_test::expect![[
            r#""OK \"Face for the selected mode line.\nThis inherits from the ‘mode-line’ face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mode-line-buffer-id)",
        expect_test::expect![[
            r#""OK \"Face used for buffer identification parts of the mode line.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mode-line-emphasis)",
        expect_test::expect![[
            r#""OK \"Face used to emphasize certain mode line features.\nUse the face ‘mode-line-highlight’ for features that can be selected.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mode-line-highlight)",
        expect_test::expect![[r#""OK \"Basic mode line face for highlighting.\"""#]],
    );
}

#[test]
fn div_face_doc_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mode-line-inactive)",
        expect_test::expect![[r#""OK \"Basic mode line face for non-selected windows.\"""#]],
    );
}

#[test]
fn div_face_doc_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mouse)",
        expect_test::expect![[r#""OK \"Basic face for the mouse color under X.\"""#]],
    );
}

#[test]
fn div_face_doc_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'mouse-drag-and-drop-region)",
        expect_test::expect![[
            r#""OK \"Face to highlight original text during dragging.\nThis face is used by ‘mouse-drag-and-drop-region’ to temporarily\nhighlight the original region when\n‘mouse-drag-and-drop-region-show-cursor’ is non-nil.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'next-error)",
        expect_test::expect![[r#""OK \"Face used to highlight next error locus.\"""#]],
    );
}

#[test]
fn div_face_doc_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'next-error-message)",
        expect_test::expect![[
            r#""OK \"Face used to highlight the current error message in the ‘next-error’ buffer.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'nobreak-hyphen)",
        expect_test::expect![[r#""OK \"Face for displaying nobreak hyphens.\"""#]],
    );
}

#[test]
fn div_face_doc_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'nobreak-space)",
        expect_test::expect![[r#""OK \"Face for displaying nobreak space.\"""#]],
    );
}

#[test]
fn div_face_doc_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'query-replace)",
        expect_test::expect![[
            r#""OK \"Face for highlighting query replacement matches.\nUsed in ‘query-replace’ and ‘query-replace-regexp’\nwhen ‘query-replace-highlight’ is non-nil\"""#
        ]],
    );
}

#[test]
fn div_face_doc_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'read-multiple-choice-face)",
        expect_test::expect![[
            r#""OK \"Face for the symbol name in ‘read-multiple-choice’ output.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'region)",
        expect_test::expect![[r#""OK \"Basic face for highlighting the region.\"""#]],
    );
}

#[test]
fn div_face_doc_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'scroll-bar)",
        expect_test::expect![[r#""OK \"Basic face for the scroll bar colors under X.\"""#]],
    );
}

#[test]
fn div_face_doc_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'secondary-selection)",
        expect_test::expect![[r#""OK \"Basic face for displaying the secondary selection.\"""#]],
    );
}

#[test]
fn div_face_doc_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'separator-line)",
        expect_test::expect![[r#""OK \"Face for separator lines.\"""#]],
    );
}

#[test]
fn div_face_doc_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'shadow)",
        expect_test::expect![[r#""OK \"Basic face for shadowed text.\"""#]],
    );
}

#[test]
fn div_face_doc_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'show-paren-match)",
        expect_test::expect![[r#""OK \"Face used for a matching paren.\"""#]],
    );
}

#[test]
fn div_face_doc_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'show-paren-match-expression)",
        expect_test::expect![[
            r#""OK \"Face used for a matching paren when highlighting the whole expression.\nThis face is used by ‘show-paren-mode’.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'show-paren-mismatch)",
        expect_test::expect![[r#""OK \"Face used for a mismatching paren.\"""#]],
    );
}

#[test]
fn div_face_doc_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'success)",
        expect_test::expect![[r#""OK \"Basic face used to indicate successful operation.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar)",
        expect_test::expect![[r#""OK \"Tab bar face.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar-tab)",
        expect_test::expect![[r#""OK \"Tab bar face for selected tab.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar-tab-group-current)",
        expect_test::expect![[r#""OK \"Tab bar face for current group tab.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar-tab-group-inactive)",
        expect_test::expect![[r#""OK \"Tab bar face for inactive group tab.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar-tab-highlight)",
        expect_test::expect![[r#""OK \"Tab bar face for highlighting.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar-tab-inactive)",
        expect_test::expect![[r#""OK \"Tab bar face for non-selected tab.\"""#]],
    );
}

#[test]
fn div_face_doc_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-bar-tab-ungrouped)",
        expect_test::expect![[
            r#""OK \"Tab bar face for ungrouped tab when tab groups are used.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-line)",
        expect_test::expect![[
            r#""OK \"Basic tab line face.\nSee ‘tab-line-active’ and ‘tab-line-inactive’ for the faces\nused on tab lines.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-line-active)",
        expect_test::expect![[
            r#""OK \"Face for the selected tab line.\nThis inherits from the ‘tab-line’ face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tab-line-inactive)",
        expect_test::expect![[r#""OK \"Basic tab line face for non-selected windows.\"""#]],
    );
}

#[test]
fn div_face_doc_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tabulated-list-fake-header)",
        expect_test::expect![[r#""OK \"Face used on fake header lines.\"""#]],
    );
}

#[test]
fn div_face_doc_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tool-bar)",
        expect_test::expect![[r#""OK \"Basic tool-bar face.\"""#]],
    );
}

#[test]
fn div_face_doc_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tooltip)",
        expect_test::expect![[
            r#""OK \"Face for tooltips.\n\nWhen using the GTK toolkit, NS, or Haiku, this face will only\nbe used if ‘use-system-tooltips’ is nil.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'trailing-whitespace)",
        expect_test::expect![[r#""OK \"Basic face for highlighting trailing whitespace.\"""#]],
    );
}

#[test]
fn div_face_doc_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tty-menu-disabled-face)",
        expect_test::expect![[r#""OK \"Face for displaying disabled items in TTY menus.\"""#]],
    );
}

#[test]
fn div_face_doc_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tty-menu-enabled-face)",
        expect_test::expect![[r#""OK \"Face for displaying enabled items in TTY menus.\"""#]],
    );
}

#[test]
fn div_face_doc_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'tty-menu-selected-face)",
        expect_test::expect![[
            r#""OK \"Face for displaying the currently selected item in TTY menus.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'underline)",
        expect_test::expect![[r#""OK \"Basic underlined face.\"""#]],
    );
}

#[test]
fn div_face_doc_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'variable-pitch)",
        expect_test::expect![[r#""OK \"The basic variable-pitch face.\"""#]],
    );
}

#[test]
fn div_face_doc_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'variable-pitch-text)",
        expect_test::expect![[
            r#""OK \"The proportional face used for longer texts.\nThis is like the ‘variable-pitch’ face, but is slightly bigger by\ndefault.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-conflict-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file contains merge conflicts.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-edited-state)",
        expect_test::expect![[r#""OK \"Face for VC modeline state when the file is edited.\"""#]],
    );
}

#[test]
fn div_face_doc_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-ignored-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file is registered, but ignored.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-locally-added-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file is locally added.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-locked-state)",
        expect_test::expect![[r#""OK \"Face for VC modeline state when the file locked.\"""#]],
    );
}

#[test]
fn div_face_doc_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-missing-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file is missing from the file system.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-needs-update-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file needs update.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-removed-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file was removed from the VC system.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-state-base)",
        expect_test::expect![[r#""OK \"Base face for VC state indicator.\"""#]],
    );
}

#[test]
fn div_face_doc_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vc-up-to-date-state)",
        expect_test::expect![[
            r#""OK \"Face for VC modeline state when the file is up to date.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'vertical-border)",
        expect_test::expect![[r#""OK \"Face used for vertical window dividers on ttys.\"""#]],
    );
}

#[test]
fn div_face_doc_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'warning)",
        expect_test::expect![[r#""OK \"Basic face used to highlight warnings.\"""#]],
    );
}

#[test]
fn div_face_doc_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'window-divider)",
        expect_test::expect![[
            r#""OK \"Basic face for window dividers.\nWhen a divider is less than 3 pixels wide, it is drawn solidly\nwith the foreground of this face.  For larger dividers this face\nis used for the inner part while the first pixel line/column is\ndrawn with the ‘window-divider-first-pixel’ face and the last\npixel line/column with the ‘window-divider-last-pixel’ face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'window-divider-first-pixel)",
        expect_test::expect![[
            r#""OK \"Basic face for first pixel line/column of window dividers.\nWhen a divider is at least 3 pixels wide, its first pixel\nline/column is drawn with the foreground of this face.  If you do\nnot want to accentuate the first pixel line/column, set this to\nthe same as ‘window-divider’ face.\"""#
        ]],
    );
}

#[test]
fn div_face_doc_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(face-documentation 'window-divider-last-pixel)",
        expect_test::expect![[
            r#""OK \"Basic face for last pixel line/column of window dividers.\nWhen a divider is at least 3 pixels wide, its last pixel\nline/column is drawn with the foreground of this face.  If you do\nnot want to accentuate the last pixel line/column, set this to\nthe same as ‘window-divider’ face.\"""#
        ]],
    );
}
