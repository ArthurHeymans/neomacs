#[inline]
pub(in crate::buffer) fn emacs_char_to_byte_in_slice(
    bytes: &[u8],
    char_pos: usize,
    multibyte: bool,
) -> usize {
    if multibyte {
        crate::emacs_core::emacs_char::char_to_byte_pos(bytes, char_pos)
    } else {
        char_pos.min(bytes.len())
    }
}

#[inline]
pub(in crate::buffer) fn emacs_byte_to_char_in_slice(
    bytes: &[u8],
    byte_pos: usize,
    multibyte: bool,
    context: &str,
) -> usize {
    if !multibyte {
        return byte_pos.min(bytes.len());
    }
    assert!(
        is_emacs_char_boundary(bytes, byte_pos, multibyte),
        "{context}: byte_pos ({byte_pos}) is not an Emacs character boundary",
    );
    crate::emacs_core::emacs_char::byte_to_char_pos(bytes, byte_pos)
}

#[inline]
pub(in crate::buffer) fn is_emacs_char_boundary(
    bytes: &[u8],
    byte_pos: usize,
    multibyte: bool,
) -> bool {
    if byte_pos > bytes.len() {
        return false;
    }
    if !multibyte || byte_pos == 0 || byte_pos == bytes.len() {
        return true;
    }
    (bytes[byte_pos] & 0xC0) != 0x80
}
