use super::{terminal_size_from_env_values, tty_enter_sequence, tty_leave_sequence};

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
