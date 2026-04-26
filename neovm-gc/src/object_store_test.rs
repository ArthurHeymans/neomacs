use crate::descriptor::{Relocator, Trace, Tracer, fixed_type_desc};
use crate::object::{ObjectRecord, SpaceKind};
use crate::object_store::{ObjectPublishLocal, ObjectStore};

#[derive(Debug)]
struct Leaf;

unsafe impl Trace for Leaf {
    fn trace(&self, _tracer: &mut dyn Tracer) {}

    fn relocate(&self, _relocator: &mut dyn Relocator) {}
}

#[test]
fn mark_only_read_uses_header_locators_without_index_snapshot() {
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let store = ObjectStore::default();
    let mut publish_local = ObjectPublishLocal::default();

    let first = ObjectRecord::allocate(desc, SpaceKind::Old, Leaf).expect("allocate first object");
    let first_key = first.object_key();
    let first_locator = store.publish_shared(first, &mut publish_local);
    assert_eq!(store.object_count(), 1);

    let read = store.read_marking();
    assert!(read.index.is_empty());
    let raw = read.raw();
    assert_eq!(raw.locator_of_key(first_key), Some(first_locator));
    assert_eq!(raw.get(first_locator).object_key(), first_key);

    let second =
        ObjectRecord::allocate(desc, SpaceKind::Old, Leaf).expect("allocate second object");
    let second_key = second.object_key();
    store.publish_shared(second, &mut publish_local);
    assert_eq!(store.object_count(), 2);

    assert_eq!(raw.locator_of_key(second_key), None);
}

#[test]
fn prepared_publish_refreshes_stale_reservation_after_store_generation_change() {
    let desc = Box::leak(Box::new(fixed_type_desc::<Leaf>()));
    let mut store = ObjectStore::default();
    let mut publish_local = ObjectPublishLocal::default();

    for shard in 0..super::OBJECT_STORE_SHARDS {
        publish_local.reservations[shard] = store.reserve_publish_chunk(shard);
    }
    let flat = store.take_flat();
    store.restore_from_flat(flat);

    let object = ObjectRecord::allocate(desc, SpaceKind::Old, Leaf).expect("allocate object");
    let object_key = object.object_key();
    let locator = store.publish_shared_prepared(object, &mut publish_local);

    let read = store.read();
    let raw = read.raw();
    assert_eq!(store.object_count(), 1);
    assert_eq!(raw.len(), 1);
    assert_eq!(raw.locator_of_key(object_key), Some(locator));
    assert_eq!(raw.get(locator).object_key(), object_key);
}
