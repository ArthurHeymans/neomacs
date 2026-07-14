use crate::thread_comm::ClipboardSelection;
use arboard::Clipboard;
use winit::window::Window;

#[cfg(target_os = "linux")]
use arboard::{ClearExtLinux, GetExtLinux, LinuxClipboardKind, SetExtLinux};
#[cfg(target_os = "linux")]
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

trait ClipboardBackend {
    fn set_text(&mut self, selection: ClipboardSelection, text: Option<&str>)
    -> Result<(), String>;

    fn text(&mut self, selection: ClipboardSelection) -> Result<Option<String>, String>;
}

pub(crate) struct ClipboardService {
    backend: Box<dyn ClipboardBackend>,
}

impl ClipboardService {
    pub(crate) fn for_window(window: &Window) -> Result<Self, String> {
        std::cfg_select! {
            target_os = "linux" => {
                let display = window
                    .display_handle()
                    .map_err(|err| format!("failed to access the window display: {err}"))?;
                if let RawDisplayHandle::Wayland(display) = display.as_raw() {
                    tracing::info!("Clipboard service using the native Wayland data-device backend");
                    // SAFETY: Winit owns this wl_display and ClipboardService is dropped
                    // before the Winit window and event loop tear the display connection down.
                    let clipboard = unsafe {
                        smithay_clipboard::Clipboard::new(display.display.as_ptr())
                    };
                    return Ok(Self {
                        backend: Box::new(WaylandClipboard { clipboard }),
                    });
                }
            }
            _ => {
                let _ = window;
            }
        }

        tracing::info!("Clipboard service using the arboard platform backend");
        Ok(Self {
            backend: Box::new(ArboardClipboard::new()?),
        })
    }

    #[cfg(test)]
    fn with_backend(backend: impl ClipboardBackend + 'static) -> Self {
        Self {
            backend: Box::new(backend),
        }
    }

    pub(crate) fn set_text(
        &mut self,
        selection: ClipboardSelection,
        text: Option<&str>,
    ) -> Result<(), String> {
        self.backend.set_text(selection, text)
    }

    pub(crate) fn text(&mut self, selection: ClipboardSelection) -> Result<Option<String>, String> {
        self.backend.text(selection)
    }
}

struct ArboardClipboard {
    clipboard: Clipboard,
}

impl ArboardClipboard {
    fn new() -> Result<Self, String> {
        Clipboard::new()
            .map(|clipboard| Self { clipboard })
            .map_err(|err| format!("failed to initialize the system clipboard: {err}"))
    }

    fn text_result(result: Result<String, arboard::Error>) -> Result<Option<String>, String> {
        match result {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }
}

impl ClipboardBackend for ArboardClipboard {
    fn set_text(
        &mut self,
        selection: ClipboardSelection,
        text: Option<&str>,
    ) -> Result<(), String> {
        std::cfg_select! {
            target_os = "linux" => {
                let selection = match selection {
                    ClipboardSelection::Clipboard => LinuxClipboardKind::Clipboard,
                    ClipboardSelection::Primary => LinuxClipboardKind::Primary,
                };
                match text {
                    Some(text) => self
                        .clipboard
                        .set()
                        .clipboard(selection)
                        .text(text.to_owned()),
                    None => self.clipboard.clear_with().clipboard(selection),
                }
                .map_err(|err| err.to_string())
            }
            _ => {
                match selection {
                    ClipboardSelection::Clipboard => match text {
                        Some(text) => self.clipboard.set_text(text.to_owned()),
                        None => self.clipboard.clear(),
                    }
                    .map_err(|err| err.to_string()),
                    ClipboardSelection::Primary => {
                        Err("PRIMARY selection is not supported on this platform".to_owned())
                    }
                }
            }
        }
    }

    fn text(&mut self, selection: ClipboardSelection) -> Result<Option<String>, String> {
        std::cfg_select! {
            target_os = "linux" => {
                let selection = match selection {
                    ClipboardSelection::Clipboard => LinuxClipboardKind::Clipboard,
                    ClipboardSelection::Primary => LinuxClipboardKind::Primary,
                };
                Self::text_result(self.clipboard.get().clipboard(selection).text())
            }
            _ => {
                match selection {
                    ClipboardSelection::Clipboard => Self::text_result(self.clipboard.get_text()),
                    ClipboardSelection::Primary => {
                        Err("PRIMARY selection is not supported on this platform".to_owned())
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
struct WaylandClipboard {
    clipboard: smithay_clipboard::Clipboard,
}

#[cfg(target_os = "linux")]
impl ClipboardBackend for WaylandClipboard {
    fn set_text(
        &mut self,
        selection: ClipboardSelection,
        text: Option<&str>,
    ) -> Result<(), String> {
        // smithay-clipboard does not expose a disown operation. Publishing an
        // empty string preserves GNU's observable "no text" result.
        let text = text.unwrap_or_default().to_owned();
        match selection {
            ClipboardSelection::Clipboard => self.clipboard.store(text),
            ClipboardSelection::Primary => self.clipboard.store_primary(text),
        }
        Ok(())
    }

    fn text(&mut self, selection: ClipboardSelection) -> Result<Option<String>, String> {
        let result = match selection {
            ClipboardSelection::Clipboard => self.clipboard.load(),
            ClipboardSelection::Primary => self.clipboard.load_primary(),
        };
        match result {
            Ok(text) => Ok(Some(text)),
            Err(err)
                if err.kind() == std::io::ErrorKind::NotFound
                    || err.to_string() == "selection is empty" =>
            {
                Ok(None)
            }
            Err(err) => Err(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryClipboard {
        selections: HashMap<ClipboardSelection, String>,
    }

    impl ClipboardBackend for MemoryClipboard {
        fn set_text(
            &mut self,
            selection: ClipboardSelection,
            text: Option<&str>,
        ) -> Result<(), String> {
            if let Some(text) = text {
                self.selections.insert(selection, text.to_owned());
            } else {
                self.selections.remove(&selection);
            }
            Ok(())
        }

        fn text(&mut self, selection: ClipboardSelection) -> Result<Option<String>, String> {
            Ok(self.selections.get(&selection).cloned())
        }
    }

    #[test]
    fn service_keeps_clipboard_and_primary_distinct_and_can_clear_them() {
        let mut service = ClipboardService::with_backend(MemoryClipboard::default());

        service
            .set_text(ClipboardSelection::Clipboard, Some("copied"))
            .unwrap();
        service
            .set_text(ClipboardSelection::Primary, Some("selected"))
            .unwrap();
        assert_eq!(
            service.text(ClipboardSelection::Clipboard).unwrap(),
            Some("copied".to_owned())
        );
        assert_eq!(
            service.text(ClipboardSelection::Primary).unwrap(),
            Some("selected".to_owned())
        );

        service
            .set_text(ClipboardSelection::Clipboard, None)
            .unwrap();
        assert_eq!(service.text(ClipboardSelection::Clipboard).unwrap(), None);
        assert_eq!(
            service.text(ClipboardSelection::Primary).unwrap(),
            Some("selected".to_owned())
        );
    }
}
