//! Neomacs-specific Lisp primitives with no GNU Emacs `src/*.c` mirror.

pub(crate) mod effects;
pub(crate) mod terminal;

#[cfg(test)]
mod terminal_test;
