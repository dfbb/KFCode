//! Side-effect hook that runs a closure and stores an optional cleanup function.
use std::cell::RefCell;
use std::rc::Rc;

type CleanupFn = Box<dyn FnMut()>;

/// Runs a side-effect closure and invokes its cleanup when re-run or dropped.
pub struct UseEffect {
    cleanup: Rc<RefCell<Option<CleanupFn>>>,
}

impl UseEffect {
    /// Create a new idle effect hook with no pending cleanup.
    pub fn new() -> Self {
        Self {
            cleanup: Rc::new(RefCell::new(None)),
        }
    }

    /// Run `effect`, calling any previous cleanup first and storing the new one.
    pub fn run<F>(&self, effect: F)
    where
        F: FnOnce() -> Option<Box<dyn FnMut()>>,
    {
        if let Some(mut cleanup) = self.cleanup.borrow_mut().take() {
            cleanup();
        }

        if let Some(new_cleanup) = effect() {
            *self.cleanup.borrow_mut() = Some(new_cleanup);
        }
    }
}

impl Default for UseEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for UseEffect {
    fn drop(&mut self) {
        if let Some(mut cleanup) = self.cleanup.borrow_mut().take() {
            cleanup();
        }
    }
}
