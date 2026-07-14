use super::RenderApp;
use crate::thread_comm::{ClipboardCommand, LifecycleCommand, RenderCommand, WindowCommand};

impl RenderApp {
    /// Process pending commands from Emacs.
    pub(super) fn process_commands(&mut self) -> bool {
        let mut should_exit = false;

        while let Ok(cmd) = self.comms.cmd_rx.try_recv() {
            match cmd {
                RenderCommand::Lifecycle(c) => {
                    if let LifecycleCommand::Shutdown = c {
                        tracing::info!("Render thread received shutdown command");
                        self.lifecycle_flags.shutdown_requested = true;
                        should_exit = true;
                        continue;
                    }
                }
                RenderCommand::Window(c) => {
                    if matches!(c, WindowCommand::ScrollBlit { .. }) {
                        tracing::debug!("ScrollBlit ignored (full-frame rendering mode)");
                        continue;
                    }
                    self.handle_window(c);
                }
                RenderCommand::Asset(c) => self.handle_asset(c),
                #[cfg(feature = "neo-term")]
                RenderCommand::Terminal(c) => self.handle_terminal(c),
                RenderCommand::Ui(c) => self.handle_ui(c),
                RenderCommand::Config(c) => self.handle_config(c),
                RenderCommand::Clipboard(c) => self.handle_clipboard(c),
            }
        }

        should_exit
    }

    fn handle_clipboard(&mut self, command: ClipboardCommand) {
        match command {
            ClipboardCommand::SetText {
                selection,
                text,
                reply,
            } => {
                let result = match self.clipboard.as_mut() {
                    Ok(clipboard) => clipboard.set_text(selection, text.as_deref()),
                    Err(err) => Err(err.clone()),
                };
                if let Err(err) = &result {
                    tracing::warn!(?selection, "clipboard set failed: {err}");
                }
                if reply.send(result).is_err() {
                    tracing::debug!("clipboard set reply receiver was dropped");
                }
            }
            ClipboardCommand::GetText { selection, reply } => {
                let result = match self.clipboard.as_mut() {
                    Ok(clipboard) => clipboard.text(selection),
                    Err(err) => Err(err.clone()),
                };
                if let Err(err) = &result {
                    tracing::warn!(?selection, "clipboard read failed: {err}");
                }
                if reply.send(result).is_err() {
                    tracing::debug!("clipboard get reply receiver was dropped");
                }
            }
        }
    }
}
