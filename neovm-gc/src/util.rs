/// Round `addr` up to the next multiple of `align`.
/// `align` must be a non-zero power of two.
#[inline]
pub(crate) fn align_up(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two(), "alignment must be a power of two");
    let mask = align.wrapping_sub(1);
    (addr.wrapping_add(mask)) & !mask
}
