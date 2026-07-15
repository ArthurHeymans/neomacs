//! TTY input reader.
//!
//! GNU Emacs reads Unix TTY keyboard input as raw bytes into its own keyboard
//! buffer (`tty_read_avail_input` in src/keyboard.c).  Keep the Unix path byte
//! based for the same semantics; use crossterm's parsed events only where the
//! platform does not expose the same Unix TTY model.

use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::thread_comm::{InputEvent, LifecycleCommand, RenderCommand, RenderComms};
#[cfg(not(unix))]
use crossterm::event::Event;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(unix)]
const GNU_KBD_BUFFER_SIZE: usize = 4096;

// Modifier masks — must match neomacs-display-runtime/src/backend/wgpu/events.rs
// SHIFT/CTRL/SUPER are consumed only by the non-unix crossterm key mapper below.
#[allow(dead_code)]
const NEOMACS_SHIFT_MASK: u32 = 1 << 0;
#[allow(dead_code)]
const NEOMACS_CTRL_MASK: u32 = 1 << 1;
const NEOMACS_META_MASK: u32 = 1 << 2;
#[allow(dead_code)]
const NEOMACS_SUPER_MASK: u32 = 1 << 3;

const XK_RETURN: u32 = 0xff0d;
const XK_TAB: u32 = 0xff09;
#[allow(dead_code)] // used only by the non-unix crossterm key mapper below
const XK_ESCAPE: u32 = 0xff1b;
const XK_LEFT: u32 = 0xff51;
const XK_UP: u32 = 0xff52;
const XK_RIGHT: u32 = 0xff53;
const XK_DOWN: u32 = 0xff54;
const XK_F1: u32 = 0xffbe;

// The crossterm `KeyEvent` -> `InputEvent` mapper (map_modifiers, without_control,
// tty_control_char_keysym, map_key_event) is wired into `read_tty_events` only on
// non-unix targets; on unix it is exercised solely by the tests, hence the allows.
/// Convert crossterm modifiers to our internal modifier mask.
#[allow(dead_code)]
fn map_modifiers(mods: KeyModifiers) -> u32 {
    let mut out = 0;
    if mods.contains(KeyModifiers::SHIFT) {
        out |= NEOMACS_SHIFT_MASK;
    }
    if mods.contains(KeyModifiers::CONTROL) {
        out |= NEOMACS_CTRL_MASK;
    }
    if mods.contains(KeyModifiers::ALT) {
        out |= NEOMACS_META_MASK;
    }
    if mods.contains(KeyModifiers::SUPER) {
        out |= NEOMACS_SUPER_MASK;
    }
    out
}

#[allow(dead_code)]
fn without_control(modifiers: u32) -> u32 {
    modifiers & !NEOMACS_CTRL_MASK
}

#[allow(dead_code)]
fn tty_control_char_keysym(c: char) -> Option<u32> {
    match c {
        '@' | '2' => Some(0x00),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '/' | '7' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

/// Map a crossterm key event to an `InputEvent::Key`.
///
/// Returns `None` for modifier-only keys (Shift, Ctrl, Alt, Super,
/// CapsLock, NumLock, etc.) — those are tracked by crossterm's modifier
/// state on subsequent key events, matching how winit delivers them.
#[allow(dead_code)]
fn map_key_event(event: KeyEvent) -> Option<InputEvent> {
    // Ignore key releases — Emacs only cares about press/repeat.
    if event.kind == KeyEventKind::Release {
        return None;
    }

    let mut modifiers = map_modifiers(event.modifiers);

    let keysym = match event.code {
        KeyCode::Char(c)
            if event.modifiers.contains(KeyModifiers::CONTROL)
                && tty_control_char_keysym(c).is_some() =>
        {
            modifiers = without_control(modifiers);
            tty_control_char_keysym(c)
        }
        KeyCode::Char(c) => Some(c as u32),
        KeyCode::F(n) if (1..=12).contains(&n) => {
            Some(0xffbe + (n as u32 - 1)) // F1=0xffbe … F12=0xffc9
        }
        KeyCode::F(_) => None, // unsupported function key
        KeyCode::Esc => Some(XK_ESCAPE),
        KeyCode::Enter => Some(XK_RETURN),
        KeyCode::Tab => Some(XK_TAB),
        KeyCode::Backspace => Some(0x7f),
        KeyCode::Delete => Some(0xffff),
        KeyCode::Insert => Some(0xff63),
        KeyCode::Home => Some(0xff50),
        KeyCode::End => Some(0xff57),
        KeyCode::PageUp => Some(0xff55),
        KeyCode::PageDown => Some(0xff56),
        KeyCode::Left => Some(XK_LEFT),
        KeyCode::Up => Some(XK_UP),
        KeyCode::Right => Some(XK_RIGHT),
        KeyCode::Down => Some(XK_DOWN),
        KeyCode::Null => Some(0x00),
        KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => None, // suppress bare modifier/media keys
        KeyCode::BackTab => Some(0xff09), // same as Tab, but with shift modifier
    };

    let keysym = keysym?;

    Some(InputEvent::Key {
        keysym,
        modifiers,
        pressed: event.kind == KeyEventKind::Press,
        emacs_frame_id: 0,
    })
}

fn raw_key_event(keysym: u32, modifiers: u32) -> InputEvent {
    InputEvent::Key {
        keysym,
        modifiers,
        pressed: true,
        emacs_frame_id: 0,
    }
}

#[cfg(unix)]
fn raw_tty_byte_keysym(byte: u8) -> u32 {
    match byte {
        b'\r' => XK_RETURN,
        b'\t' => XK_TAB,
        _ => byte as u32,
    }
}

#[cfg(unix)]
#[derive(Default)]
struct RawTtyDecoder {
    pending: VecDeque<u8>,
}

#[cfg(unix)]
impl RawTtyDecoder {
    fn push_bytes(&mut self, bytes: &[u8], events: &mut Vec<InputEvent>) {
        self.pending.extend(bytes);

        while !self.pending.is_empty() {
            if self.pending[0] != 0x1b {
                if !self.decode_plain_char(0, events) {
                    break;
                }
                continue;
            }

            if self.pending.len() == 1 {
                break;
            }

            if self.escape_sequence_incomplete() {
                break;
            }

            if self.try_escape_sequence(events) {
                continue;
            }

            if !self.decode_plain_char(NEOMACS_META_MASK, events) {
                break;
            }
        }
    }

    fn decode_plain_char(&mut self, modifiers: u32, events: &mut Vec<InputEvent>) -> bool {
        let start = usize::from(modifiers & NEOMACS_META_MASK != 0);
        let Some(&byte) = self.pending.get(start) else {
            return false;
        };

        if byte < 0x80 {
            let is_meta = modifiers & NEOMACS_META_MASK != 0;
            if is_meta {
                self.pending.drain(0..2);
            } else {
                self.pending.pop_front();
            }
            // An ESC-prefixed ASCII control byte is GNU's `meta-<char>`: on a
            // terminal, `ESC RET` is M-RET (meta + char 13), `ESC TAB` is M-TAB
            // (meta + char 9) -- NOT `M-<return>` / `M-<tab>` function keys. Only
            // the UNPREFIXED byte maps to the RET/TAB keysyms (whose `return`/
            // `tab` symbols `function-key-map` then translates back to 13/9);
            // there is no `[M-return]`/`[M-tab]` fkm entry, so a modified control
            // byte must carry the control character itself or bindings like
            // org-mode's M-RET (org-meta-return) never resolve.
            let keysym = if is_meta {
                u32::from(byte)
            } else {
                raw_tty_byte_keysym(byte)
            };
            events.push(raw_key_event(keysym, modifiers));
            return true;
        }

        let utf8_len = match byte {
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => {
                if modifiers & NEOMACS_META_MASK != 0 {
                    self.pending.drain(0..2);
                } else {
                    self.pending.pop_front();
                }
                events.push(raw_key_event(byte as u32, modifiers));
                return true;
            }
        };

        if self.pending.len() < start + utf8_len {
            return false;
        }

        let bytes: Vec<u8> = self
            .pending
            .iter()
            .skip(start)
            .take(utf8_len)
            .copied()
            .collect();
        match std::str::from_utf8(&bytes) {
            Ok(text) => {
                let ch = text.chars().next().expect("utf8 character");
                self.pending.drain(0..start + utf8_len);
                events.push(raw_key_event(ch as u32, modifiers));
            }
            Err(_) => {
                if modifiers & NEOMACS_META_MASK != 0 {
                    self.pending.drain(0..2);
                } else {
                    self.pending.pop_front();
                }
                events.push(raw_key_event(byte as u32, modifiers));
            }
        }

        true
    }

    fn try_escape_sequence(&mut self, events: &mut Vec<InputEvent>) -> bool {
        if self.pending.len() >= 3 && self.pending[1] == b'[' {
            let keysym = match self.pending[2] {
                b'A' => Some(XK_UP),
                b'B' => Some(XK_DOWN),
                b'C' => Some(XK_RIGHT),
                b'D' => Some(XK_LEFT),
                _ => None,
            };

            if let Some(keysym) = keysym {
                self.pending.drain(0..3);
                events.push(raw_key_event(keysym, 0));
                return true;
            }
        }

        if self.pending.len() >= 5
            && self.pending[1] == b'['
            && self.pending[2] == b'2'
            && self.pending[3] == b'1'
            && self.pending[4] == b'~'
        {
            self.pending.drain(0..5);
            events.push(raw_key_event(XK_F1 + 9, 0));
            return true;
        }

        false
    }

    fn escape_sequence_incomplete(&self) -> bool {
        if self.pending.len() < 2 || self.pending[0] != 0x1b || self.pending[1] != b'[' {
            return false;
        }

        if self.pending.len() < 3 {
            return true;
        }

        self.pending[2].is_ascii_digit() && self.pending.iter().skip(2).all(u8::is_ascii_digit)
    }
}

#[cfg(unix)]
fn read_tty_events(
    tx: crossbeam_channel::Sender<InputEvent>,
    stop: Arc<AtomicBool>,
    _paused: Arc<AtomicBool>,
) {
    let mut decoder = RawTtyDecoder::default();
    let mut last_size = crossterm::terminal::size().ok();

    while !stop.load(Ordering::Relaxed) {
        if let Ok(size) = crossterm::terminal::size()
            && last_size != Some(size)
        {
            last_size = Some(size);
            let event = InputEvent::WindowResize {
                width: size.0 as u32,
                height: size.1 as u32,
                scale_factor: 1.0,
                emacs_frame_id: 0,
            };
            if tx.send(event).is_err() {
                tracing::warn!("tty_input: channel closed");
                return;
            }
        }

        let mut pollfd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        let poll_result = unsafe { libc::poll(&mut pollfd, 1, 50) };
        if poll_result < 0 {
            let err = io::Error::last_os_error();
            if err.kind() != io::ErrorKind::Interrupted {
                tracing::warn!("tty_input: poll error: {}", err);
                thread::sleep(Duration::from_millis(100));
            }
            continue;
        }
        if poll_result == 0 || pollfd.revents & libc::POLLIN == 0 {
            continue;
        }

        let mut buf = [0u8; GNU_KBD_BUFFER_SIZE - 1];
        let nread = unsafe {
            libc::read(
                libc::STDIN_FILENO,
                buf.as_mut_ptr().cast::<libc::c_void>(),
                buf.len(),
            )
        };

        if nread < 0 {
            let err = io::Error::last_os_error();
            if err.kind() != io::ErrorKind::Interrupted && err.kind() != io::ErrorKind::WouldBlock {
                tracing::warn!("tty_input: read error: {}", err);
                thread::sleep(Duration::from_millis(100));
            }
            continue;
        }
        if nread == 0 {
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        let mut events = Vec::new();
        decoder.push_bytes(&buf[..nread as usize], &mut events);
        for event in events {
            tracing::debug!("tty_input: key event {:?}", event);
            if tx.send(event).is_err() {
                tracing::warn!("tty_input: channel closed");
                return;
            }
        }
    }
}

#[cfg(not(unix))]
fn read_tty_events(
    tx: crossbeam_channel::Sender<InputEvent>,
    stop: Arc<AtomicBool>,
    _paused: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        match crossterm::event::poll(Duration::from_millis(50)) {
            Ok(true) => {
                match crossterm::event::read() {
                    Ok(Event::Key(key)) => {
                        if let Some(event) = map_key_event(key) {
                            tracing::debug!("tty_input: key event {:?}", event);
                            if tx.send(event).is_err() {
                                tracing::warn!("tty_input: channel closed");
                                return;
                            }
                        }
                    }
                    Ok(Event::Resize(cols, rows)) => {
                        tracing::debug!("tty_input: resize {}x{}", cols, rows);
                        let event = InputEvent::WindowResize {
                            width: cols as u32,
                            height: rows as u32,
                            scale_factor: 1.0,
                            emacs_frame_id: 0,
                        };
                        if tx.send(event).is_err() {
                            tracing::warn!("tty_input: channel closed");
                            return;
                        }
                    }
                    Ok(Event::Mouse(mouse)) => {
                        tracing::debug!("tty_input: mouse event {:?}", mouse);
                        // Mouse events are not yet wired to the evaluator;
                        // they can be added later when TTY mouse support is
                        // needed. For now we just log them.
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("tty_input: crossterm read error: {}", e);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
            Ok(false) => continue,
            Err(e) => {
                tracing::warn!("tty_input: crossterm poll error: {}", e);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A standalone TTY input reader that forwards terminal key and resize
/// events to `RenderComms` using crossterm.
///
/// Used by the `-nw` path when rendering goes through `TtyRif` on the
/// evaluator thread.
pub struct TtyInputReader {
    handle: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl TtyInputReader {
    /// Spawn a background thread that reads terminal input and sends events
    /// through `comms.send_input()`.
    pub fn spawn(comms: RenderComms) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let input_stop = Arc::clone(&stop);
        let handle = thread::Builder::new()
            .name("tty-input-reader".to_string())
            .spawn(move || {
                let pause = Arc::new(AtomicBool::new(false));
                let (tx, rx) = crossbeam_channel::unbounded();
                let reader_stop = Arc::clone(&input_stop);
                let reader_pause = Arc::clone(&pause);
                let reader_handle = thread::Builder::new()
                    .name("tty-input-raw".to_string())
                    .spawn(move || read_tty_events(tx, reader_stop, reader_pause))
                    .ok();

                // Forward events to the RenderComms channel and listen for
                // shutdown commands.
                loop {
                    crossbeam_channel::select! {
                        recv(comms.cmd_rx) -> msg => {
                            match msg {
                                Ok(RenderCommand::Lifecycle(LifecycleCommand::Shutdown)) | Err(_) => break,
                                Ok(_) => {}
                            }
                        }
                        recv(rx) -> msg => {
                            match msg {
                                Ok(event) => comms.send_input(event),
                                Err(_) => break,
                            }
                        }
                        default(Duration::from_millis(50)) => {}
                    }
                }

                input_stop.store(true, Ordering::Relaxed);
                if let Some(h) = reader_handle {
                    let _ = h.join();
                }
            })
            .expect("Failed to spawn tty-input-reader thread");

        Self {
            handle: Some(handle),
            stop,
        }
    }

    /// Signal the input reader to stop and wait for it to finish.
    pub fn join(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn key_parts(code: KeyCode, modifiers: KeyModifiers) -> (u32, u32) {
        match map_key_event(key_event(code, modifiers)).expect("key event") {
            InputEvent::Key {
                keysym, modifiers, ..
            } => (keysym, modifiers),
            _ => panic!("expected key event"),
        }
    }

    #[test]
    fn tty_control_digit_aliases_are_raw_control_bytes() {
        let cases = [
            ('2', 0x00),
            ('3', 0x1b),
            ('4', 0x1c),
            ('5', 0x1d),
            ('6', 0x1e),
            ('7', 0x1f),
            ('8', 0x7f),
            ('/', 0x1f),
        ];

        for (input, expected) in cases {
            let (keysym, modifiers) = key_parts(KeyCode::Char(input), KeyModifiers::CONTROL);
            assert_eq!(keysym, expected, "input C-{input}");
            assert_eq!(modifiers & NEOMACS_CTRL_MASK, 0, "input C-{input}");
        }
    }

    #[test]
    fn tty_meta_control_alias_preserves_meta_only() {
        let (keysym, modifiers) = key_parts(
            KeyCode::Char('4'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );

        assert_eq!(keysym, 0x1c);
        assert_eq!(modifiers & NEOMACS_CTRL_MASK, 0);
        assert_ne!(modifiers & NEOMACS_META_MASK, 0);
    }

    #[test]
    fn tty_backspace_is_raw_del_byte() {
        let (keysym, modifiers) = key_parts(KeyCode::Backspace, KeyModifiers::ALT);

        assert_eq!(keysym, 0x7f);
        assert_ne!(modifiers & NEOMACS_META_MASK, 0);
    }

    #[cfg(unix)]
    #[test]
    fn raw_tty_decoder_keeps_more_than_crossterm_buffer() {
        let mut decoder = RawTtyDecoder::default();
        let input = vec![b'a'; 1500];
        let mut events = Vec::new();

        decoder.push_bytes(&input, &mut events);

        assert_eq!(events.len(), input.len());
        for event in events {
            match event {
                InputEvent::Key {
                    keysym, modifiers, ..
                } => {
                    assert_eq!(keysym, b'a' as u32);
                    assert_eq!(modifiers, 0);
                }
                _ => panic!("expected key event"),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn raw_tty_decoder_maps_meta_ret_and_meta_tab_to_control_chars() {
        // ESC RET / ESC TAB are GNU's M-RET / M-TAB: meta + the control CHAR
        // (13 / 9), not `M-<return>` / `M-<tab>` function keys (there is no
        // [M-return]/[M-tab] function-key-map entry to translate those back).
        let mut decoder = RawTtyDecoder::default();
        let mut events = Vec::new();
        decoder.push_bytes(&[0x1b, b'\r', 0x1b, b'\t'], &mut events);
        assert_eq!(events.len(), 2);
        match events[0] {
            InputEvent::Key {
                keysym, modifiers, ..
            } => {
                assert_eq!(keysym, u32::from(b'\r'));
                assert_eq!(modifiers, NEOMACS_META_MASK);
            }
            _ => panic!("expected key event"),
        }
        match events[1] {
            InputEvent::Key {
                keysym, modifiers, ..
            } => {
                assert_eq!(keysym, u32::from(b'\t'));
                assert_eq!(modifiers, NEOMACS_META_MASK);
            }
            _ => panic!("expected key event"),
        }
        // The UNPREFIXED RET/TAB bytes still map to the function-key symbols.
        let mut plain = Vec::new();
        RawTtyDecoder::default().push_bytes(&[b'\r'], &mut plain);
        match plain[0] {
            InputEvent::Key { keysym, .. } => assert_eq!(keysym, XK_RETURN),
            _ => panic!("expected key event"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn raw_tty_decoder_maps_meta_del_and_control_meta_backslash() {
        let mut decoder = RawTtyDecoder::default();
        let mut events = Vec::new();

        decoder.push_bytes(&[0x1b, 0x7f, 0x1b, 0x1c], &mut events);

        assert_eq!(events.len(), 2);
        match events[0] {
            InputEvent::Key {
                keysym, modifiers, ..
            } => {
                assert_eq!(keysym, 0x7f);
                assert_eq!(modifiers, NEOMACS_META_MASK);
            }
            _ => panic!("expected key event"),
        }
        match events[1] {
            InputEvent::Key {
                keysym, modifiers, ..
            } => {
                assert_eq!(keysym, 0x1c);
                assert_eq!(modifiers, NEOMACS_META_MASK);
            }
            _ => panic!("expected key event"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn raw_tty_decoder_maps_test_escape_sequences() {
        let mut decoder = RawTtyDecoder::default();
        let mut events = Vec::new();

        decoder.push_bytes(b"\x1b[A\x1b[B\x1b[C\x1b[D\x1b[21~", &mut events);

        let keysyms: Vec<u32> = events
            .into_iter()
            .map(|event| match event {
                InputEvent::Key { keysym, .. } => keysym,
                _ => panic!("expected key event"),
            })
            .collect();
        assert_eq!(keysyms, [XK_UP, XK_DOWN, XK_RIGHT, XK_LEFT, XK_F1 + 9]);
    }

    #[cfg(unix)]
    #[test]
    fn raw_tty_decoder_keeps_control_j_as_control_character() {
        let mut decoder = RawTtyDecoder::default();
        let mut events = Vec::new();

        decoder.push_bytes(&[b'\n', b'\r'], &mut events);

        let keysyms: Vec<u32> = events
            .into_iter()
            .map(|event| match event {
                InputEvent::Key { keysym, .. } => keysym,
                _ => panic!("expected key event"),
            })
            .collect();
        assert_eq!(keysyms, [b'\n' as u32, XK_RETURN]);
    }

    #[cfg(unix)]
    #[test]
    fn raw_tty_decoder_decodes_utf8_characters() {
        let mut decoder = RawTtyDecoder::default();
        let mut events = Vec::new();

        decoder.push_bytes("ä中".as_bytes(), &mut events);

        let keysyms: Vec<u32> = events
            .into_iter()
            .map(|event| match event {
                InputEvent::Key { keysym, .. } => keysym,
                _ => panic!("expected key event"),
            })
            .collect();
        assert_eq!(keysyms, ['ä' as u32, '中' as u32]);
    }
}
