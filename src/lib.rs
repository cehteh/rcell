#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use parking_lot::Mutex;
use std::mem;
use std::sync::{Arc, Weak};

struct RCell<T>(Mutex<ArcState<T>>);

enum ArcState<T> {
    Arc(Arc<T>),
    Weak(Weak<T>),
}

impl<T> RCell<T> {
    pub fn new() -> RCell<T> {
        RCell(Mutex::new(ArcState::Weak(Weak::new())))
    }

    pub fn retained(&self) -> bool {
        matches!(*self.0.lock(), ArcState::Arc(_))
    }

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

    pub fn remove(&self) {
        let _ = mem::replace(&mut *self.0.lock(), ArcState::Weak(Weak::new()));
    }

    pub fn request(&self) -> Option<Arc<T>> {
        if let ArcState::Arc(arc) = &*self.0.lock() {
            Some(arc.clone())
        } else {
            None
        }
    }
}

impl<T> Default for RCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

trait Replace<T> {
    fn replace(&self, new: T);
}

impl<T> Replace<Arc<T>> for RCell<T> {
    fn replace(&self, arc: Arc<T>) {
        let _ = mem::replace(&mut *self.0.lock(), ArcState::Arc(arc));
    }
}

impl<T> Replace<Weak<T>> for RCell<T> {
    fn replace(&self, weak: Weak<T>) {
        let _ = mem::replace(&mut *self.0.lock(), ArcState::Weak(weak));
    }
}

impl<T> From<Arc<T>> for RCell<T> {
    fn from(arc: Arc<T>) -> Self {
        RCell(Mutex::new(ArcState::Arc(arc)))
    }
}

impl<T> From<Weak<T>> for RCell<T> {
    fn from(weak: Weak<T>) -> Self {
        RCell(Mutex::new(ArcState::Weak(weak)))
    }
}

impl<T> From<T> for RCell<T> {
    fn from(value: T) -> Self {
        RCell(Mutex::new(ArcState::Arc(Arc::new(value))))
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
