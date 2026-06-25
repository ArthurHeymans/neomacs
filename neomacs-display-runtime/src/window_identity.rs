//! Platform window identity helpers.

use winit::window::WindowAttributes;

pub(crate) const NEOMACS_APP_ID: &str = "neomacs";

#[cfg(target_os = "linux")]
pub(crate) fn apply_platform_window_identity(attrs: WindowAttributes) -> WindowAttributes {
    winit::platform::wayland::WindowAttributesExtWayland::with_name(
        attrs,
        NEOMACS_APP_ID,
        NEOMACS_APP_ID,
    )
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn apply_platform_window_identity(attrs: WindowAttributes) -> WindowAttributes {
    attrs
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use winit::window::Window;

    #[test]
    fn linux_window_identity_uses_packaged_desktop_id() {
        let attrs = apply_platform_window_identity(Window::default_attributes());

        assert!(format!("{attrs:?}").contains(NEOMACS_APP_ID));
    }
}
