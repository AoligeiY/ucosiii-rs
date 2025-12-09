//! Critical section protected cell
//!
//! Zero-overhead wrapper for data that must be accessed within critical sections.

use core::cell::UnsafeCell;
use crate::critical::CriticalSection;

/// A cell that can only be accessed within a critical section.
pub struct CsCell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for CsCell<T> {}

impl<T> CsCell<T> {
    /// Create a new CsCell
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// Get a mutable reference to the inner value
    #[inline(always)]
    pub fn get(&self, _cs: &CriticalSection) -> &mut T {
        unsafe { &mut *self.0.get() }
    }

    /// Get a mutable reference without requiring a CriticalSection guard
    #[inline(always)]
    pub unsafe fn get_unchecked(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }

    /// Get a raw pointer
    #[inline(always)]
    pub const fn as_ptr(&self) -> *mut T {
        self.0.get()
    }
}
