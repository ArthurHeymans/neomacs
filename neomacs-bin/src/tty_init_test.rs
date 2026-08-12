use super::{
    terminal_size_from_env_values, tty_enter_sequence, tty_erase_char_value, tty_leave_sequence,
};
use neovm_core::emacs_core::value::Value;

#[test]
fn terminal_size_from_env_values_uses_positive_columns_and_lines() {
    assert_eq!(
        terminal_size_from_env_values(Some("160".to_string()), Some("50".to_string())),
        Some((160, 50))
    );
}

#[test]
fn terminal_size_from_env_values_rejects_missing_zero_or_invalid_values() {
    assert_eq!(
        terminal_size_from_env_values(None, Some("50".to_string())),
        None
    );
    assert_eq!(
        terminal_size_from_env_values(Some("160".to_string()), Some("0".to_string())),
        None
    );
    assert_eq!(
        terminal_size_from_env_values(Some("wide".to_string()), Some("50".to_string())),
        None
    );
}

#[test]
fn tty_lifecycle_enables_and_restores_gnu_input_modes() {
    assert_eq!(
        tty_enter_sequence(),
        b"\x1b[?1049h\x1b=\x1b[?1h\x1b[?2004h\x1b[?25l\x1b[2J"
    );
    assert_eq!(
        tty_leave_sequence(),
        b"\x1b[0m\x1b[?25h\x1b[?2004l\x1b[?1l\x1b>\x1b[?1049l"
    );
}

#[test]
fn tty_erase_char_value_mirrors_init_sys_modes() {
    // GNU src/sysdep.c init_sys_modes starts Vtty_erase_char at Qnil (1112)
    // and assigns c_cc[VERASE] only once it has a live tty (1130). Off a
    // terminal the value stays nil rather than becoming a number, which is
    // what `normal-erase-is-backspace-setup-frame' compares against ?\^H.
    assert_eq!(tty_erase_char_value(None), Value::NIL);
    // The two erase characters a terminal actually reports: DEL and C-h. On a
    // ^H terminal GNU enables normal-erase-is-backspace-mode and translates
    // C-h to DEL, so Backspace deletes instead of opening help.
    assert_eq!(tty_erase_char_value(Some(0x7f)), Value::fixnum(127));
    assert_eq!(tty_erase_char_value(Some(0x08)), Value::fixnum(8));
}
