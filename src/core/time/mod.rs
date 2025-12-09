//! Time management module
//!
//! Provides tick handling, time delays, and timeout management.

use core::ptr::NonNull;

use crate::config::{CFG_TICK_RATE_HZ, CFG_TICK_WHEEL_SIZE};
use crate::critical::{critical_section, is_isr_context};
use crate::error::{OsError, OsResult};
use crate::kernel;
use crate::sched;
use crate::task::OsTcb;
use crate::types::{OsTaskState, OsTick};

/// Time delay in ticks
///
/// Delays the calling task for the specified number of system ticks.
/// The task is removed from the ready list and placed on the tick list.
/// When the delay expires, the tick handler moves the task back to ready.
///
/// # Arguments
/// * `ticks` - Number of ticks to delay (0 = no delay)
///
/// # Returns
/// * `Ok(())` - Delay completed
/// * `Err(OsError::TimeDlyIsr)` - Cannot delay from ISR
/// * `Err(OsError::SchedLocked)` - Scheduler is locked
pub fn os_time_dly(ticks: OsTick) -> OsResult<()> {
    if !kernel::KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }

    if is_isr_context() {
        return Err(OsError::TimeDlyIsr);
    }

    if kernel::KERNEL.sched_lock_nesting() > 0 {
        return Err(OsError::SchedLocked);
    }

    if ticks == 0 {
        return Ok(());
    }

    critical_section(|_cs| {
        unsafe {
            if let Some(cur_tcb) = kernel::tcb_cur_ptr() {
                let tcb = &mut *cur_tcb.as_ptr();
                
                // Set delay tick count
                tcb.tick_remain = ticks;
                tcb.task_state = OsTaskState::Delayed;
                
                let current_tick = kernel::KERNEL.tick_get();
                let expiry_tick = current_tick.wrapping_add(ticks);
                kernel::tick_wheel_insert(cur_tcb, expiry_tick);
                
                sched::os_rdy_list_remove(cur_tcb);
            }
        }
    });
    
    sched::os_sched();

    Ok(())
}

/// Time delay in hours, minutes, seconds, milliseconds
///
/// # Arguments
/// * `hours` - Hours (0-999)
/// * `minutes` - Minutes (0-59)
/// * `seconds` - Seconds (0-59)
/// * `milliseconds` - Milliseconds (0-999)
pub fn os_time_dly_hmsm(
    hours: u16,
    minutes: u8,
    seconds: u8,
    milliseconds: u16,
) -> OsResult<()> {
    if minutes > 59 {
        return Err(OsError::StateInvalid);
    }
    if seconds > 59 {
        return Err(OsError::StateInvalid);
    }
    if milliseconds > 999 {
        return Err(OsError::StateInvalid);
    }

    let total_ms = (hours as u32) * 3600_000
        + (minutes as u32) * 60_000
        + (seconds as u32) * 1000
        + (milliseconds as u32);

    let ticks = (total_ms * CFG_TICK_RATE_HZ) / 1000;

    os_time_dly(ticks)
}

/// Resume a delayed task before its delay expires
pub fn os_time_dly_resume(tcb: NonNull<OsTcb>) -> OsResult<()> {
    if !kernel::KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }

    if is_isr_context() {
        return Err(OsError::TimeDlyIsr);
    }

    critical_section(|_cs| {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };

        if !tcb_ref.is_delayed() {
            return Err(OsError::TaskNotDly);
        }

        tcb_ref.tick_remain = 0;

        match tcb_ref.task_state {
            OsTaskState::Delayed => {
                tcb_ref.task_state = OsTaskState::Ready;
                unsafe { sched::os_rdy_list_insert(tcb) };
            }
            OsTaskState::DelayedSuspended => {
                tcb_ref.task_state = OsTaskState::Suspended;
            }
            _ => {}
        }

        sched::os_sched();

        Ok(())
    })
}

/// Get current tick count
#[inline]
pub fn os_time_get() -> OsTick {
    kernel::KERNEL.tick_get()
}

/// Tick handler
pub fn os_tick_handler() {
    if !kernel::KERNEL.is_running() {
        return;
    }

    kernel::KERNEL.int_enter();

    let _tick = kernel::KERNEL.tick_increment();

    critical_section(|_cs| {
        // Process delayed tasks
        process_delayed_tasks();
        // Round-robin time slicing
        sched::os_sched_round_robin();
    });

    kernel::os_int_exit();
}

/// Process delayed tasks in the current tick wheel slot
fn process_delayed_tasks() {
    let current_tick = kernel::KERNEL.tick_get();
    let slot = (current_tick as usize) % CFG_TICK_WHEEL_SIZE;
    
    unsafe {
        let mut current = kernel::tick_wheel_head(slot);
        
        while let Some(tcb_ptr) = current {
            let tcb = &mut *tcb_ptr.as_ptr();
            
            let next = tcb.tick_next_ptr;
            
            // Check if task is due this rotation
            if tcb.tick_remain <= CFG_TICK_WHEEL_SIZE as u32 {
                kernel::tick_wheel_remove(tcb_ptr);
                tcb.tick_remain = 0;
                
                match tcb.task_state {
                    OsTaskState::Delayed => {
                        tcb.task_state = OsTaskState::Ready;
                        sched::os_rdy_list_insert(tcb_ptr);
                    }
                    OsTaskState::DelayedSuspended => {
                        tcb.task_state = OsTaskState::Suspended;
                    }
                    OsTaskState::PendTimeout => {
                        tcb.task_state = OsTaskState::Ready;
                        tcb.pend_status = crate::types::OsPendStatus::Timeout;
                        sched::os_rdy_list_insert(tcb_ptr);
                    }
                    _ => {}
                }
            } else {
                tcb.tick_remain -= CFG_TICK_WHEEL_SIZE as u32;
            }
            
            current = next;
        }
    }
}

/// SysTick interrupt handler
#[no_mangle]
pub extern "C" fn SysTick() {
    os_tick_handler();
}
