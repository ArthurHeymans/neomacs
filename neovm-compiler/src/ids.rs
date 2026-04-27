//! Typed compiler entity identifiers.
//!
//! These use `cranelift-entity` rather than hand-rolled ID plumbing. That
//! gives us mature dense maps, boxed slices, and typed indexing for IR storage.

use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(u32);
entity_impl!(SymbolId, "sym");

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstId(u32);
entity_impl!(ConstId, "const");

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(u32);
entity_impl!(LocalId, "local");

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(u32);
entity_impl!(BlockId, "block");

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueId(u32);
entity_impl!(ValueId, "v");

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegId(u32);
entity_impl!(RegId, "r");

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SafepointId(u32);
entity_impl!(SafepointId, "sp");

pub type PrimaryMap<K, V> = cranelift_entity::PrimaryMap<K, V>;
pub type SecondaryMap<K, V> = cranelift_entity::SecondaryMap<K, V>;

#[cfg(test)]
mod tests {
    use cranelift_entity::EntityRef;

    use super::*;

    #[test]
    fn entity_ids_are_indexable() {
        let id = LocalId::new(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn primary_map_allocates_typed_ids() {
        let mut map: PrimaryMap<BlockId, &'static str> = PrimaryMap::new();
        let block = map.push("entry");
        assert_eq!(block.index(), 0);
        assert_eq!(map[block], "entry");
    }
}
