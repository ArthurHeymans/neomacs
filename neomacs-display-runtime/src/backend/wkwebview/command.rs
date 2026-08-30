//! The one place the WebKit command taxonomy is spelled for this backend.
//!
//! `AssetCommand` carries the WebKit commands across the render channel for
//! both backends. Everything this backend does with them -- queue them before
//! a host exists, replay them, apply them to a live view -- is expressed over
//! [`WebKitViewCommand`], so adding a command is one variant here, one
//! conversion arm here, and one exhaustive `match` arm in
//! `WkWebViewHost::apply_live`. The compiler refuses anything less.

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

    /// The WebKit command inside an asset command, if it is one.
    ///
    /// Borrows rather than consumes so the caller can still hand the same
    /// `AssetCommand` to the WPE arms; the payloads are an id and at most one
    /// short string, so the clone is not worth avoiding.
    pub(crate) fn from_asset(command: &AssetCommand) -> Option<Self> {
        Some(match *command {
            AssetCommand::WebKitCreate { id, width, height } => Self::Create {
                id,
                width: f64::from(width),
                height: f64::from(height),
            },
            AssetCommand::WebKitLoadUri { id, ref url } => Self::LoadUri {
                id,
                url: url.clone(),
            },
            AssetCommand::WebKitResize { id, width, height } => Self::Resize {
                id,
                width: f64::from(width),
                height: f64::from(height),
            },
            AssetCommand::WebKitExecuteScript { id, ref script } => Self::ExecuteScript {
                id,
                script: script.clone(),
            },
            AssetCommand::WebKitDestroy { id } => Self::Destroy { id },
            _ => return None,
        })
    }
}

#[cfg(test)]
#[path = "command_test.rs"]
mod tests;
