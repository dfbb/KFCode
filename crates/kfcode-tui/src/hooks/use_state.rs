//! Shared reactive state hook with a dirty-flag for change detection.
use std::cell::RefCell;
use std::rc::Rc;

/// Shared, cloneable state cell that tracks whether the value has changed since the last check.
pub struct UseState<T> {
    value: Rc<RefCell<T>>,
    // Dirty flag set on every write; cleared by `clear_changed`.
    changed: Rc<RefCell<bool>>,
}

impl<T: Clone> UseState<T> {
    /// Create a new state cell with the given initial value.
    pub fn new(initial: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(initial)),
            changed: Rc::new(RefCell::new(false)),
        }
    }

    /// Return a clone of the current value.
    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }

    /// Replace the value and mark the state as changed.
    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        *self.changed.borrow_mut() = true;
    }

    /// Mutate the value in place and mark the state as changed.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.value.borrow_mut());
        *self.changed.borrow_mut() = true;
    }

    /// Returns `true` if the value has been set or updated since the last `clear_changed` call.
    pub fn changed(&self) -> bool {
        *self.changed.borrow()
    }

    /// Reset the dirty flag.
    pub fn clear_changed(&self) {
        *self.changed.borrow_mut() = false;
    }
}

impl<T: Clone> Clone for UseState<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            changed: Rc::clone(&self.changed),
        }
    }
}
