//! The one place the WebKit command taxonomy is spelled for this backend.
//!
//! `AssetCommand` carries the WebKit commands across the render channel for
//! both backends. Everything this backend does with them -- queue them before
//! a host exists, replay them, apply them to a live view -- is expressed over
//! [`WebKitViewCommand`], so adding a command is one variant here, one
//! conversion arm here, and one exhaustive `match` arm in
//! `WkWebViewHost::apply_live`. The last two are compiler-checked; the
//! conversion is not, because `AssetCommand` also carries WPE-only commands
//! this backend declines -- a nested `AssetCommand::WebKit(_)` would close
//! that gap and is tracked in issue 300.

use crate::thread_comm::AssetCommand;

/// A WebKit command addressed to this backend.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WebKitViewCommand {
    Create { id: u32, width: f64, height: f64 },
    LoadUri { id: u32, url: String },
    Resize { id: u32, width: f64, height: f64 },
    ExecuteScript { id: u32, script: String },
    Destroy { id: u32 },
}

impl WebKitViewCommand {
    pub(crate) fn id(&self) -> u32 {
        match *self {
            Self::Create { id, .. }
            | Self::LoadUri { id, .. }
            | Self::Resize { id, .. }
            | Self::ExecuteScript { id, .. }
            | Self::Destroy { id } => id,
        }
    }

    /// The WebKit command inside an asset command, or the command back.
    ///
    /// Consumes rather than borrows: scripts and `data:` URLs can be large,
    /// and copying them on the render thread for every command is not a cost
    /// worth paying to keep the original around. A command this backend does
    /// not handle is returned untouched for the other arms.
    pub(crate) fn from_asset(command: AssetCommand) -> Result<Self, AssetCommand> {
        Ok(match command {
            AssetCommand::WebKitCreate { id, width, height } => Self::Create {
                id,
                width: f64::from(width),
                height: f64::from(height),
            },
            AssetCommand::WebKitLoadUri { id, url } => Self::LoadUri { id, url },
            AssetCommand::WebKitResize { id, width, height } => Self::Resize {
                id,
                width: f64::from(width),
                height: f64::from(height),
            },
            AssetCommand::WebKitExecuteScript { id, script } => Self::ExecuteScript { id, script },
            AssetCommand::WebKitDestroy { id } => Self::Destroy { id },
            other => return Err(other),
        })
    }
}

#[cfg(test)]
#[path = "command_test.rs"]
mod tests;
