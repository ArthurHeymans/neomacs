//! Complex combo batch 71 — keymap / event / command loop semantics: key
//! translations, `kbd`, `key-binding` lookup through active maps, `where-is`,
//! `read-key-sequence`, modifier decomposition, mouse event structures.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx71_kbd_parse_various_key_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (kbd "C-x C-f")
          (kbd "M-x")
          (kbd "C-M-a")
          (kbd "RET")
          (kbd "<f1>")
          (kbd "<M-down>")
          (kbd "C-c C-c")
          (kbd "C-u M-x")
          (kbd "C-h k")
          (kbd "M-<")
          (kbd "M->"))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx71_key_description_canonical_forms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (key-description (kbd "C-x C-f"))
 (key-description [(control ?x) (control ?f)])
 (key-description "\C-x\C-f")
 (single-key-description ?a)
 (single-key-description 'return)
 (single-key-description '(control ?x))
 (single-key-description '(meta ?x))
 (single-key-description '(control meta ?a)))
"##,
    );
}

#[test]
fn div_cx71_event_modifiers_and_basic_event_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (event-modifiers ?a)
 (event-modifiers 'C-a)
 (event-modifiers '(control ?a))
 (event-modifiers '(control meta ?a))
 (event-modifiers 'return)
 (event-modifiers 'C-return)
 (event-basic-type ?a)
 (event-basic-type 'C-a)
 (event-basic-type 'C-M-a)
 (event-basic-type 'return))
"##,
    );
}

#[test]
fn div_cx71_make_keymap_and_define_key_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap)))
  (define-key map (kbd "C-c C-a") 'neo-cx71-cmd-a)
  (define-key map (kbd "C-c C-b") 'neo-cx71-cmd-b)
  (define-key map (kbd "C-c C-c") 'neo-cx71-cmd-c)
  (define-key map (kbd "C-c C-d") 'neo-cx71-cmd-d)
  (list (lookup-key map (kbd "C-c C-a"))
        (lookup-key map (kbd "C-c C-b"))
        (lookup-key map (kbd "C-c C-x"))
        (where-is-internal 'neo-cx71-cmd-a map)
        (length map)))
"##,
    );
}

#[test]
fn div_cx71_keymap_prefix_command_nesting() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((outer (make-sparse-keymap))
      (inner (make-sparse-keymap)))
  (define-key inner "a" 'inner-a)
  (define-key inner "b" 'inner-b)
  (define-key outer "\C-c" inner)
  (list (keymapp outer)
        (keymapp inner)
        (lookup-key outer "\C-ca")
        (lookup-key outer "\C-cb")
        (lookup-key outer (kbd "C-c a"))
        (lookup-key outer (kbd "C-c x"))))
"##,
    );
}

#[test]
fn div_cx71_define_prefix_command_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((state nil))
  (define-prefix-command 'neo-cx71-prefix-cmd (symbol-value 'neo-cx71-map-var))
  (global-set-key "\C-c n" 'neo-cx71-prefix-cmd)
  (define-key 'neo-cx71-prefix-cmd "a" 'neo-cx71-action-a)
  (define-key 'neo-cx71-prefix-cmd "b" 'neo-cx71-action-b)
  (list (commandp 'neo-cx71-prefix-cmd)
        (lookup-key (current-global-map) "\C-cn")
        (lookup-key 'neo-cx71-prefix-cmd "a")
        (lookup-key 'neo-cx71-prefix-cmd "b")))
"##,
    );
}

#[test]
fn div_cx71_where_is_internal_with_remapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap)))
  (define-key map [remap neo-cx71-old] 'neo-cx71-new)
  (list (command-remapping 'neo-cx71-old map)
        (where-is-internal 'neo-cx71-old map)
        (lookup-key map [remap neo-cx71-old])))
"##,
    );
}

#[test]
fn div_cx71_mouse_event_structure_decomposition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mouse-event (list 'mouse-1
                         (posn-make (selected-window)
                                    '(0 . 0)
                                    (selected-window)
                                    1))))
  (list (event-basic-type mouse-event)
        (event-modifiers mouse-event)
        (event-start mouse-event)
        (posn-window (event-start mouse-event))
        (posn-point (event-start mouse-event))))
"##,
    );
}

#[test]
fn div_cx71_key_translation_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((tbl (make-keymap)))
      (define-key tbl [?\C-a] [?\C-x])
      (use-global-map tbl)
      (list (lookup-key tbl [?\C-a])))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx71_keymap_lookup_through_minor_mode_maps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap)))
  (define-key map (kbd "C-c C-c") 'neo-cx71-minor-cmd)
  (let ((minor-mode-map-alist (list (cons 'neo-cx71-minor map))))
    (list (assq 'neo-cx71-minor minor-mode-map-alist)
          (lookup-key map (kbd "C-c C-c"))
          (lookup-key map (kbd "C-c C-x")))))
"##,
    );
}

#[test]
fn div_cx71_accessible_keymaps_and_map_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap)))
  (define-key map "a" 'cmd-a)
  (define-key map "b" 'cmd-b)
  (let (collected)
    (map-keymap (lambda (key def) (push (cons key def) collected)) map)
    (sort collected (lambda (a b)
                      (string< (prin1-to-string (car a))
                               (prin1-to-string (car b)))))))
"##,
    );
}

#[test]
fn div_cx71_commandp_functionp_subrp_and_indirect_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((sym (defalias 'neo-cx71-alias (lambda () "doc" :result))))
  (list (commandp 'forward-char)
        (commandp 'neo-cx71-alias)
        (functionp 'neo-cx71-alias)
        (functionp (lambda () nil))
        (subrp (symbol-function 'car))
        (subrp (symbol-function 'forward-char))
        (indirect-function 'neo-cx71-alias)
        (fboundp 'car)
        (fboundp 'undefined-neo-cx71)))
"##,
    );
}
