//! Critical section handling for μC/OS-III
//!
//! Provides safe critical section primitives for protecting shared resources.

use core::sync::atomic::{AtomicBool, Ordering};

/// Global flag indicating whether it is in the critical section
static IN_CRITICAL: AtomicBool = AtomicBool::new(false);

/// RAII guard for critical sections
/// 
/// When this guard is created, interrupts are disabled.
/// When it is dropped, interrupts are restored to their previous state.
pub struct CriticalSection {
    _private: (),
}

impl CriticalSection {
    /// Enter a critical section by disabling interrupts.
    /// 
    /// Returns a guard that will restore interrupt state when dropped.
    #[inline(always)]
    pub fn enter() -> Self {
        #[cfg(target_arch = "arm")]
        cortex_m::interrupt::disable();
        
        IN_CRITICAL.store(true, Ordering::Release);
        CriticalSection { _private: () }
    }

    /// Check if we're currently in a critical section
    #[inline(always)]
    pub fn is_active() -> bool {
        IN_CRITICAL.load(Ordering::Acquire)
    }
}

impl Drop for CriticalSection {
    #[inline(always)]
    fn drop(&mut self) {
        IN_CRITICAL.store(false, Ordering::Release);
        
        #[cfg(target_arch = "arm")]
        unsafe { cortex_m::interrupt::enable() };
    }
}

/// Execute a closure with interrupts disabled
/// 
/// The closure receives a reference to the critical section guard,
/// which can be used to access [`CsCell`] protected data.
#[inline]
pub fn critical_section<F, R>(f: F) -> R
where
    F: FnOnce(&CriticalSection) -> R,
{
    let cs = CriticalSection::enter();
    f(&cs)
}

/// Check if currently executing in an ISR context
#[inline]
pub fn is_isr_context() -> bool {
    #[cfg(target_arch = "arm")]
    {
        let ipsr: u32;
        unsafe {
            core::arch::asm!(
                "mrs {}, IPSR",
                out(reg) ipsr,
                options(nomem, nostack, preserves_flags)
            );
        }
        ipsr != 0
    }
    
    #[cfg(not(target_arch = "arm"))]
    {
        false
    }
}

/// Mask priority levels using BASEPRI (Cortex-M3/M4/M7)
/// 
/// This allows selective interrupt masking where only interrupts
/// with a priority value >= the mask value are blocked.
#[inline]
pub fn set_basepri(priority: u8) {
    #[cfg(target_arch = "arm")]
    unsafe {
        core::arch::asm!(
            "msr BASEPRI, {}",
            in(reg) priority as u32,
            options(nomem, nostack, preserves_flags)
        );
    }
    
    #[cfg(not(target_arch = "arm"))]
    {
        let _ = priority;
    }
}

/// Get current BASEPRI value
#[inline]
pub fn get_basepri() -> u8 {
    #[cfg(target_arch = "arm")]
    {
        let basepri: u32;
        unsafe {
            core::arch::asm!(
                "mrs {}, BASEPRI",
                out(reg) basepri,
                options(nomem, nostack, preserves_flags)
            );
        }
        basepri as u8
    }
    
    #[cfg(not(target_arch = "arm"))]
    {
        0
    }
}
