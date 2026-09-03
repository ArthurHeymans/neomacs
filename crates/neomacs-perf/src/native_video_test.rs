use super::{InvalidPresentationTarget, NativeVideoPresentationTarget};
use crate::Frontend;

#[test]
fn presentation_target_preserves_non_zero_gui_dimensions() {
    let target = NativeVideoPresentationTarget::from_frontend(Frontend::Gui {
        width: 1920,
        height: 1080,
    })
    .expect("valid GUI presentation target");

    assert_eq!(target.width(), 1920);
    assert_eq!(target.height(), 1080);
}

#[test]
fn presentation_target_rejects_an_unrepresentable_size() {
    assert_eq!(
        NativeVideoPresentationTarget::from_frontend(Frontend::Gui {
            width: 0,
            height: 1080,
        }),
        Err(InvalidPresentationTarget::ZeroWidth)
    );
    assert_eq!(
        NativeVideoPresentationTarget::from_frontend(Frontend::Gui {
            width: 1920,
            height: 0,
        }),
        Err(InvalidPresentationTarget::ZeroHeight)
    );
    assert_eq!(
        NativeVideoPresentationTarget::from_frontend(Frontend::Batch),
        Err(InvalidPresentationTarget::NotGui)
    );
}
