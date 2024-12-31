use core::cell::UnsafeCell;

pub struct SingleThreadLocal<T>(UnsafeCell<T>);

unsafe impl<T> Sync for SingleThreadLocal<T> {}

impl<T> SingleThreadLocal<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn with_borrow_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        unsafe { f(&mut *self.0.get()) }
    }
}
