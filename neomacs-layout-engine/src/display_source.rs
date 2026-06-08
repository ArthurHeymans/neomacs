#![allow(dead_code)]

use crate::display_item::{DisplayItem, DisplaySourcePosition};
use std::marker::PhantomData;

#[derive(Debug)]
pub(crate) struct DisplaySourceContext<'a> {
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> DisplaySourceContext<'a> {
    pub(crate) const fn empty() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl Default for DisplaySourceContext<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

pub(crate) trait DisplayItemSource {
    fn next_item(&mut self, context: &mut DisplaySourceContext<'_>) -> Option<DisplayItem>;
    fn source_position(&self) -> DisplaySourcePosition;
}
