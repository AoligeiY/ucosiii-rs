//! Core type definitions for μC/OS-III
//!
//! These types provide strong typing for RTOS primitives.

/// Task priority (0 = highest priority)
pub type OsPrio = u8;

/// Tick counter type
pub type OsTick = u32;

/// Semaphore counter type
pub type OsSemCtr = u32;

/// Nesting counter
pub type OsNestingCtr = u8;

/// Option flags for API calls
pub type OsOpt = u16;

/// Message size type
pub type OsMsgSize = usize;

/// Object quantity type
pub type OsObjQty = u16;

/// Stack element type
pub type OsStkElement = u32;

/// Event flags type
pub type OsFlags = u32;

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OsTaskState {
    /// Task is ready to run
    Ready = 0,
    /// Task is delayed
    Delayed = 1,
    /// Task is pending on a kernel object
    Pend = 2,
    /// Task is pending with timeout
    PendTimeout = 3,
    /// Task is suspended
    Suspended = 4,
    /// Task is delayed and suspended
    DelayedSuspended = 5,
    /// Task is pending and suspended
    PendSuspended = 6,
    /// Task is pending with timeout and suspended
    PendTimeoutSuspended = 7,
}

/// What the task is pending on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OsPendOn {
    Nothing = 0,
    Flag = 1,
    Mutex = 2,
    Queue = 3,
    Semaphore = 4,
    TaskSem = 5,
    TaskQueue = 6,
    Cond = 7,
}

/// Pend status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OsPendStatus {
    /// Pend succeeded
    Ok = 0,
    /// Pend was aborted
    Abort = 1,
    /// Object was deleted while pending
    Del = 2,
    /// Timeout occurred
    Timeout = 3,
}

/// Kernel object type marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OsObjType {
    None = 0,
    Flag = 0x464C4147,    // 'FLAG'
    Mem = 0x4D454D20,     // 'MEM '
    Mutex = 0x4D555458,   // 'MUTX'
    Queue = 0x51554555,   // 'QUEU'
    Sem = 0x53454D41,     // 'SEMA'
    Task = 0x5441534B,    // 'TASK'
    Timer = 0x544D5220,   // 'TMR '
}

// ============ Option flags ============

/// Delete options
pub mod opt {
    use super::OsOpt;
    
    pub const NONE: OsOpt = 0x0000;
    
    // Delete options
    pub const DEL_NO_PEND: OsOpt = 0x0000;
    pub const DEL_ALWAYS: OsOpt = 0x0001;
    
    // Pend options
    pub const PEND_BLOCKING: OsOpt = 0x0000;
    pub const PEND_NON_BLOCKING: OsOpt = 0x8000;
    
    // Post options
    pub const POST_FIFO: OsOpt = 0x0000;
    pub const POST_LIFO: OsOpt = 0x0010;
    pub const POST_ALL: OsOpt = 0x0200;
    pub const POST_NO_SCHED: OsOpt = 0x8000;
    
    // Task options
    pub const TASK_NONE: OsOpt = 0x0000;
    pub const TASK_STK_CHK: OsOpt = 0x0001;
    pub const TASK_STK_CLR: OsOpt = 0x0002;
    pub const TASK_SAVE_FP: OsOpt = 0x0004;
    
    // Flag options
    pub const FLAG_CLR_ALL: OsOpt = 0x0001;
    pub const FLAG_CLR_ANY: OsOpt = 0x0002;
    pub const FLAG_SET_ALL: OsOpt = 0x0004;
    pub const FLAG_SET_ANY: OsOpt = 0x0008;
    pub const FLAG_CONSUME: OsOpt = 0x0100;
}
