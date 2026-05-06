use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::descriptor::{EphemeronVisitor, GcErased, WeakProcessor};
use crate::object::ObjectHeader;
use crate::root::Gc;

/// Weak reference to a managed object.
#[derive(Debug)]
pub struct Weak<T: ?Sized> {
    target: Option<Gc<T>>,
}

impl<T: ?Sized> Weak<T> {
    /// Create a weak handle from an existing managed object.
    pub const fn new(target: Gc<T>) -> Self {
        Self {
            target: Some(target),
        }
    }

    /// Create an empty weak reference.
    pub const fn empty() -> Self {
        Self { target: None }
    }

    /// Return the underlying weak target when still known.
    pub fn target(&self) -> Option<Gc<T>> {
        self.target
    }
}

impl<T: ?Sized> Copy for Weak<T> {}

impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> PartialEq for Weak<T> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
    }
}

impl<T: ?Sized> Eq for Weak<T> {}

impl<T: ?Sized> Hash for Weak<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.target.hash(state);
    }
}

/// Interior-mutable weak edge slot.
///
/// `process` is called by the collector during stop-the-world phases
/// (safepoint write lock held), so it cannot race with mutator `set`
/// (which requires the safepoint read lock).  Consequently the
/// read-modify-write in `process` is correct under the current
/// safepoint discipline; this invariant must be preserved by any
/// future architectural change.
pub struct WeakCell<T: ?Sized> {
    value: AtomicPtr<ObjectHeader>,
    _marker: PhantomData<fn() -> T>,
}

impl<T: ?Sized> WeakCell<T> {
    /// Create a weak slot with `value`.
    pub fn new(value: Weak<T>) -> Self {
        Self {
            value: AtomicPtr::new(Self::raw_value(value)),
            _marker: PhantomData,
        }
    }

    /// Read the current weak value.
    pub fn get(&self) -> Weak<T> {
        match self.load_target() {
            Some(target) => Weak::new(target),
            None => Weak::empty(),
        }
    }

    /// Return the current weak target when still known.
    pub fn target(&self) -> Option<Gc<T>> {
        self.load_target()
    }

    /// Overwrite the current weak value.
    pub fn set(&self, value: Weak<T>) {
        self.value.store(Self::raw_value(value), Ordering::Release);
    }

    /// Clear the current weak target.
    pub fn clear(&self) {
        self.set(Weak::empty());
    }

    /// Process this weak slot against the current collector liveness view.
    pub fn process(&self, processor: &mut dyn WeakProcessor) {
        if let Some(target) = self.target() {
            let remapped = processor.remap_or_drop(target.erase());
            if let Some(object) = remapped {
                self.set(Weak::new(unsafe { Gc::from_erased(object) }));
            } else {
                self.clear();
            }
        }
    }

    fn raw_value(value: Weak<T>) -> *mut ObjectHeader {
        match value.target() {
            Some(target) => target.erase().as_raw(),
            None => core::ptr::null_mut(),
        }
    }

    fn load_target(&self) -> Option<Gc<T>> {
        let raw = self.value.load(Ordering::Acquire);
        unsafe { GcErased::from_raw(raw).map(|value| Gc::from_erased(value)) }
    }
}

impl<T: ?Sized> Default for WeakCell<T> {
    fn default() -> Self {
        Self::new(Weak::empty())
    }
}

impl<T: ?Sized> fmt::Debug for WeakCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakCell")
            .field("is_some", &self.target().is_some())
            .finish()
    }
}

/// Token identifying one weak-map instance in the collector.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct WeakMapToken(pub u64);

/// Interior-mutable ephemeron slot.
///
/// Key and value are written through a single [`AtomicPtr`] to a
/// shared heap-allocated pair so that a concurrent reader always
/// observes a consistent (key, value) tuple, never a torn
/// old-key + new-value or new-key + old-value combination.
///
/// `process` is called by the collector during stop-the-world
/// phases (safepoint write lock held), so the read-modify-write
/// cycle cannot race with a concurrent mutator `set`.  This
/// invariant must be preserved by any future architectural change.
pub struct Ephemeron<K: ?Sized, V: ?Sized> {
    slot: AtomicPtr<EphemeronPair>,
    _key_marker: PhantomData<fn() -> K>,
    _value_marker: PhantomData<fn() -> V>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EphemeronPair {
    key: *mut ObjectHeader,
    value: *mut ObjectHeader,
}

impl EphemeronPair {
    fn new(key: *mut ObjectHeader, value: *mut ObjectHeader) -> *mut Self {
        Box::into_raw(Box::new(Self { key, value }))
    }
}

impl<K: ?Sized, V: ?Sized> Ephemeron<K, V> {
    /// Create a new ephemeron entry.
    pub fn new(key: Weak<K>, value: Weak<V>) -> Self {
        let raw_key = match key.target() {
            Some(t) => t.erase().as_raw(),
            None => core::ptr::null_mut(),
        };
        let raw_val = match value.target() {
            Some(t) => t.erase().as_raw(),
            None => core::ptr::null_mut(),
        };
        Self {
            slot: AtomicPtr::new(EphemeronPair::new(raw_key, raw_val)),
            _key_marker: PhantomData,
            _value_marker: PhantomData,
        }
    }

    /// Create an empty ephemeron entry.
    pub fn empty() -> Self {
        Self::new(Weak::empty(), Weak::empty())
    }

    /// Return the current ephemeron key when still known.
    pub fn key(&self) -> Option<Gc<K>> {
        let pair = self.load_pair();
        unsafe { GcErased::from_raw(pair.key).map(|value| Gc::from_erased(value)) }
    }

    /// Return the current ephemeron value when still known.
    pub fn value(&self) -> Option<Gc<V>> {
        let pair = self.load_pair();
        unsafe { GcErased::from_raw(pair.value).map(|value| Gc::from_erased(value)) }
    }

    /// Overwrite the current ephemeron pair with a single atomic store.
    pub fn set(&self, key: Weak<K>, value: Weak<V>) {
        let raw_key = match key.target() {
            Some(t) => t.erase().as_raw(),
            None => core::ptr::null_mut(),
        };
        let raw_val = match value.target() {
            Some(t) => t.erase().as_raw(),
            None => core::ptr::null_mut(),
        };
        let new = EphemeronPair::new(raw_key, raw_val);
        let old = self.slot.swap(new, Ordering::AcqRel);
        // SAFETY: old was allocated by Self::new / Self::set; unique owner.
        if !old.is_null() {
            unsafe { drop(Box::from_raw(old)) };
        }
    }

    /// Clear the current ephemeron pair.
    pub fn clear(&self) {
        self.set(Weak::empty(), Weak::empty());
    }

    /// Visit the current ephemeron pair during fixpoint tracing.
    pub fn visit(&self, visitor: &mut dyn EphemeronVisitor) {
        let pair = self.load_pair();
        let key = unsafe { GcErased::from_raw(pair.key) };
        let value = unsafe { GcErased::from_raw(pair.value) };
        if let (Some(key), Some(value)) = (key, value) {
            visitor.visit_ephemeron(key, value);
        }
    }

    /// Process the current ephemeron pair against the collector liveness view.
    pub fn process(&self, processor: &mut dyn WeakProcessor) {
        let pair = self.load_pair();
        let key = unsafe { GcErased::from_raw(pair.key) };
        let value = unsafe { GcErased::from_raw(pair.value) };
        let (Some(key), Some(value)) = (key, value) else {
            self.clear();
            return;
        };
        let Some(remapped_key) = processor.remap_or_drop(key) else {
            self.clear();
            return;
        };
        let Some(remapped_value) = processor.remap_or_drop(value) else {
            self.clear();
            return;
        };
        self.set(
            Weak::new(unsafe { Gc::from_erased(remapped_key) }),
            Weak::new(unsafe { Gc::from_erased(remapped_value) }),
        );
    }

    fn load_pair(&self) -> EphemeronPair {
        let ptr = self.slot.load(Ordering::Acquire);
        if ptr.is_null() {
            EphemeronPair {
                key: core::ptr::null_mut(),
                value: core::ptr::null_mut(),
            }
        } else {
            // SAFETY: ptr was allocated by EphemeronPair::new and is
            // published through a Release store.  The AtomicPtr ensures
            // the reader sees a fully initialised pair.
            unsafe { *ptr }
        }
    }
}

impl<K: ?Sized, V: ?Sized> Drop for Ephemeron<K, V> {
    fn drop(&mut self) {
        let ptr = self.slot.load(Ordering::Relaxed);
        if !ptr.is_null() {
            // SAFETY: ptr was allocated by EphemeronPair::new; unique owner at drop time.
            unsafe { drop(Box::from_raw(ptr)) };
        }
    }
}

impl<K: ?Sized, V: ?Sized> Default for Ephemeron<K, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<K: ?Sized, V: ?Sized> fmt::Debug for Ephemeron<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pair = self.load_pair();
        f.debug_struct("Ephemeron")
            .field("has_key", &!pair.key.is_null())
            .field("has_value", &!pair.value.is_null())
            .finish()
    }
}

#[cfg(test)]
#[path = "weak_test.rs"]
mod tests;
