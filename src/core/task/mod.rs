//! Task management module
//!
//! Provides task creation, deletion, and control functions.

mod tcb;

pub use tcb::OsTcb;

use core::ptr::NonNull;

use crate::config::{CFG_PRIO_MAX, CFG_STK_SIZE_MIN, CFG_TIME_QUANTA_DEFAULT};
use crate::critical::{critical_section, is_isr_context};
use crate::error::{OsError, OsResult};
use crate::kernel;
use crate::types::{OsOpt, OsPrio, OsStkElement, OsTaskState, OsTick};

/// Task entry point function type
pub type OsTaskFn = fn(*mut ()) -> !;

/// Create a new task
///
/// # Arguments
/// * `tcb` - Pointer to the Task Control Block
/// * `name` - Task name for debugging
/// * `task_fn` - Task entry point function
/// * `arg` - Argument to pass to task function
/// * `prio` - Task priority
/// * `stk_base` - Pointer to base of stack array
/// * `stk_size` - Stack size in words
/// * `opt` - Task options
unsafe fn os_task_create_raw(
    tcb: *mut OsTcb,
    name: &'static str,
    task_fn: OsTaskFn,
    arg: *mut (),
    prio: OsPrio,
    stk_base: *mut OsStkElement,
    stk_size: usize,
    time_quanta: OsTick,
    opt: OsOpt,
) -> OsResult<()> {
    if tcb.is_null() {
        return Err(OsError::TcbInvalid);
    }
    
    if stk_base.is_null() {
        return Err(OsError::StkInvalid);
    }
    
    if stk_size < CFG_STK_SIZE_MIN {
        return Err(OsError::StkSizeInvalid);
    }
    
    if prio as usize >= CFG_PRIO_MAX {
        return Err(OsError::PrioInvalid);
    }
    
    if is_isr_context() {
        return Err(OsError::TaskCreateIsr);
    }

    critical_section(|_cs| {
        // Initialize TCB
        let tcb_ref = unsafe { &mut *tcb };
        tcb_ref.init();
        
        tcb_ref.name = name;
        tcb_ref.prio = prio;
        tcb_ref.base_prio = prio;
        tcb_ref.time_quanta = time_quanta;
        tcb_ref.time_quanta_ctr = time_quanta;
        tcb_ref.opt = opt;
        tcb_ref.task_state = OsTaskState::Ready;
        
        // Initialize stack
        let stk_ptr = unsafe {
            crate::port::os_task_stk_init(task_fn, arg, stk_base, stk_size, opt)
        };
        tcb_ref.stk_ptr = stk_ptr;
        tcb_ref.stk_base = stk_base;
        tcb_ref.stk_size = stk_size;
        tcb_ref.stk_limit = unsafe { stk_base.add(stk_size / 10) }; // 10% watermark
        
        // Store task entry point
        tcb_ref.task_entry_addr = task_fn as u32;
        tcb_ref.task_entry_arg = arg;

        // Add to ready list
        let tcb_nonnull = unsafe { NonNull::new_unchecked(tcb) };
        unsafe {
            let prio_tbl = kernel::prio_table();
            let rdy_list = kernel::rdy_list(prio);
            
            rdy_list.insert_tail(tcb_nonnull);
            prio_tbl.insert(prio);
        }
        
        if kernel::KERNEL.is_running() {
            crate::sched::os_sched();
        }
        
        Ok(())
    })
}

/// Create a new task using static references
///
/// This is the recommended way to create tasks
///
/// # Arguments
/// * `tcb` - Static mutable reference to the Task Control Block
/// * `stack` - Static mutable reference to the stack array
/// * `name` - Task name for debugging
/// * `task_fn` - Task entry point function
/// * `prio` - Task priority (0 = highest)
///
/// # Example
/// ```ignore
/// static mut TASK_TCB: OsTcb = OsTcb::new();
/// static mut TASK_STK: [OsStkElement; 256] = [0; 256];
///
/// fn my_task(_: *mut ()) -> ! {
///     loop { /* ... */ }
/// }
///
/// // In main:
/// os_task_create(
///     unsafe { &mut TASK_TCB },
///     unsafe { &mut TASK_STK },
///     "MyTask",
///     my_task,
///     5,
/// ).expect("Task creation failed");
/// ```
pub fn os_task_create(
    tcb: &'static mut OsTcb,
    stack: &'static mut [OsStkElement],
    name: &'static str,
    task_fn: OsTaskFn,
    prio: OsPrio,
) -> OsResult<()> {
    unsafe {
        os_task_create_raw(
            tcb as *mut OsTcb,
            name,
            task_fn,
            core::ptr::null_mut(),
            prio,
            stack.as_mut_ptr(),
            stack.len(),
            CFG_TIME_QUANTA_DEFAULT,
            0,
        )
    }
}

/// Internal task creation for kernel use
#[doc(hidden)]
pub unsafe fn os_task_create_internal(
    tcb: *mut OsTcb,
    name: &'static str,
    task_fn: OsTaskFn,
    arg: *mut (),
    prio: OsPrio,
    stk_base: *mut OsStkElement,
    stk_size: usize,
    time_quanta: OsTick,
    opt: OsOpt,
) -> OsResult<()> {
    if tcb.is_null() || stk_base.is_null() {
        return Err(OsError::TcbInvalid);
    }

    // Initialize TCB
    let tcb_ref = unsafe { &mut *tcb };
    tcb_ref.init();
    
    tcb_ref.name = name;
    tcb_ref.prio = prio;
    tcb_ref.base_prio = prio;
    tcb_ref.time_quanta = time_quanta;
    tcb_ref.time_quanta_ctr = time_quanta;
    tcb_ref.opt = opt;
    tcb_ref.task_state = OsTaskState::Ready;
    
    // Initialize stack
    let stk_ptr = unsafe {
        crate::port::os_task_stk_init(task_fn, arg, stk_base, stk_size, opt)
    };
    tcb_ref.stk_ptr = stk_ptr;
    tcb_ref.stk_base = stk_base;
    tcb_ref.stk_size = stk_size;
    tcb_ref.stk_limit = unsafe { stk_base.add(stk_size / 10) };
    
    tcb_ref.task_entry_addr = task_fn as u32;
    tcb_ref.task_entry_arg = arg;
    
    // Add to ready list
    let tcb_nonnull = unsafe { NonNull::new_unchecked(tcb) };
    unsafe {
        let prio_tbl = kernel::prio_table();
        let rdy_list = kernel::rdy_list(prio);
        
        rdy_list.insert_tail(tcb_nonnull);
        prio_tbl.insert(prio);
    }
    
    Ok(())
}

/// Delete a task
pub fn os_task_del(tcb: Option<NonNull<OsTcb>>) -> OsResult<()> {
    if !kernel::KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }
    
    if is_isr_context() {
        return Err(OsError::TaskDelIsr);
    }

    critical_section(|_cs| {
        let tcb_ptr = match tcb {
            Some(ptr) => ptr,
            None => {
                // Delete self
                unsafe { kernel::tcb_cur_ptr() }.ok_or(OsError::TcbInvalid)?
            }
        };

        let tcb_ref = unsafe { tcb_ptr.as_ref() };
        let prio = tcb_ref.prio;
        
        if prio == crate::config::CFG_PRIO_IDLE {
            return Err(OsError::TaskDelIdle);
        }

        // Remove from ready list
        unsafe {
            let rdy_list = kernel::rdy_list(prio);
            rdy_list.remove(tcb_ptr);
            
            if rdy_list.is_empty() {
                kernel::prio_table().remove(prio);
            }
        }

        let tcb_mut = unsafe { &mut *tcb_ptr.as_ptr() };
        tcb_mut.task_state = OsTaskState::Suspended;

        // If deleting current task, trigger reschedule
        let is_current = unsafe { kernel::tcb_cur_ptr() } == Some(tcb_ptr);
        if is_current {
            crate::sched::os_sched();
        }

        Ok(())
    })
}

/// Suspend a task
pub fn os_task_suspend(tcb: Option<NonNull<OsTcb>>) -> OsResult<()> {
    if !kernel::KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }

    if is_isr_context() {
        return Err(OsError::TaskSuspendIsr);
    }

    critical_section(|_cs| {
        let tcb_ptr = match tcb {
            Some(ptr) => ptr,
            None => unsafe { kernel::tcb_cur_ptr() }.ok_or(OsError::TcbInvalid)?,
        };

        let tcb_ref = unsafe { &mut *tcb_ptr.as_ptr() };
        
        if tcb_ref.prio == crate::config::CFG_PRIO_IDLE {
            return Err(OsError::TaskSuspendIdle);
        }

        tcb_ref.suspend_ctr = tcb_ref.suspend_ctr.saturating_add(1);

        match tcb_ref.task_state {
            OsTaskState::Ready => {
                tcb_ref.task_state = OsTaskState::Suspended;
                unsafe {
                    let rdy_list = kernel::rdy_list(tcb_ref.prio);
                    rdy_list.remove(tcb_ptr);
                    if rdy_list.is_empty() {
                        kernel::prio_table().remove(tcb_ref.prio);
                    }
                }
            }
            OsTaskState::Delayed => {
                tcb_ref.task_state = OsTaskState::DelayedSuspended;
            }
            OsTaskState::Pend => {
                tcb_ref.task_state = OsTaskState::PendSuspended;
            }
            OsTaskState::PendTimeout => {
                tcb_ref.task_state = OsTaskState::PendTimeoutSuspended;
            }
            _ => {} // Already suspended
        }

        // Reschedule if suspended current task
        let is_current = unsafe { kernel::tcb_cur_ptr() } == Some(tcb_ptr);
        if is_current {
            crate::sched::os_sched();
        }

        Ok(())
    })
}

/// Resume a suspended task
pub fn os_task_resume(tcb: NonNull<OsTcb>) -> OsResult<()> {
    if !kernel::KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }

    if is_isr_context() {
        return Err(OsError::TaskResumeIsr);
    }

    critical_section(|_cs| {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };

        if tcb_ref.suspend_ctr == 0 {
            return Err(OsError::TaskNotSuspended);
        }

        tcb_ref.suspend_ctr -= 1;

        // Only resume if suspend counter reaches 0
        if tcb_ref.suspend_ctr == 0 {
            match tcb_ref.task_state {
                OsTaskState::Suspended => {
                    tcb_ref.task_state = OsTaskState::Ready;
                    unsafe {
                        let rdy_list = kernel::rdy_list(tcb_ref.prio);
                        rdy_list.insert_tail(tcb);
                        kernel::prio_table().insert(tcb_ref.prio);
                    }
                }
                OsTaskState::DelayedSuspended => {
                    tcb_ref.task_state = OsTaskState::Delayed;
                }
                OsTaskState::PendSuspended => {
                    tcb_ref.task_state = OsTaskState::Pend;
                }
                OsTaskState::PendTimeoutSuspended => {
                    tcb_ref.task_state = OsTaskState::PendTimeout;
                }
                _ => {}
            }

            crate::sched::os_sched();
        }

        Ok(())
    })
}
