use super::CaptureRoute;
use crate::Frontend;

#[test]
fn capture_route_distinguishes_adapter_and_direct_gui_launches() {
    let gui = Frontend::Gui {
        width: 1920,
        height: 1080,
    };
    assert_eq!(
        CaptureRoute::for_frontend(gui, false),
        CaptureRoute::Adapter("GUI")
    );
    assert_eq!(CaptureRoute::for_frontend(gui, true), CaptureRoute::Direct);
}

#[test]
fn batch_is_direct_and_tui_names_its_adapter() {
    assert_eq!(
        CaptureRoute::for_frontend(Frontend::Batch, false),
        CaptureRoute::Direct
    );
    assert_eq!(
        CaptureRoute::for_frontend(
            Frontend::Tui {
                rows: 40,
                columns: 120,
            },
            false,
        ),
        CaptureRoute::Adapter("PTY")
    );
}
