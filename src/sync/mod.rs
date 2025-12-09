//! Synchronization primitives
//!
//! Contains semaphores and mutexes.

#[cfg(feature = "sem")]
pub mod sem;

#[cfg(feature = "mutex")]
pub mod mutex;
