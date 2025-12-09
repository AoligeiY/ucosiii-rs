//! Task Control Block (TCB) definition
//!
//! The TCB contains all the information needed to manage a task.

use core::ptr::NonNull;

use crate::types::{
    OsFlags, OsMsgSize, OsNestingCtr, OsOpt, OsPendOn, OsPendStatus,
    OsPrio, OsSemCtr, OsStkElement, OsTaskState, OsTick,
};

/// Task Control Block
#[repr(C)]
pub struct OsTcb {
    // ============ Stack pointer ============
    /// Current stack pointer
    pub stk_ptr: *mut OsStkElement,

    // ============ Stack information ============
    /// Base of stack
    pub stk_base: *mut OsStkElement,
    /// Stack limit pointer
    pub stk_limit: *mut OsStkElement,
    /// Stack size in words
    pub stk_size: usize,

    // ============ Task identification ============
    /// Task name
    pub name: &'static str,

    // ============ Ready list links ============
    /// Next TCB in ready list
    pub next_ptr: Option<NonNull<OsTcb>>,
    /// Previous TCB in ready list
    pub prev_ptr: Option<NonNull<OsTcb>>,

    // ============ Pend list links ============
    /// Next TCB in pend list
    pub pend_next_ptr: Option<NonNull<OsTcb>>,
    /// Previous TCB in pend list
    pub pend_prev_ptr: Option<NonNull<OsTcb>>,
    /// Object this task is pending on
    pub pend_obj_ptr: *const (),
    /// What type of object the task is pending on
    pub pend_on: OsPendOn,
    /// Result of pend operation
    pub pend_status: OsPendStatus,

    // ============ Tick list links ============
    /// Next TCB in tick list
    pub tick_next_ptr: Option<NonNull<OsTcb>>,
    /// Previous TCB in tick list
    pub tick_prev_ptr: Option<NonNull<OsTcb>>,
    /// Remaining ticks for delay/timeout
    pub tick_remain: OsTick,
    /// Which tick wheel slot this task is in
    pub tick_wheel_slot: u8,

    // ============ Priority ============
    /// Current priority
    pub prio: OsPrio,
    /// Base priority
    pub base_prio: OsPrio,

    // ============ State ============
    /// Current task state
    pub task_state: OsTaskState,
    /// Task options
    pub opt: OsOpt,

    // ============ Suspend ============
    /// Suspend nesting counter
    pub suspend_ctr: OsNestingCtr,

    // ============ Time slicing ============
    /// Time quanta for this task
    pub time_quanta: OsTick,
    /// Remaining time quanta
    pub time_quanta_ctr: OsTick,

    // ============ Task semaphore ============
    /// Task-specific semaphore counter
    pub sem_ctr: OsSemCtr,

    // ============ Event flags ============
    /// Flags being waited for
    pub flags_pend: OsFlags,
    /// Flags that made the task ready
    pub flags_rdy: OsFlags,
    /// Flag options
    pub flags_opt: OsOpt,

    // ============ Message ============
    /// Message pointer
    pub msg_ptr: *const (),
    /// Message size
    pub msg_size: OsMsgSize,

    // ============ Mutex priority inheritance ============
    /// Head of list of mutexes owned by this task
    pub mutex_grp_head: *const (),

    // ============ Task entry point ============
    /// Task function address
    pub task_entry_addr: u32,
    /// Task argument
    pub task_entry_arg: *mut (),

    // ============ Extension pointer ============
    /// User-defined extension data
    pub ext_ptr: *mut (),
}

impl OsTcb {
    /// Create a new, uninitialized TCB
    pub const fn new() -> Self {
        OsTcb {
            stk_ptr: core::ptr::null_mut(),
            stk_base: core::ptr::null_mut(),
            stk_limit: core::ptr::null_mut(),
            stk_size: 0,
            
            name: "",
            
            next_ptr: None,
            prev_ptr: None,
            
            pend_next_ptr: None,
            pend_prev_ptr: None,
            pend_obj_ptr: core::ptr::null(),
            pend_on: OsPendOn::Nothing,
            pend_status: OsPendStatus::Ok,
            
            tick_next_ptr: None,
            tick_prev_ptr: None,
            tick_remain: 0,
            tick_wheel_slot: 0,
            
            prio: 0,
            base_prio: 0,
            
            task_state: OsTaskState::Ready,
            opt: 0,
            
            suspend_ctr: 0,
            
            time_quanta: 0,
            time_quanta_ctr: 0,
            
            sem_ctr: 0,
            
            flags_pend: 0,
            flags_rdy: 0,
            flags_opt: 0,
            
            msg_ptr: core::ptr::null(),
            msg_size: 0,
            
            mutex_grp_head: core::ptr::null(),
            
            task_entry_addr: 0,
            task_entry_arg: core::ptr::null_mut(),
            
            ext_ptr: core::ptr::null_mut(),
        }
    }

    /// Initialize TCB to default values
    pub fn init(&mut self) {
        *self = Self::new();
    }

    /// Check if task is ready to run
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.task_state == OsTaskState::Ready
    }

    /// Check if task is pending
    #[inline]
    pub fn is_pending(&self) -> bool {
        matches!(
            self.task_state,
            OsTaskState::Pend | OsTaskState::PendTimeout |
            OsTaskState::PendSuspended | OsTaskState::PendTimeoutSuspended
        )
    }

    /// Check if task is suspended
    #[inline]
    pub fn is_suspended(&self) -> bool {
        matches!(
            self.task_state,
            OsTaskState::Suspended | OsTaskState::DelayedSuspended |
            OsTaskState::PendSuspended | OsTaskState::PendTimeoutSuspended
        )
    }

    /// Check if task is delayed
    #[inline]
    pub fn is_delayed(&self) -> bool {
        matches!(
            self.task_state,
            OsTaskState::Delayed | OsTaskState::DelayedSuspended
        )
    }
}

impl Default for OsTcb {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for OsTcb {}
unsafe impl Sync for OsTcb {}
