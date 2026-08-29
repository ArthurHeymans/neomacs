//! Native Lisp declarations for the xwidget runtime.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("make-xwidget", create, 4, Some(7)),
    SubrSpec::many("xwidgetp", is_xwidget, 1, Some(1)),
    SubrSpec::many("xwidget-view-p", is_view, 1, Some(1)),
    SubrSpec::many("xwidget-live-p", is_live, 1, Some(1)),
    SubrSpec::many("xwidget-info", info, 1, Some(1)),
    SubrSpec::many("xwidget-view-info", view_info, 1, Some(1)),
    SubrSpec::many("xwidget-view-model", view_model, 1, Some(1)),
    SubrSpec::many("xwidget-view-window", view_window, 1, Some(1)),
    SubrSpec::many("xwidget-view-lookup", lookup_view, 2, Some(2)),
    SubrSpec::many("delete-xwidget-view", delete_view, 1, Some(1)),
    SubrSpec::many("xwidget-plist", plist, 1, Some(1)),
    SubrSpec::many("set-xwidget-plist", set_plist, 2, Some(2)),
    SubrSpec::many("xwidget-buffer", buffer, 1, Some(1)),
    SubrSpec::many("set-xwidget-buffer", set_buffer, 2, Some(2)),
    SubrSpec::many("xwidget-query-on-exit-flag", query_on_exit, 1, Some(1)),
    SubrSpec::many(
        "set-xwidget-query-on-exit-flag",
        set_query_on_exit,
        2,
        Some(2),
    ),
    SubrSpec::many("get-buffer-xwidgets", buffer_xwidgets, 1, Some(1)),
    SubrSpec::many("kill-xwidget", kill, 1, Some(1)),
    SubrSpec::many("xwidget-resize", resize, 3, Some(3)),
    SubrSpec::many("xwidget-size-request", size_request, 1, Some(1)),
    SubrSpec::many("xwidget-webkit-uri", webkit_uri, 1, Some(1)),
    SubrSpec::many("xwidget-webkit-title", webkit_title, 1, Some(1)),
    SubrSpec::many("xwidget-webkit-goto-uri", navigate_webkit, 2, Some(2)),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
