#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use sharded_mutex::{ShardedMutex, ShardedMutexGuard};
use std::mem;
use std::sync::{Arc, Weak};

/// A RCell holding either an `Arc<T>`, a `Weak<T>` or being `Empty`.
#[derive(Debug)]
pub struct RCell<T>(ShardedMutex<ArcState<T>>);

#[derive(Debug)]
enum ArcState<T> {
    Arc(Arc<T>),
    Weak(Weak<T>),
    Empty,
}

impl<T> RCell<T> {
    /// Creates a new strong (Arc<T>) RCell from the supplied value.
    pub fn new(value: T) -> Self {
        RCell(ShardedMutex::new(ArcState::Arc(Arc::new(value))))
    }

    /// Returns 'true' when this RCell contains a strong `Arc<T>`.
    pub fn retained(&self) -> bool {
        matches!(*self.lock(), ArcState::Arc(_))
    }

    /// Returns the number of strong references holding an object alive. The returned strong
    /// count is informal only, the result may be appoximate and has race conditions when
    /// other threads modify the refcount concurrently.
    pub fn refcount(&self) -> usize {
        self.lock().refcount()
    }

    /// Tries to upgrade this RCell from Weak<T> to Arc<T>. This means that as long the RCell
    /// is not dropped the associated data won't be either. When successful it returns
    /// Some<Arc<T>> containing the value, otherwise None is returned on failure.
    pub fn retain(&self) -> Option<Arc<T>> {
        let mut cell = self.lock();
        match cell.as_ref() {
            ArcState::Arc(arc) => Some(arc.clone()),
            ArcState::Weak(weak) => {
                if let Some(arc) = weak.upgrade() {
                    let _ = mem::replace(cell.as_mut(), ArcState::Arc(arc.clone()));
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
        let mut cell = self.lock();

        if let Some(weak) = match cell.as_ref() {
            ArcState::Arc(arc) => Some(Arc::downgrade(arc)),
            ArcState::Weak(weak) => Some(weak.clone()),
            ArcState::Empty => None,
        } {
            if weak.strong_count() > 0 {
                let _ = mem::replace(cell.as_mut(), ArcState::Weak(weak));
            } else {
                let _ = mem::replace(cell.as_mut(), ArcState::Empty);
            }
        }
    }

    /// Removes the reference to the value. The rationale for this function is to release
    /// *any* resource associated with a RCell (potentially member of a struct that lives
    /// longer) in case one knows that it will never be upgraded again.
    pub fn remove(&self) {
        let _ = mem::replace(self.lock().as_mut(), ArcState::Empty);
    }

    /// Tries to get an `Arc<T>` from the RCell. This may fail if the RCell was Weak and all
    /// other `Arc's` became dropped.
    pub fn request(&self) -> Option<Arc<T>> {
        match self.lock().as_ref() {
            ArcState::Arc(arc) => Some(arc.clone()),
            ArcState::Weak(weak) => weak.upgrade(),
            ArcState::Empty => None,
        }
    }

    // Acquire a global sharded lock with unlock on drop semantics
    fn lock(&self) -> ShardedMutexGuard<ArcState<T>> {
        self.0.lock()
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
        let _ = mem::replace(self.lock().as_mut(), ArcState::Arc(arc));
    }
}

impl<T> Replace<Weak<T>> for RCell<T> {
    /// Replaces the RCell with the supplied `Weak<T>`. The old entry becomes dropped.
    fn replace(&self, weak: Weak<T>) {
        let _ = mem::replace(self.lock().as_mut(), ArcState::Weak(weak));
    }
}

impl<T> From<Arc<T>> for RCell<T> {
    /// Creates a new strong RCell with the supplied `Arc<T>`.
    fn from(arc: Arc<T>) -> Self {
        RCell(ShardedMutex::new(ArcState::Arc(arc)))
    }
}

impl<T> From<Weak<T>> for RCell<T> {
    /// Creates a new weak RCell with the supplied `Weak<T>`.
    fn from(weak: Weak<T>) -> Self {
        RCell(ShardedMutex::new(ArcState::Weak(weak)))
    }
}

impl<T> Default for RCell<T> {
    /// Creates an RCell that doesn't hold any reference.
    fn default() -> Self {
        RCell(ShardedMutex::new(ArcState::Empty))
    }
}

impl<T> Clone for RCell<T> {
    fn clone(&self) -> Self {
        RCell(ShardedMutex::new(self.lock().clone()))
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
    use crate::{RCell, Replace};
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

    #[test]
    fn replace_arc() {
        let rcell = RCell::default();
        assert!(!rcell.retained());
        rcell.replace(Arc::new("foobar"));
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn replace_weak() {
        let arc = Arc::new("foobar");
        let rcell = RCell::default();
        assert!(!rcell.retained());
        rcell.replace(Arc::downgrade(&arc));
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }

}
