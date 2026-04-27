use std::fmt;

const TAG_BITS: u32 = 3;
const TAG_MASK: usize = (1 << TAG_BITS) - 1;
const FIXNUM_TAG: usize = 0b000;
const HEAP_TAG: usize = 0b001;
const CHAR_TAG: usize = 0b010;
const SPECIAL_TAG: usize = 0b110;

const NIL_BITS: usize = SPECIAL_TAG;
const TRUE_BITS: usize = (1 << TAG_BITS) | SPECIAL_TAG;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("neovm-executor currently requires a 64-bit target for LispValue ABI bits");

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LispValue(usize);

impl LispValue {
    pub const NIL: Self = Self(NIL_BITS);
    pub const TRUE: Self = Self(TRUE_BITS);

    pub const fn from_bits(bits: usize) -> Self {
        Self(bits)
    }

    pub const fn to_bits(self) -> usize {
        self.0
    }

    pub const fn from_abi_i64(bits: i64) -> Self {
        Self(bits as usize)
    }

    pub const fn to_abi_i64(self) -> i64 {
        self.0 as i64
    }

    pub fn from_fixnum(value: isize) -> Option<Self> {
        if !(Self::FIXNUM_MIN..=Self::FIXNUM_MAX).contains(&value) {
            return None;
        }
        Some(Self(((value as usize) << TAG_BITS) | FIXNUM_TAG))
    }

    pub fn expect_fixnum(value: isize) -> Self {
        Self::from_fixnum(value).expect("fixnum value is outside LispValue immediate range")
    }

    pub fn as_fixnum(self) -> Option<isize> {
        self.is_fixnum().then_some((self.0 as isize) >> TAG_BITS)
    }

    pub fn from_char(value: char) -> Self {
        Self(((value as u32 as usize) << TAG_BITS) | CHAR_TAG)
    }

    pub fn as_char(self) -> Option<char> {
        if self.tag() != CHAR_TAG {
            return None;
        }
        char::from_u32((self.0 >> TAG_BITS) as u32)
    }

    pub const fn is_nil(self) -> bool {
        self.0 == NIL_BITS
    }

    pub const fn is_true(self) -> bool {
        self.0 == TRUE_BITS
    }

    pub const fn is_fixnum(self) -> bool {
        self.tag() == FIXNUM_TAG
    }

    pub const fn is_heap(self) -> bool {
        self.tag() == HEAP_TAG
    }

    pub(crate) fn from_heap_addr(addr: usize) -> Self {
        debug_assert_eq!(addr & TAG_MASK, 0, "heap object pointers must be aligned");
        Self(addr | HEAP_TAG)
    }

    pub(crate) fn heap_addr(self) -> Option<usize> {
        self.is_heap().then_some(self.0 & !TAG_MASK)
    }

    const FIXNUM_MIN: isize = isize::MIN >> TAG_BITS;
    const FIXNUM_MAX: isize = isize::MAX >> TAG_BITS;

    const fn tag(self) -> usize {
        self.0 & TAG_MASK
    }
}

impl fmt::Debug for LispValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            return f.write_str("nil");
        }
        if self.is_true() {
            return f.write_str("t");
        }
        if let Some(value) = self.as_fixnum() {
            return write!(f, "{value}");
        }
        if let Some(value) = self.as_char() {
            return write!(f, "?{value}");
        }
        if self.is_heap() {
            return write!(f, "#<heap 0x{:x}>", self.heap_addr().unwrap_or(0));
        }
        write!(f, "#<lisp-value 0x{:x}>", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::LispValue;

    #[test]
    fn fixnums_roundtrip_through_tagged_word() {
        for value in [-17, -1, 0, 42, 1_000_000] {
            let tagged = LispValue::from_fixnum(value).expect("fixnum");
            assert!(tagged.is_fixnum());
            assert_eq!(tagged.as_fixnum(), Some(value));
        }
    }

    #[test]
    fn specials_are_not_fixnums() {
        assert!(LispValue::NIL.is_nil());
        assert!(LispValue::TRUE.is_true());
        assert!(!LispValue::NIL.is_fixnum());
        assert!(!LispValue::TRUE.is_fixnum());
    }

    #[test]
    fn chars_roundtrip_through_tagged_word() {
        let tagged = LispValue::from_char('a');
        assert_eq!(tagged.as_char(), Some('a'));
        assert!(!tagged.is_fixnum());
        assert!(!tagged.is_heap());
    }

    #[test]
    fn abi_bits_roundtrip_losslessly() {
        let value = LispValue::expect_fixnum(-12);
        assert_eq!(LispValue::from_abi_i64(value.to_abi_i64()), value);
    }
}
