//! Error types for μC/OS-III
//!
//! Uses Rust's Result pattern instead of C-style error pointers.

/// RTOS error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OsError {
    /// No error
    None = 0,

    // ============ ISR errors ============
    /// Function cannot be called from ISR
    AcceptIsr = 10001,
    /// Cannot create object from ISR
    CreateIsr = 12001,
    /// Cannot delete object from ISR
    DelIsr = 13001,
    /// Cannot flush from ISR
    FlushIsr = 15104,

    // ============ Fatal errors ============
    /// Fatal return (task returned unexpectedly)
    FatalReturn = 15001,

    // ============ Flag errors ============
    /// Flag group depleted
    FlagGrpDepleted = 15101,
    /// Flag not ready
    FlagNotRdy = 15102,
    /// Invalid flag pend option
    FlagPendOpt = 15103,

    // ============ Lock errors ============
    /// Lock nesting overflow
    LockNestingOvf = 21001,

    // ============ Memory errors ============
    /// Memory pool full
    MemFull = 22202,
    /// Invalid memory address
    MemInvalidAddr = 22203,
    /// No free blocks
    MemNoFreeBlks = 22210,

    // ============ Mutex errors ============
    /// Caller is not the mutex owner
    MutexNotOwner = 22401,
    /// Task already owns the mutex
    MutexOwner = 22402,
    /// Mutex nesting error
    MutexNesting = 22403,
    /// Mutex nesting overflow
    MutexOvf = 22404,

    // ============ Object errors ============
    /// Object already created
    ObjCreated = 24001,
    /// Object was deleted
    ObjDel = 24002,
    /// Null pointer for object
    ObjPtrNull = 24003,
    /// Wrong object type
    ObjType = 24004,

    // ============ Option errors ============
    /// Invalid option specified
    OptInvalid = 24101,

    // ============ OS state errors ============
    /// OS is not running
    OsNotRunning = 24201,
    /// OS is already running
    OsRunning = 24202,
    /// OS not initialized
    OsNotInit = 24203,
    /// No application task created
    OsNoAppTask = 24204,

    // ============ Pend errors ============
    /// Pend was aborted
    PendAbort = 25001,
    /// Cannot abort pend from ISR
    PendAbortIsr = 25002,
    /// No task to abort
    PendAbortNone = 25003,
    /// Cannot abort self
    PendAbortSelf = 25004,
    /// Object deleted while pending
    PendDel = 25005,
    /// Cannot pend from ISR
    PendIsr = 25006,
    /// Scheduler is locked
    PendLocked = 25007,
    /// Pend would block (non-blocking mode)
    PendWouldBlock = 25008,

    // ============ Priority errors ============
    /// Priority already exists
    PrioExist = 25201,
    /// Invalid priority
    PrioInvalid = 25203,

    // ============ Queue errors ============
    /// Queue is full
    QFull = 26001,
    /// Queue is empty
    QEmpty = 26002,
    /// Queue max size exceeded
    QMax = 26003,
    /// Message pool is empty (no free messages)
    MsgPoolEmpty = 26004,

    // ============ Scheduler errors ============
    /// Invalid time slice
    SchedInvalidTimeSlice = 28001,
    /// Cannot lock scheduler from ISR
    SchedLockIsr = 28002,
    /// Scheduler is locked
    SchedLocked = 28003,
    /// Scheduler is not locked
    SchedNotLocked = 28004,
    /// Cannot unlock scheduler from ISR
    SchedUnlockIsr = 28005,

    // ============ Semaphore errors ============
    /// Semaphore overflow
    SemOvf = 28101,

    // ============ State errors ============
    /// Invalid state
    StateInvalid = 28205,
    /// Invalid status
    StatusInvalid = 28206,
    /// Invalid stack pointer
    StkInvalid = 28207,
    /// Invalid stack size
    StkSizeInvalid = 28208,
    /// Stack overflow detected
    StkOvf = 28210,

    // ============ Task errors ============
    /// Cannot change priority from ISR
    TaskChangePrioIsr = 29001,
    /// Cannot create task from ISR
    TaskCreateIsr = 29002,
    /// Task delete error
    TaskDel = 29003,
    /// Cannot delete idle task
    TaskDelIdle = 29004,
    /// Invalid task for deletion
    TaskDelInvalid = 29005,
    /// Cannot delete task from ISR
    TaskDelIsr = 29006,
    /// Invalid task
    TaskInvalid = 29007,
    /// No more TCBs available
    TaskNoMoreTcb = 29008,
    /// Task is not delayed
    TaskNotDly = 29009,
    /// Task does not exist
    TaskNotExist = 29010,
    /// Task is not suspended
    TaskNotSuspended = 29011,
    /// Invalid task option
    TaskOpt = 29012,
    /// Task is running
    TaskRunning = 29016,
    /// Cannot suspend task from ISR
    TaskSuspendIsr = 29017,
    /// Task is suspended
    TaskSuspended = 29018,
    /// Cannot suspend idle task
    TaskSuspendIdle = 29019,
    /// Cannot resume task from ISR
    TaskResumeIsr = 29020,

    // ============ TCB errors ============
    /// Invalid TCB pointer
    TcbInvalid = 29101,

    // ============ Time errors ============
    /// Cannot delay from ISR
    TimeDlyIsr = 29301,
    /// Zero delay specified
    TimeZeroDly = 29310,

    // ============ Timeout ============
    /// Operation timed out
    Timeout = 29401,

    // ============ Timer errors ============
    /// Timer is inactive
    TmrInactive = 29501,
    /// Invalid timer delay
    TmrInvalidDly = 29503,
    /// Invalid timer period
    TmrInvalidPeriod = 29504,
    /// Invalid timer state
    TmrInvalidState = 29505,
    /// Timer ISR error
    TmrIsr = 29507,
    /// No timer callback
    TmrNoCallback = 29508,
    /// Timer stopped
    TmrStopped = 29513,

    // ============ Yield errors ============
    /// Cannot yield from ISR
    YieldIsr = 34001,
}

/// Result type alias for RTOS operations
pub type OsResult<T> = Result<T, OsError>;

impl OsError {
    #[inline]
    pub fn is_ok(self) -> bool {
        self == OsError::None
    }

    #[inline]
    pub fn is_err(self) -> bool {
        self != OsError::None
    }
}
