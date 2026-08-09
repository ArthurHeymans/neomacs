//! Terminal render commands.

#[cfg(feature = "neo-term")]
use super::RenderApp;

#[cfg(feature = "neo-term")]
use crate::thread_comm::TerminalCommand;

#[cfg(feature = "neo-term")]
impl RenderApp {
    pub(super) fn handle_terminal(&mut self, cmd: TerminalCommand) {
        match cmd {
            TerminalCommand::TerminalCreate {
                id,
                cols,
                rows,
                mode,
                shell,
            } => match crate::terminal::TerminalView::new(id, cols, rows, mode, shell.as_deref()) {
                Ok(view) => {
                    if let Ok(mut shared) = self.shared_terminals.lock() {
                        shared.insert(id, view.term.clone());
                    }
                    self.terminal_manager.terminals.insert(id, view);
                    tracing::info!("Terminal {} created ({}x{}, {:?})", id, cols, rows, mode);
                }
                Err(e) => {
                    tracing::error!("Failed to create terminal {}: {}", id, e);
                }
            },
            TerminalCommand::TerminalWrite { id, data } => {
                if let Some(view) = self.terminal_manager.get_mut(id)
                    && let Err(e) = view.write(&data)
                {
                    tracing::warn!("Terminal {} write error: {}", id, e);
                }
            }
            TerminalCommand::TerminalResize { id, cols, rows } => {
                if let Some(view) = self.terminal_manager.get_mut(id) {
                    view.resize(cols, rows);
                }
            }
            TerminalCommand::TerminalDestroy { id } => {
                if let Ok(mut shared) = self.shared_terminals.lock() {
                    shared.remove(&id);
                }
                self.terminal_manager.destroy(id);
                tracing::info!("Terminal {} destroyed", id);
            }
            TerminalCommand::TerminalSetFloat { id, x, y, opacity } => {
                if let Some(view) = self.terminal_manager.get_mut(id) {
                    view.float_x = x;
                    view.float_y = y;
                    view.float_opacity = opacity;
                }
            }
        }
    }
}
