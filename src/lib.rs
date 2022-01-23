use parking_lot::Mutex;
use std::sync::{Arc, Weak};

struct RCell<T>(Mutex<StrongOrWeak<T>>);

enum StrongOrWeak<T> {
    Strong(Arc<T>),
    Weak(Weak<T>),
}

impl<T> RCell<T> {
    pub fn new() -> RCell<T> {
        RCell(Mutex::new(StrongOrWeak::Weak(Weak::new())))
    }

    pub fn retain(&self) -> Option<Arc<T>> {
        todo!()
    }

    pub fn release(&self) {
        todo!()
    }

    pub fn remove(&self) {
        todo!()
    }

    pub fn request() -> Option<Arc<T>> {
        todo!()
    }
}

trait Replace<T> {
    fn replace(&self, new: T);
}

impl<T> Replace<Arc<T>> for RCell<T> {
    fn replace(&self, new: Arc<T>) {
        todo!()
    }
}

impl<T> Replace<Weak<T>> for RCell<T> {
    fn replace(&self, new: Weak<T>) {
        todo!()
    }
}

impl<T> From<T> for RCell<T> {
    fn from(value: T) -> Self {
        RCell(Mutex::new(StrongOrWeak::Strong(Arc::new(value))))
    }
}

impl<T> From<Arc<T>> for RCell<T> {
    fn from(arc: Arc<T>) -> Self {
        RCell(Mutex::new(StrongOrWeak::Strong(arc)))
    }
}

impl<T> From<Weak<T>> for RCell<T> {
    fn from(weak: Weak<T>) -> Self {
        RCell(Mutex::new(StrongOrWeak::Weak(weak)))
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
