use super::RenderApp;
use crate::thread_comm::{LifecycleCommand, RenderCommand, WindowCommand};

impl RenderApp {
    /// Process pending commands from Emacs.
    pub(super) fn process_commands(&mut self) -> bool {
        let mut should_exit = false;

        while let Ok(cmd) = self.comms.cmd_rx.try_recv() {
            match cmd {
                RenderCommand::Lifecycle(c) => match c {
                    LifecycleCommand::Shutdown => {
                        tracing::info!("Render thread received shutdown command");
                        should_exit = true;
                        continue;
                    }
                    _ => {}
                },
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
            }
        }

        should_exit
    }
}
