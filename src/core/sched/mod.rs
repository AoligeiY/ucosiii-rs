//! Scheduler module
//!
//! Priority-based preemptive scheduler with round-robin for same priority.

mod rdy_list;

pub use rdy_list::ReadyList;

use core::ptr::NonNull;

use crate::config::CFG_SCHED_ROUND_ROBIN_EN;
use crate::critical::{critical_section, CriticalSection, is_isr_context};

use crate::kernel;
use crate::task::OsTcb;
use crate::types::OsPrio;

/// Main scheduling point
///
/// This function determines the highest priority ready task and
/// triggers a context switch if needed. It should be called:
/// - After any operation that may change task readiness
/// - After releasing a semaphore/mutex
/// - After resuming a task
/// - When a delay/timeout expires
pub fn os_sched() {
    if !kernel::KERNEL.is_running() {
        return;
    }
    
    if is_isr_context() {
        return;
    }

    if kernel::KERNEL.sched_lock_nesting() > 0 {
        return;
    }

    let _cs = CriticalSection::enter();

    let high_prio = unsafe { kernel::prio_table().get_highest() };
    
    unsafe {
        if let Some(high_rdy) = kernel::rdy_list(high_prio).head() {
            kernel::set_prio_high_rdy(high_prio);
            kernel::set_tcb_high_rdy_ptr(Some(high_rdy));
            
            if Some(high_rdy) != kernel::tcb_cur_ptr() {
                crate::port::os_ctx_sw();
            }
        }
    }
}

/// Round-robin scheduling for tasks at the same priority
pub fn os_sched_round_robin() {
    if !CFG_SCHED_ROUND_ROBIN_EN {
        return;
    }

    if !kernel::KERNEL.is_running() {
        return;
    }

    if kernel::KERNEL.sched_lock_nesting() > 0 {
        return;
    }

    critical_section(|_cs| {
        unsafe {
            if let Some(cur_tcb_ptr) = kernel::tcb_cur_ptr() {
                let cur_tcb = &mut *cur_tcb_ptr.as_ptr();
                
                if cur_tcb.time_quanta_ctr > 0 {
                    cur_tcb.time_quanta_ctr -= 1;
                }
                
                if cur_tcb.time_quanta_ctr == 0 {
                    cur_tcb.time_quanta_ctr = cur_tcb.time_quanta;
                    
                    let prio = cur_tcb.prio;
                    let rdy_list = kernel::rdy_list(prio);
                    
                    // Only rotate if more than one task at this priority
                    if rdy_list.head() != rdy_list.tail() {
                        rdy_list.remove(cur_tcb_ptr);
                        rdy_list.insert_tail(cur_tcb_ptr);
                        
                        if let Some(new_head) = rdy_list.head() {
                            kernel::set_tcb_high_rdy_ptr(Some(new_head));
                        }
                        
                        crate::port::os_ctx_sw();
                    }
                }
            }
        }
    });
}

/// Make a task ready
pub(crate) unsafe fn os_rdy_list_insert(tcb: NonNull<OsTcb>) {
    let tcb_ref = unsafe { tcb.as_ref() };
    let prio = tcb_ref.prio;
    
    unsafe {
        let rdy_list = kernel::rdy_list(prio);
        rdy_list.insert_tail(tcb);
        kernel::prio_table().insert(prio);
    }
}

/// Remove a task from ready list
pub(crate) unsafe fn os_rdy_list_remove(tcb: NonNull<OsTcb>) {
    let tcb_ref = unsafe { tcb.as_ref() };
    let prio = tcb_ref.prio;
    
    unsafe {
        let rdy_list = kernel::rdy_list(prio);
        rdy_list.remove(tcb);
        
        if rdy_list.is_empty() {
            kernel::prio_table().remove(prio);
        }
    }
}

/// Move task to different priority
pub(crate) unsafe fn os_rdy_list_change_prio(
    tcb: NonNull<OsTcb>,
    new_prio: OsPrio,
) {
    let tcb_ref = unsafe { &mut *tcb.as_ptr() };
    let old_prio = tcb_ref.prio;
    
    if old_prio == new_prio {
        return;
    }

    unsafe {
        let old_rdy_list = kernel::rdy_list(old_prio);
        old_rdy_list.remove(tcb);
        if old_rdy_list.is_empty() {
            kernel::prio_table().remove(old_prio);
        }
    }

    tcb_ref.prio = new_prio;
    
    unsafe {
        let new_rdy_list = kernel::rdy_list(new_prio);
        new_rdy_list.insert_tail(tcb);
        kernel::prio_table().insert(new_prio);
    }
}
