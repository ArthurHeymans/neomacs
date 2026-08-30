use super::WebKitViewCommand;
use crate::thread_comm::AssetCommand;

/// Every WebKit command the Lisp side produces reaches this backend through
/// one conversion, with the Lisp-side u32 sizes widened to points.
#[test]
fn every_lisp_side_webkit_command_converts() {
    let cases = [
        (
            AssetCommand::WebKitCreate {
                id: 1,
                width: 640,
                height: 480,
            },
            WebKitViewCommand::Create {
                id: 1,
                width: 640.0,
                height: 480.0,
            },
        ),
        (
            AssetCommand::WebKitLoadUri {
                id: 1,
                url: "https://example.invalid/".into(),
            },
            WebKitViewCommand::LoadUri {
                id: 1,
                url: "https://example.invalid/".into(),
            },
        ),
        (
            AssetCommand::WebKitResize {
                id: 1,
                width: 10,
                height: 20,
            },
            WebKitViewCommand::Resize {
                id: 1,
                width: 10.0,
                height: 20.0,
            },
        ),
        (
            AssetCommand::WebKitExecuteScript {
                id: 1,
                script: "document.title".into(),
            },
            WebKitViewCommand::ExecuteScript {
                id: 1,
                script: "document.title".into(),
            },
        ),
        (
            AssetCommand::WebKitDestroy { id: 1 },
            WebKitViewCommand::Destroy { id: 1 },
        ),
    ];
    for (asset, expected) in cases {
        assert_eq!(WebKitViewCommand::from_asset(&asset), Some(expected));
    }
}

/// Commands that are not this backend's business are declined, not mangled.
#[test]
fn a_non_webkit_command_is_not_converted() {
    let other = AssetCommand::WebKitReload { id: 1 }; // WPE-only, no producer here
    assert_eq!(WebKitViewCommand::from_asset(&other), None);
}
