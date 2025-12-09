//! μC/OS-III RTOS implementation in Rust
//!
//! A real-time operating system kernel providing:
//! - Priority-based preemptive scheduling
//! - Synchronization primitives (semaphores, mutexes)
//! - Time management with tick-based delays
//! - Context switching for ARM Cortex-M

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

// ============ Critical Section ============

#[cfg(target_arch = "arm")]
mod cs_impl {
    use cortex_m::interrupt;
    use cortex_m::register::primask;
    use critical_section::{set_impl, Impl, RawRestoreState};

    struct SingleCoreCriticalSection;
    set_impl!(SingleCoreCriticalSection);

    unsafe impl Impl for SingleCoreCriticalSection {
        unsafe fn acquire() -> RawRestoreState {
            let was_active = primask::read().is_active();
            interrupt::disable();
            was_active
        }

        unsafe fn release(was_active: RawRestoreState) {
            if was_active {
                unsafe { interrupt::enable() }
            }
        }
    }
}

// ============ Modules ============

pub mod log;
mod lang_items;

pub mod core;
pub mod sync;
pub mod port;

// ============ Re-exports ============

pub use core::config;
pub use core::config::*;
pub use core::critical;
pub use core::error;
pub use core::error::OsError;
pub use core::kernel;
pub use core::kernel::{os_init, os_start};
pub use core::prio;
pub use core::types;
pub use core::types::*;
pub use core::task;
pub use core::task::os_task_create;
pub use core::sched;
pub use core::time;

#[cfg(feature = "sem")]
pub use sync::sem;
#[cfg(feature = "mutex")]
pub use sync::mutex;

#[cfg(feature = "pac")]
pub use stm32_metapac as pac;
