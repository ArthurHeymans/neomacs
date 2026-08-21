//! Bridge between display runtime InputEvent and neovm-core keyboard::InputEvent.
//!
//! GUI and non-Unix frontends send keysyms plus modifier bitmasks; Unix TTYs
//! send uninterpreted byte batches. This module preserves that distinction
//! while converting the display transport into the core input transport.

use neomacs_display_runtime::thread_comm::{
    InputEvent as DisplayEvent, MonitorInfo as DisplayMonitorInfo, PointerAction, PointerTarget,
    PositionedPointerInput, ScrollDelta,
};
use neovm_core::emacs_core::builtins::NeomacsMonitorInfo;
use neovm_core::keyboard::{self, InputEvent as KbInputEvent, MouseButton};

pub(crate) fn should_log_display_event(event: &DisplayEvent) -> bool {
    !matches!(
        event,
        DisplayEvent::PositionedPointer(PositionedPointerInput {
            action: PointerAction::Move { .. },
            ..
        })
    )
}

/// Allocation-free result of adapting one display event to the evaluator
/// queue. A positioned pointer input may expand to an observation followed by
/// its action; all other inputs produce at most one event.
#[derive(Debug)]
#[must_use]
pub(crate) struct EvaluatorInputBatch {
    events: [Option<KbInputEvent>; 2],
}

impl EvaluatorInputBatch {
    fn none() -> Self {
        Self {
            events: [None, None],
        }
    }

    fn one(event: KbInputEvent) -> Self {
        Self {
            events: [Some(event), None],
        }
    }

    fn optional(event: Option<KbInputEvent>) -> Self {
        event.map_or_else(Self::none, Self::one)
    }

    fn positioned(observation: Option<KbInputEvent>, action: KbInputEvent) -> Self {
        match observation {
            Some(observation) => Self {
                events: [Some(observation), Some(action)],
            },
            None => Self::one(action),
        }
    }
}

impl IntoIterator for EvaluatorInputBatch {
    type Item = KbInputEvent;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<KbInputEvent>, 2>>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter().flatten()
    }
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
/// The batch is empty for input that should be silently dropped (for example,
/// key releases and modifier-only keys). Presented pointer input expands to an
/// observation immediately followed by its raw evaluator action.
pub(crate) fn convert_display_event(event: &DisplayEvent) -> EvaluatorInputBatch {
    if let DisplayEvent::PositionedPointer(input) = event {
        return convert_positioned_pointer_input(*input);
    }
    EvaluatorInputBatch::optional(convert_single_display_event(event))
}

fn convert_positioned_pointer_input(input: PositionedPointerInput) -> EvaluatorInputBatch {
    let position = input.position;
    let observation = match input.target {
        PointerTarget::Presented { presentation, hit } => Some(KbInputEvent::PresentedRegion {
            presentation,
            hit,
            x: position.x,
            y: position.y,
            target_frame_id: position.target_frame_id,
        }),
        PointerTarget::Unpresented => None,
    };
    let action = match input.action {
        PointerAction::Button {
            button,
            pressed,
            modifiers,
            ..
        } => {
            let button = match button {
                1 => MouseButton::Left,
                2 => MouseButton::Middle,
                3 => MouseButton::Right,
                4 => MouseButton::Button4,
                5 => MouseButton::Button5,
                _ => return EvaluatorInputBatch::none(),
            };
            if pressed {
                KbInputEvent::MousePress {
                    button,
                    x: position.x,
                    y: position.y,
                    modifiers: keyboard::render_modifiers_to_modifiers(modifiers),
                    target_frame_id: position.target_frame_id,
                }
            } else {
                KbInputEvent::MouseRelease {
                    button,
                    x: position.x,
                    y: position.y,
                    target_frame_id: position.target_frame_id,
                }
            }
        }
        PointerAction::Move { modifiers } => KbInputEvent::MouseMove {
            x: position.x,
            y: position.y,
            modifiers: keyboard::render_modifiers_to_modifiers(modifiers),
            target_frame_id: position.target_frame_id,
        },
        PointerAction::Scroll {
            delta, modifiers, ..
        } => {
            let modifiers = keyboard::render_modifiers_to_modifiers(modifiers);
            match delta {
                ScrollDelta::Lines { x, y } => KbInputEvent::MouseScroll {
                    delta_x: x,
                    delta_y: y,
                    x: position.x,
                    y: position.y,
                    modifiers,
                    target_frame_id: position.target_frame_id,
                },
                ScrollDelta::Pixels { x, y } => KbInputEvent::PixelScroll {
                    delta_x: x,
                    delta_y: y,
                    x: position.x,
                    y: position.y,
                    modifiers,
                    target_frame_id: position.target_frame_id,
                },
            }
        }
    };
    EvaluatorInputBatch::positioned(observation, action)
}

fn convert_single_display_event(event: &DisplayEvent) -> Option<KbInputEvent> {
    match event {
        DisplayEvent::PositionedPointer(_) => unreachable!("handled by convert_display_event"),
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
        DisplayEvent::MenuSelection { index } => {
            Some(KbInputEvent::MenuSelection { index: *index })
        }
        DisplayEvent::ImageStateChanged { id, change } => Some(KbInputEvent::ImageStateChanged {
            id: *id,
            change: *change,
        }),
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
