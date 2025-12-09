//! Port layer - CPU-specific implementations
//!
//! This module provides the hardware abstraction layer for context switching
//! and other CPU-specific operations.

#[cfg(target_arch = "arm")]
pub mod cortex_m4;

#[cfg(target_arch = "arm")]
pub use cortex_m4::*;

// Stub implementations for non-ARM targets (for testing)
#[cfg(not(target_arch = "arm"))]
pub mod stub {
    use crate::task::OsTaskFn;
    use crate::types::{OsOpt, OsStkElement};

    pub unsafe fn os_start_high_rdy() {
        panic!("os_start_high_rdy not available on this platform");
    }

    pub fn os_ctx_sw() {
        // No-op for testing
    }

    pub fn os_int_ctx_sw() {
        // No-op for testing
    }

    pub unsafe fn os_task_stk_init(
        _task_fn: OsTaskFn,
        _arg: *mut (),
        stk_base: *mut OsStkElement,
        stk_size: usize,
        _opt: OsOpt,
    ) -> *mut OsStkElement {
        // Return top of stack for testing
        unsafe { stk_base.add(stk_size - 1) }
    }

    pub fn os_cpu_systick_init(_freq: u32) {
        // No-op for testing
    }
}

#[cfg(not(target_arch = "arm"))]
pub use stub::*;
