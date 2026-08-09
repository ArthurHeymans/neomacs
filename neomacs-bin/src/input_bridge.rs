//! Bridge between display runtime InputEvent and neovm-core keyboard::InputEvent.
//!
//! GUI and non-Unix frontends send keysyms plus modifier bitmasks; Unix TTYs
//! send uninterpreted byte batches. This module preserves that distinction
//! while converting the display transport into the core input transport.

use neomacs_display_runtime::thread_comm::{
    InputEvent as DisplayEvent, MonitorInfo as DisplayMonitorInfo,
};
use neovm_core::emacs_core::builtins::NeomacsMonitorInfo;
use neovm_core::keyboard::{self, InputEvent as KbInputEvent, MouseButton};

pub(crate) fn should_log_display_event(event: &DisplayEvent) -> bool {
    !matches!(event, DisplayEvent::MouseMove { .. })
}

pub(crate) fn convert_monitor_infos(monitors: &[DisplayMonitorInfo]) -> Vec<NeomacsMonitorInfo> {
    monitors
        .iter()
        .map(|monitor| NeomacsMonitorInfo {
            x: monitor.x,
            y: monitor.y,
            width: monitor.width,
            height: monitor.height,
            scale: monitor.scale,
            width_mm: monitor.width_mm,
            height_mm: monitor.height_mm,
            name: monitor.name.clone(),
        })
        .collect()
}

/// Convert a display runtime input event to a neovm-core keyboard input event.
///
/// Returns `None` for events that should be silently dropped (e.g. key
/// releases, modifier-only keys).
pub fn convert_display_event(event: &DisplayEvent) -> Option<KbInputEvent> {
    match event {
        DisplayEvent::RawTtyBytes {
            bytes,
            emacs_frame_id,
        } => Some(KbInputEvent::raw_tty_bytes(bytes.clone(), *emacs_frame_id)),
        DisplayEvent::Key {
            keysym,
            modifiers,
            pressed,
            emacs_frame_id,
        } => {
            tracing::debug!(
                "input_bridge: key keysym=0x{:04x} mods=0x{:x} pressed={}",
                *keysym,
                *modifiers,
                *pressed
            );
            let event = keyboard::render_key_transport_to_input_event(
                *keysym,
                *modifiers,
                *pressed,
                *emacs_frame_id,
            )?;
            tracing::debug!("input_bridge: converted to {:?}", event);
            Some(event)
        }
        DisplayEvent::MouseButton {
            button,
            x,
            y,
            pressed,
            modifiers,
            target_frame_id,
            ..
        } => {
            let mb = match *button {
                1 => MouseButton::Left,
                2 => MouseButton::Middle,
                3 => MouseButton::Right,
                4 => MouseButton::Button4,
                5 => MouseButton::Button5,
                _ => return None,
            };
            if *pressed {
                Some(KbInputEvent::MousePress {
                    button: mb,
                    x: *x,
                    y: *y,
                    modifiers: keyboard::render_modifiers_to_modifiers(*modifiers),
                    target_frame_id: *target_frame_id,
                })
            } else {
                Some(KbInputEvent::MouseRelease {
                    button: mb,
                    x: *x,
                    y: *y,
                    target_frame_id: *target_frame_id,
                })
            }
        }
        DisplayEvent::MouseMove {
            x,
            y,
            modifiers,
            target_frame_id,
            ..
        } => Some(KbInputEvent::MouseMove {
            x: *x,
            y: *y,
            modifiers: keyboard::render_modifiers_to_modifiers(*modifiers),
            target_frame_id: *target_frame_id,
        }),
        DisplayEvent::PresentedRegion {
            presentation,
            hit,
            x,
            y,
            target_frame_id,
        } => Some(KbInputEvent::PresentedRegion {
            presentation: *presentation,
            hit: *hit,
            x: *x,
            y: *y,
            target_frame_id: *target_frame_id,
        }),
        DisplayEvent::MouseScroll {
            delta_x,
            delta_y,
            x,
            y,
            modifiers,
            target_frame_id,
            pixel_precise,
            ..
        } => {
            let modifiers = keyboard::render_modifiers_to_modifiers(*modifiers);
            if *pixel_precise {
                // Trackpad pixel-precise → smooth scroll (Phase 1, T4): applied as a
                // sub-line vscroll by the layout pass, not a discrete wheel event.
                Some(KbInputEvent::PixelScroll {
                    delta_x: *delta_x,
                    delta_y: *delta_y,
                    x: *x,
                    y: *y,
                    modifiers,
                    target_frame_id: *target_frame_id,
                })
            } else {
                // Mouse wheel → discrete wheel event (existing mwheel behavior).
                Some(KbInputEvent::MouseScroll {
                    delta_x: *delta_x,
                    delta_y: *delta_y,
                    x: *x,
                    y: *y,
                    modifiers,
                    target_frame_id: *target_frame_id,
                })
            }
        }
        DisplayEvent::MenuSelection { index } => {
            Some(KbInputEvent::MenuSelection { index: *index })
        }
        DisplayEvent::ImageStateChanged { .. } => Some(KbInputEvent::LayoutInvalidated),
        DisplayEvent::ToolBarClick {
            index,
            emacs_frame_id,
        } => Some(KbInputEvent::ToolBarClick {
            index: *index,
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::PresentedPointer {
            presentation,
            interaction,
            pressed,
            button,
            x,
            y,
            emacs_frame_id,
        } => Some(KbInputEvent::PresentedPointer {
            presentation: *presentation,
            interaction: *interaction,
            pressed: *pressed,
            button: *button,
            x: *x,
            y: *y,
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::PresentationActivated {
            presentation,
            emacs_frame_id,
        } => Some(KbInputEvent::PresentationActivated {
            presentation: *presentation,
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::PresentationDiscarded {
            presentation,
            emacs_frame_id,
        } => Some(KbInputEvent::PresentationDiscarded {
            presentation: *presentation,
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::PresentationRetired { presentation } => {
            Some(KbInputEvent::PresentationRetired {
                presentation: *presentation,
            })
        }
        DisplayEvent::MenuBarClick {
            index,
            key,
            menu_x,
            anchor,
            emacs_frame_id,
        } => Some(KbInputEvent::MenuBarClick {
            index: *index,
            key: key.clone(),
            menu_x: *menu_x,
            menu_y: 0.0,
            anchor_x: anchor.x,
            anchor_y: anchor.y,
            anchor_width: anchor.width,
            anchor_height: anchor.height,
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::WindowResize {
            width,
            height,
            scale_factor,
            emacs_frame_id,
        } => {
            tracing::debug!(
                "input_bridge: resize {}x{} emacs_frame_id=0x{:x}",
                width,
                height,
                emacs_frame_id
            );
            Some(KbInputEvent::Resize {
                width: *width,
                height: *height,
                scale_factor: *scale_factor,
                emacs_frame_id: *emacs_frame_id,
            })
        }
        DisplayEvent::WindowClose { emacs_frame_id } => Some(KbInputEvent::WindowClose {
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::WindowFocus {
            focused,
            emacs_frame_id,
        } => Some(KbInputEvent::Focus {
            focused: *focused,
            emacs_frame_id: *emacs_frame_id,
        }),
        DisplayEvent::MonitorsChanged { monitors } => Some(KbInputEvent::MonitorsChanged {
            monitors: convert_monitor_infos(monitors),
        }),
        // GPU device lost and rebuilt: the evaluator re-resolves media and
        // forces a full redisplay.
        DisplayEvent::DisplayReset => Some(KbInputEvent::DisplayReset),
        // A shader surface failed to build on the render thread past naga
        // pre-validation: hand it to the evaluator to surface to Lisp.
        DisplayEvent::SurfaceCreateFailed { id, error } => {
            Some(KbInputEvent::SurfaceCreateFailed {
                id: *id,
                error: error.clone(),
            })
        }
        #[cfg(feature = "neo-term")]
        DisplayEvent::TerminalCreateFailed { id, error } => {
            Some(KbInputEvent::TerminalCreateFailed {
                id: *id,
                error: error.clone(),
            })
        }
        #[cfg(feature = "neo-term")]
        DisplayEvent::TerminalExited { id } => Some(KbInputEvent::TerminalExited { id: *id }),
        #[cfg(feature = "neo-term")]
        DisplayEvent::TerminalTitleChanged { id, title } => {
            Some(KbInputEvent::TerminalTitleChanged {
                id: *id,
                title: title.clone(),
            })
        }
        // Ignore other events (WebKit title changes, etc.)
        _ => None,
    }
}

#[cfg(test)]
#[path = "input_bridge_test.rs"]
mod tests;
