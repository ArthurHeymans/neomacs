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
                target,
                shell,
            } => match crate::terminal::TerminalView::new(id, size, target, shell.as_deref()) {
                Ok(view) => {
                    if let Err(error) = self.shared_terminals.mark_live(id, view.term.clone()) {
                        tracing::error!("Failed to publish terminal {id}: {error}");
                        return;
                    }
                    self.terminal_manager.terminals.insert(id, view);
                    tracing::info!(
                        "Terminal {} created ({}x{}, {:?})",
                        id,
                        size.cols,
                        size.rows,
                        target
                    );
                }
                Err(e) => {
                    let error = e.to_string();
                    self.shared_terminals.mark_failed(id, error.clone());
                    self.comms
                        .send_input(crate::thread_comm::InputEvent::TerminalCreateFailed {
                            id,
                            error: error.clone(),
                        });
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
            TerminalCommand::TerminalDestroy { id } => match self.terminal_manager.destroy(id) {
                Ok(true) => {
                    self.shared_terminals.complete_destroy(id);
                    tracing::info!("Terminal {} destroyed", id);
                }
                Ok(false) => {
                    self.shared_terminals.complete_destroy(id);
                    tracing::debug!("Terminal {} was already absent", id);
                }
                Err(error) => {
                    self.shared_terminals
                        .mark_destroy_failed(id, format!("teardown failed: {error}"));
                    tracing::error!("Terminal {} teardown failed: {}", id, error);
                }
            },
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
