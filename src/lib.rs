#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
//! Example:
//! ```
//! use rcell::*;
//!
//! // Our Type
//! #[derive(Debug, PartialEq)]
//! struct MyType<T>(T);
//!
//! let my_rcell = RCell::new(MyType(100u8));
//! assert_eq!(*my_rcell.request().unwrap(), MyType(100));
//! ```

use std::mem;
#[cfg(feature = "sync")]
pub use std::sync::{Arc as Strong, Weak};

#[cfg(not(feature = "sync"))]
pub use std::rc::{Rc as Strong, Weak};

/// A RCell holding either an `Strong<T>`, a `Weak<T>` or being `Empty`.
#[derive(Debug)]
pub enum RCell<T> {
    /// Strong reference
    Strong(Strong<T>),
    /// Weak reference
    Weak(Weak<T>),
    /// Empty cell
    Empty,
}

impl<T> RCell<T> {
    /// Creates a new strong (Strong<T>) RCell from the supplied value.
    pub fn new(value: T) -> Self {
        RCell::Strong(Strong::new(value))
    }

    /// Returns 'true' when this RCell contains a `Strong<T>`.
    pub fn retained(&self) -> bool {
        matches!(*self, RCell::Strong(_))
    }

    /// Returns the number of strong references holding an object alive. The returned strong
    /// count is informal only, the result may be approximate and has race conditions when
    /// other threads modify the reference count concurrently.
    pub fn refcount(&self) -> usize {
        match self {
            RCell::Strong(arc) => Strong::strong_count(arc),
            RCell::Weak(weak) => weak.strong_count(),
            RCell::Empty => 0,
        }
    }

    /// Tries to upgrade this RCell from Weak<T> to Strong<T>. This means that as long the RCell
    /// is not dropped the associated data won't be either. When successful it returns
    /// Some<Strong<T>> containing the value, otherwise None is returned on failure.
    pub fn retain(&mut self) -> Option<Strong<T>> {
        match self {
            RCell::Strong(strong) => Some(strong.clone()),
            RCell::Weak(weak) => {
                if let Some(strong) = weak.upgrade() {
                    let _ = mem::replace(self, RCell::Strong(strong.clone()));
                    Some(strong)
                } else {
                    None
                }
            }
            RCell::Empty => None,
        }
    }

    /// Downgrades the RCell, any associated value may become dropped when no other references
    /// exist. When no strong reference left remaining this cell becomes Empty.
    pub fn release(&mut self) {
        if let Some(weak) = match self {
            RCell::Strong(strong) => Some(Strong::downgrade(strong)),
            RCell::Weak(weak) => Some(weak.clone()),
            RCell::Empty => None,
        } {
            if weak.strong_count() > 0 {
                let _ = mem::replace(self, RCell::Weak(weak));
            } else {
                let _ = mem::replace(self, RCell::Empty);
            }
        }
    }

    /// Removes the reference to the value. The rationale for this function is to release
    /// *any* resource associated with a RCell (potentially member of a struct that lives
    /// longer) in case one knows that it will never be upgraded again.
    pub fn remove(&mut self) {
        let _ = mem::replace(self, RCell::Empty);
    }

    /// Tries to get an `Strong<T>` from the RCell. This may fail if the RCell was Weak and all
    /// other strong references became dropped.
    pub fn request(&self) -> Option<Strong<T>> {
        match self {
            RCell::Strong(arc) => Some(arc.clone()),
            RCell::Weak(weak) => weak.upgrade(),
            RCell::Empty => None,
        }
    }
}

/// Helper Trait for replacing the content of a RCell with something new.
pub trait Replace<T> {
    /// Replaces the contained value in self with T.
    fn replace(&mut self, new: T);
}

impl<T> Replace<Strong<T>> for RCell<T> {
    /// Replaces the RCell with the supplied `Strong<T>`. The old entry becomes dropped.
    fn replace(&mut self, strong: Strong<T>) {
        let _ = mem::replace(self, RCell::Strong(strong));
    }
}

impl<T> Replace<Weak<T>> for RCell<T> {
    /// Replaces the RCell with the supplied `Weak<T>`. The old entry becomes dropped.
    fn replace(&mut self, weak: Weak<T>) {
        let _ = mem::replace(self, RCell::Weak(weak));
    }
}

impl<T> From<Strong<T>> for RCell<T> {
    /// Creates a new strong RCell with the supplied `Strong<T>`.
    fn from(strong: Strong<T>) -> Self {
        RCell::Strong(strong)
    }
}

impl<T> From<Weak<T>> for RCell<T> {
    /// Creates a new weak RCell with the supplied `Weak<T>`.
    fn from(weak: Weak<T>) -> Self {
        RCell::Weak(weak)
    }
}

impl<T> Default for RCell<T> {
    /// Creates an RCell that doesn't hold any reference.
    fn default() -> Self {
        RCell::Empty
    }
}

// impl<T> Clone for RCell<T>
// {
//     fn clone(&self) -> Self {
//         RCell(self.clone())
//     }
// }

// impl<T> Clone for RCell<T> {
//     fn clone(&self) -> Self {
//         match self {
//             Strong(arc) => Strong(arc.clone()),
//             Weak(weak) => Weak(weak.clone()),
//             Empty => Empty,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use crate::{RCell, Strong, Replace};

    #[test]
    fn smoke() {
        let rcell = RCell::new("foobar");
        assert!(rcell.retained());
    }

    #[test]
    fn new() {
        let mut rcell = RCell::new("foobar");
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
    fn from_strong() {
        let strong = Strong::new("foobar");
        let mut rcell = RCell::from(strong);
        assert!(rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.release();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn from_weak_release() {
        let strong = Strong::new("foobar");
        let weak = Strong::downgrade(&strong);
        let mut rcell = RCell::from(weak);
        assert!(!rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.release();
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn from_weak_drop_original() {
        let strong = Strong::new("foobar");
        let weak = Strong::downgrade(&strong);
        let mut rcell = RCell::from(weak);
        assert!(!rcell.retained());
        assert_eq!(*rcell.request().unwrap(), "foobar");
        drop(strong);
        assert_eq!(rcell.request(), None);
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn replace_strong() {
        let mut rcell = RCell::default();
        assert!(!rcell.retained());
        rcell.replace(Strong::new("foobar"));
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }

    #[test]
    fn replace_weak() {
        let strong = Strong::new("foobar");
        let mut rcell = RCell::default();
        assert!(!rcell.retained());
        rcell.replace(Strong::downgrade(&strong));
        assert_eq!(*rcell.request().unwrap(), "foobar");
        rcell.remove();
        assert_eq!(rcell.request(), None);
    }
}
