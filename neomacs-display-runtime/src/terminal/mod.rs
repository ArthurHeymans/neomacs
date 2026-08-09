//! Neo-term: GPU-accelerated terminal emulator for Neomacs.
//!
//! Uses `rio-vt` for VT parsing and terminal state,
//! renders cells directly via the wgpu pipeline.

pub mod colors;
pub mod content;
pub mod view;

pub use content::TerminalContent;
pub use neovm_core::emacs_core::display_host::{
    TerminalDisplayMode as TerminalMode, TerminalFloatPlacement, TerminalGridSize, TerminalId,
};
pub use view::{TerminalManager, TerminalView};

/// Shared terminal state accessible from both Emacs and render threads.
/// Maps terminal ID to its Arc<FairMutex<Term>> for cross-thread text extraction.
pub type SharedTerminals = std::sync::Arc<
    std::sync::Mutex<
        std::collections::HashMap<
            TerminalId,
            std::sync::Arc<
                parking_lot::FairMutex<rio_vt::crosswords::Crosswords<view::NeomacsEventProxy>>,
            >,
        >,
    >,
>;

/// Create one terminal registry to share between the Lisp display host and
/// the render loop. Keeping this state explicit avoids a process-global
/// registry and lets independent editor instances remain isolated.
pub fn new_shared_terminals() -> SharedTerminals {
    std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Snapshot all visible terminal text without exposing rio-vt's grid types
/// across the display-host boundary.
pub fn visible_text(terminals: &SharedTerminals, id: TerminalId) -> Option<String> {
    let terminal = match terminals.lock() {
        Ok(terminals) => terminals.get(&id).cloned(),
        Err(poisoned) => poisoned.into_inner().get(&id).cloned(),
    }?;
    let terminal = terminal.lock();
    let cols = terminal.columns();
    let rows = terminal.screen_lines();
    if cols == 0 || rows == 0 {
        return Some(String::new());
    }
    Some(content::extract_text(&*terminal, 0, 0, rows - 1, cols - 1))
}
