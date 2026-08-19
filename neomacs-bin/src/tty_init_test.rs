use super::{StringCapability, TerminalCapabilityDatabase};
use super::{
    terminal_size_from_env_values, tty_color_cells, tty_enter_sequence, tty_erase_char_value,
    tty_leave_sequence,
};
use neovm_core::emacs_core::value::Value;

/// A terminal database that answers exactly one capability, so the colour-cell
/// rule can be measured against terminfo entries this machine need not have.
struct ColorsOnlyDatabase(Option<i32>);

impl TerminalCapabilityDatabase for ColorsOnlyDatabase {
    fn get_string(&mut self, _cap: StringCapability<'_>) -> Option<Vec<u8>> {
        None
    }

    fn get_termcap_number(&mut self, cap: &str) -> Option<i32> {
        (cap == "Co").then_some(self.0).flatten()
    }

    fn get_termcap_flag(&mut self, _cap: &str) -> bool {
        false
    }
}

fn database(
    colors: Option<i32>,
) -> impl FnOnce(&str) -> Option<Box<dyn TerminalCapabilityDatabase>> {
    move |_term| Some(Box::new(ColorsOnlyDatabase(colors)) as Box<dyn TerminalCapabilityDatabase>)
}

fn no_database(_term: &str) -> Option<Box<dyn TerminalCapabilityDatabase>> {
    None
}

/// GNU reads the colour count out of the terminal database -- `init_tty` does
/// `tty->TN_max_colors = tgetnum ("Co")` (src/term.c) -- never out of the TERM
/// name, and that number decides how many entries `tty-color-alist` gets and
/// which `((class color) (min-colors N) ...)` specs match.
///
/// Measured in a PTY with COLORTERM unset, both editors:
///   TERM=rxvt-16color   GNU => cells 16, alist 16;  Neomacs before => 8, 8
///   TERM=linux-16color  GNU => cells 16, alist 8;   Neomacs before => 8, 8
#[test]
fn color_cells_come_from_the_terminal_database_not_the_name() {
    assert_eq!(tty_color_cells("", "rxvt-16color", database(Some(16))), 16);
    assert_eq!(tty_color_cells("", "linux-16color", database(Some(16))), 16);
    assert_eq!(
        tty_color_cells("", "screen-256color", database(Some(256))),
        256
    );
    assert_eq!(tty_color_cells("", "xterm", database(Some(8))), 8);
    // A name that says 256 does not make it so: the entry is the authority.
    assert_eq!(
        tty_color_cells("", "wrapper-256color", database(Some(8))),
        8
    );
}

/// GNU treats `tgetnum ("Co")` == -1 as "no colours", and a monochrome entry is
/// not a reason to guess from the name either.
#[test]
fn a_terminal_that_reports_no_colors_has_none() {
    assert_eq!(tty_color_cells("", "vt100", database(None)), 0);
    assert_eq!(tty_color_cells("", "vt100", database(Some(-1))), 0);
    assert_eq!(tty_color_cells("", "vt100", database(Some(0))), 0);
}

/// COLORTERM stays ahead of the database: no terminfo entry describes 24-bit
/// colour, so the environment is the only place it is announced.  `dumb` and an
/// unset TERM answer 0 without consulting anything, as GNU's dumb-terminal
/// fallback does.
#[test]
fn colorterm_wins_and_dumb_terminals_never_consult_the_database() {
    assert_eq!(
        tty_color_cells("truecolor", "screen-256color", database(Some(256))),
        16_777_216
    );
    assert_eq!(
        tty_color_cells("24bit", "xterm", database(Some(8))),
        16_777_216
    );
    assert_eq!(tty_color_cells("", "dumb", database(Some(8))), 0);
    assert_eq!(tty_color_cells("", "", database(Some(8))), 0);
}

/// The name heuristic survives only where GNU would have refused to start at
/// all ("Terminal type X is not defined"), so it is a fallback, not the rule.
#[test]
fn an_unreadable_entry_falls_back_to_the_name() {
    assert_eq!(tty_color_cells("", "screen-256color", no_database), 256);
    assert_eq!(tty_color_cells("", "rxvt-16color", no_database), 8);
}

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
