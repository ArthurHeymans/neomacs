//! Cross-platform TTY input reader using crossterm.
//!
//! Replaces raw `libc::poll`/`libc::read` with crossterm's event loop,
//! giving us parsed keycodes, resize events, and mouse events on both
//! Unix and Windows.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::thread_comm::{InputEvent, RenderCommand, RenderComms};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

// Modifier masks — must match neomacs-display-runtime/src/backend/wgpu/events.rs
const NEOMACS_SHIFT_MASK: u32 = 1 << 0;
const NEOMACS_CTRL_MASK: u32 = 1 << 1;
const NEOMACS_META_MASK: u32 = 1 << 2;
const NEOMACS_SUPER_MASK: u32 = 1 << 3;

/// Convert crossterm modifiers to our internal modifier mask.
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

/// Map a crossterm key event to an `InputEvent::Key`.
///
/// Returns `None` for modifier-only keys (Shift, Ctrl, Alt, Super,
/// CapsLock, NumLock, etc.) — those are tracked by crossterm's modifier
/// state on subsequent key events, matching how winit delivers them.
fn map_key_event(event: KeyEvent) -> Option<InputEvent> {
    // Ignore key releases — Emacs only cares about press/repeat.
    if event.kind == KeyEventKind::Release {
        return None;
    }

    let modifiers = map_modifiers(event.modifiers);

    let keysym = match event.code {
        KeyCode::Char(c) => c as u32,
        KeyCode::F(n) if (1..=12).contains(&n) => {
            0xffbe + (n as u32 - 1) // F1=0xffbe … F12=0xffc9
        }
        KeyCode::F(_) => 0, // unsupported function key
        KeyCode::Esc => 0xff1b,
        KeyCode::Enter => 0xff0d,
        KeyCode::Tab => 0xff09,
        KeyCode::Backspace => 0xff08,
        KeyCode::Delete => 0xffff,
        KeyCode::Insert => 0xff63,
        KeyCode::Home => 0xff50,
        KeyCode::End => 0xff57,
        KeyCode::PageUp => 0xff55,
        KeyCode::PageDown => 0xff56,
        KeyCode::Left => 0xff51,
        KeyCode::Up => 0xff52,
        KeyCode::Right => 0xff53,
        KeyCode::Down => 0xff54,
        KeyCode::Null => 0x00,
        KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => 0, // suppress bare modifier/media keys
        KeyCode::BackTab => 0xff09, // same as Tab, but with shift modifier
    };

    if keysym == 0 {
        return None;
    }

    Some(InputEvent::Key {
        keysym,
        modifiers,
        pressed: event.kind == KeyEventKind::Press,
        emacs_frame_id: 0,
    })
}

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
                                Ok(RenderCommand::Shutdown) | Err(_) => break,
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
