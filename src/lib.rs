#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use parking_lot::Mutex;
use std::mem;
use std::sync::{Arc, Weak};

/// A RCell holding either an `Arc<T>` or a `Weak<T>`.
#[derive(Debug)]
pub struct RCell<T>(Mutex<ArcState<T>>);

#[derive(Debug)]
enum ArcState<T> {
    Arc(Arc<T>),
    Weak(Weak<T>),
}

impl<T> RCell<T> {
    /// Create a nee RCell holding a empty reference always returning None on upgrading.
    pub fn new() -> RCell<T> {
        RCell(Mutex::new(ArcState::Weak(Weak::new())))
    }

    /// Creates a new strong RCell from the supplied value.
    pub fn new_from(value: T) -> Self {
        RCell(Mutex::new(ArcState::Arc(Arc::new(value))))
    }

    /// Returns 'true' when this RCell contains a strong `Arc<T>`.
    pub fn retained(&self) -> bool {
        matches!(*self.0.lock(), ArcState::Arc(_))
    }

    /// Tries to upgrade this RCell from Weak<T> to Arc<T>. This means that as long the RCell
    /// is not dropped the associated data won't be either.
    pub fn retain(&self) -> Option<Arc<T>> {
        let mut lock = self.0.lock();
        match &*lock {
            ArcState::Arc(arc) => Some(arc.clone()),
            ArcState::Weak(weak) => {
                if let Some(arc) = weak.upgrade() {
                    let _ = mem::replace(&mut *lock, ArcState::Arc(arc.clone()));
                    Some(arc)
                } else {
                    None
                }
            }
        }
    }

    /// Downgrades the RCell, any associated value may become dropped when no other references exist.
    pub fn release(&self) {
        let mut lock = self.0.lock();
        let new = if let ArcState::Arc(arc) = &*lock {
            Some(ArcState::Weak(Arc::downgrade(arc)))
        } else {
            None
        };

        if let Some(new) = new {
            let _ = mem::replace(&mut *lock, new);
        }
    }

    /// Removes the reference to the value, initializes it as with `RCell::new()`.
    pub fn remove(&self) {
        let _ = mem::replace(&mut *self.0.lock(), ArcState::Weak(Weak::new()));
    }

    /// Tries to get an `Arc<T>` from the RCell. This may fail if the RCell was Weak and all
    /// other `Arc's` became dropped.
    pub fn request(&self) -> Option<Arc<T>> {
        if let ArcState::Arc(arc) = &*self.0.lock() {
            Some(arc.clone())
        } else {
            None
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
        let _ = mem::replace(&mut *self.0.lock(), ArcState::Arc(arc));
    }
}

impl<T> Replace<Weak<T>> for RCell<T> {
    /// Replaces the RCell with the supplied `Weak<T>`. The old entry becomes dropped.
    fn replace(&self, weak: Weak<T>) {
        let _ = mem::replace(&mut *self.0.lock(), ArcState::Weak(weak));
    }
}

impl<T> From<Arc<T>> for RCell<T> {
    /// Creates a new strong RCell with the supplied `Arc<T>`.
    fn from(arc: Arc<T>) -> Self {
        RCell(Mutex::new(ArcState::Arc(arc)))
    }
}

impl<T> From<Weak<T>> for RCell<T> {
    /// Creates a new weak RCell with the supplied `Weak<T>`.
    fn from(weak: Weak<T>) -> Self {
        RCell(Mutex::new(ArcState::Weak(weak)))
    }
}

impl<T> Default for RCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for RCell<T> {
    fn clone(&self) -> Self {
        RCell(Mutex::new(self.0.lock().clone()))
    }
}

impl<T> Clone for ArcState<T> {
    fn clone(&self) -> Self {
        use ArcState::*;
        match self {
            Arc(arc) => Arc(arc.clone()),
            Weak(weak) => Weak(weak.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
