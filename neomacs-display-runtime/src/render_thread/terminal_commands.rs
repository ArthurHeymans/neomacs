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
                size,
                mode,
                shell,
            } => match crate::terminal::TerminalView::new(id, size, mode, shell.as_deref()) {
                Ok(view) => {
                    if let Ok(mut shared) = self.shared_terminals.lock() {
                        shared.insert(id, view.term.clone());
                    }
                    self.terminal_manager.terminals.insert(id, view);
                    tracing::info!(
                        "Terminal {} created ({}x{}, {:?})",
                        id,
                        size.cols,
                        size.rows,
                        mode
                    );
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
            TerminalCommand::TerminalResize { id, size } => {
                if let Some(view) = self.terminal_manager.get_mut(id) {
                    view.resize(size);
                }
            }
            TerminalCommand::TerminalDestroy { id } => {
                if let Ok(mut shared) = self.shared_terminals.lock() {
                    shared.remove(&id);
                }
                match self.terminal_manager.destroy(id) {
                    Ok(true) => tracing::info!("Terminal {} destroyed", id),
                    Ok(false) => tracing::debug!("Terminal {} was already absent", id),
                    Err(error) => {
                        tracing::error!("Terminal {} teardown failed: {}", id, error);
                    }
                }
            }
            TerminalCommand::TerminalSetFloat { id, placement } => {
                if let Some(view) = self.terminal_manager.get_mut(id) {
                    view.float_x = placement.x();
                    view.float_y = placement.y();
                    view.float_opacity = placement.opacity();
                }
            }
        }
    }
}
