//! Neomacs compositor-owned terminal support.

mod subrs;

pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    subrs::register_subrs(ctx);
}
