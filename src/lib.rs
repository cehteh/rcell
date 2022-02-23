#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use parking_lot::lock_api::RawMutex as RawMutexTrait;
use parking_lot::RawMutex;
use std::cell::UnsafeCell;
use std::mem;
use std::sync::{Arc, Weak};

/// A RCell holding either an `Arc<T>`, a `Weak<T>` or being `Empty`.
#[derive(Debug)]
pub struct RCell<T>(UnsafeCell<ArcState<T>>);

#[derive(Debug)]
enum ArcState<T> {
    Arc(Arc<T>),
    Weak(Weak<T>),
    Empty,
}

impl<T> RCell<T> {
    /// Creates a new strong (Arc<T>) RCell from the supplied value.
    pub fn new(value: T) -> Self {
        RCell(UnsafeCell::new(ArcState::Arc(Arc::new(value))))
    }

    /// Returns 'true' when this RCell contains a strong `Arc<T>`.
    pub fn retained(&self) -> bool {
        let lock = self.sharded_lock();
        matches!(self.get_mut(&lock), ArcState::Arc(_))
    }

    /// Returns the number of strong references holding an object alive. The returned strong
    /// count is informal only, the result may be appoximate and has race conditions when
    /// other threads modify the refcount concurrently.
    pub fn refcount(&self) -> usize {
        let lock = self.sharded_lock();
        self.get_mut(&lock).refcount()
    }

    /// Tries to upgrade this RCell from Weak<T> to Arc<T>. This means that as long the RCell
    /// is not dropped the associated data won't be either. When successful it returns
    /// Some<Arc<T>> containing the value, otherwise None is returned on failure.
    pub fn retain(&self) -> Option<Arc<T>> {
        let lock = self.sharded_lock();
        let cell = self.get_mut(&lock);
        match cell {
            ArcState::Arc(arc) => Some(arc.clone()),
            ArcState::Weak(weak) => {
                if let Some(arc) = weak.upgrade() {
                    let _ = mem::replace(cell, ArcState::Arc(arc.clone()));
                    Some(arc)
                } else {
                    None
                }
            }
            ArcState::Empty => None,
        }
    }

    /// Downgrades the RCell, any associated value may become dropped when no other references
    /// exist. When no strong reference left remaining this cell becomes Empty.
    pub fn release(&self) {
        let lock = self.sharded_lock();
        let cell = self.get_mut(&lock);

        if let Some(weak) = match cell {
            ArcState::Arc(arc) => Some(Arc::downgrade(arc)),
            ArcState::Weak(weak) => Some(weak.clone()),
            ArcState::Empty => None,
        } {
            if weak.strong_count() > 0 {
                let _ = mem::replace(cell, ArcState::Weak(weak));
            } else {
                let _ = mem::replace(cell, ArcState::Empty);
            }
        }
    }

    /// Removes the reference to the value. The rationale for this function is to release
    /// *any* resource associated with a RCell (potentially member of a struct that lives
    /// longer) in case one knows that it will never be upgraded again.
    pub fn remove(&self) {
        let lock = self.sharded_lock();
        let _ = mem::replace(self.get_mut(&lock), ArcState::Empty);
    }

    /// Tries to get an `Arc<T>` from the RCell. This may fail if the RCell was Weak and all
    /// other `Arc's` became dropped.
    pub fn request(&self) -> Option<Arc<T>> {
        let lock = self.sharded_lock();
        match self.get_mut(&lock) {
            ArcState::Arc(arc) => Some(arc.clone()),
            ArcState::Weak(weak) => weak.upgrade(),
            ArcState::Empty => None,
        }
    }

    // idea borrowed from crossbeam SeqLock
    fn sharded_mutex(&self) -> &'static RawMutex {
        const LEN: usize = 97;
        static LOCKS: [RawMutex; LEN] = [RawMutex::INIT; LEN];
        &LOCKS[self as *const Self as usize % LEN]
    }

    // Acquire a global sharded lock with unlock on drop semantics
    fn sharded_lock(&self) -> RawMutexGuard<T> {
        self.sharded_mutex().lock();
        RawMutexGuard(self)
    }

    // SAFETY: _is_locked_ is intentionally unused, its only there to denote that the lock must been held.
    #[allow(clippy::mut_from_ref)]
    fn get_mut(&self, _is_locked_: &RawMutexGuard<T>) -> &mut ArcState<T> {
        unsafe { &mut *self.0.get() }
    }
}

struct RawMutexGuard<'a, T>(&'a RCell<T>);
impl<T> Drop for RawMutexGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: the guard gurantees that the we have the lock
            self.0.sharded_mutex().unlock();
        }
    }
}

/// Helper Trait for replacing the content of a RCell with something new.
pub trait Replace<T> {
    /// Replaces the contained value in self with T.
    fn replace(&self, new: T);
}

impl<T> Replace<Arc<T>> for RCell<T> {
    /// Replaces the RCell with the supplied `Arc<T>`. The old entry becomes dropped.
    fn replace(&self, arc: Arc<T>) {
        let lock = self.sharded_lock();
        let _ = mem::replace(self.get_mut(&lock), ArcState::Arc(arc));
    }
}

impl<T> Replace<Weak<T>> for RCell<T> {
    /// Replaces the RCell with the supplied `Weak<T>`. The old entry becomes dropped.
    fn replace(&self, weak: Weak<T>) {
        let lock = self.sharded_lock();
        let _ = mem::replace(self.get_mut(&lock), ArcState::Weak(weak));
    }
}

impl<T> From<Arc<T>> for RCell<T> {
    /// Creates a new strong RCell with the supplied `Arc<T>`.
    fn from(arc: Arc<T>) -> Self {
        RCell(UnsafeCell::new(ArcState::Arc(arc)))
    }
}

impl<T> From<Weak<T>> for RCell<T> {
    /// Creates a new weak RCell with the supplied `Weak<T>`.
    fn from(weak: Weak<T>) -> Self {
        RCell(UnsafeCell::new(ArcState::Weak(weak)))
    }
}

impl<T> Default for RCell<T> {
    /// Creates an RCell that doesn't hold any reference.
    fn default() -> Self {
        RCell(UnsafeCell::new(ArcState::Empty))
    }
}

impl<T> Clone for RCell<T> {
    fn clone(&self) -> Self {
        let lock = self.sharded_lock();
        RCell(UnsafeCell::new(self.get_mut(&lock).clone()))
    }
}

impl<T> ArcState<T> {
    fn refcount(&self) -> usize {
        match self {
            ArcState::Arc(arc) => Arc::strong_count(arc),
            ArcState::Weak(weak) => weak.strong_count(),
            ArcState::Empty => 0,
        }
    }
}

impl<T> Clone for ArcState<T> {
    fn clone(&self) -> Self {
        use ArcState::*;
        match self {
            Arc(arc) => Arc(arc.clone()),
            Weak(weak) => Weak(weak.clone()),
            Empty => Empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RCell;
    use std::sync::Arc;

    #[test]
    fn smoke() {
        let rcell = RCell::new("foobar");
        assert!(rcell.retained());
    }

    #[test]
    fn new() {
        let rcell = RCell::new("foobar");
        assert!(rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.release();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn default() {
        let rcell = RCell::<i32>::default();
        assert!(!rcell.retained());
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn from_arc() {
        let arc = Arc::new("foobar");
        let rcell = RCell::from(arc);
        assert!(rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.release();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn from_weak_release() {
        let arc = Arc::new("foobar");
        let weak = Arc::downgrade(&arc);
        let rcell = RCell::from(weak);
        assert!(!rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.release();
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn from_weak_drop_original() {
        let arc = Arc::new("foobar");
        let weak = Arc::downgrade(&arc);
        let rcell = RCell::from(weak);
        assert!(!rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        drop(arc);
        assert_eq!(rcell.request(), None);
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }
}
