#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::buffer) struct BufferTextBytesSnapshot {
    bytes: Vec<u8>,
    multibyte: bool,
}

impl BufferTextBytesSnapshot {
    pub(in crate::buffer) const fn new(bytes: Vec<u8>, multibyte: bool) -> Self {
        Self { bytes, multibyte }
    }

    pub(in crate::buffer) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(in crate::buffer) const fn is_multibyte(&self) -> bool {
        self.multibyte
    }

    pub(in crate::buffer) fn into_parts(self) -> (Vec<u8>, bool) {
        (self.bytes, self.multibyte)
    }
}
