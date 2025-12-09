//! Semaphore implementation
//!
//! Counting semaphores for task synchronization and resource counting.

use core::ptr::NonNull;

use crate::critical::{critical_section, is_isr_context};
use crate::error::{OsError, OsResult};
use crate::kernel;
use crate::sched;
use crate::task::OsTcb;
use crate::types::{OsObjType, OsOpt, OsPendOn, OsPendStatus, OsSemCtr, OsTaskState, OsTick, opt};

/// Pend list for tasks waiting on a kernel object
#[derive(Debug)]
pub struct PendList {
    head: Option<NonNull<OsTcb>>,
    tail: Option<NonNull<OsTcb>>,
    #[cfg(feature = "defmt")]
    count: usize,
}

impl PendList {
    /// Create a new empty pend list
    pub const fn new() -> Self {
        PendList {
            head: None,
            tail: None,
            #[cfg(feature = "defmt")]
            count: 0,
        }
    }

    /// Initialize the pend list
    pub fn init(&mut self) {
        self.head = None;
        self.tail = None;
        #[cfg(feature = "defmt")]
        {
            self.count = 0;
        }
    }

    /// Check if list is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Get head of list
    #[inline(always)]
    pub fn head(&self) -> Option<NonNull<OsTcb>> {
        self.head
    }

    /// Insert TCB at tail
    pub fn insert(&mut self, tcb: NonNull<OsTcb>) {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };
        
        tcb_ref.pend_next_ptr = None;
        tcb_ref.pend_prev_ptr = self.tail;

        match self.tail {
            Some(tail) => {
                unsafe { (*tail.as_ptr()).pend_next_ptr = Some(tcb) };
            }
            None => {
                self.head = Some(tcb);
            }
        }

        self.tail = Some(tcb);

        #[cfg(feature = "defmt")]
        {
            self.count += 1;
        }
    }

    /// Insert in priority order
    pub fn insert_by_prio(&mut self, tcb: NonNull<OsTcb>) {
        let tcb_ref = unsafe { tcb.as_ref() };
        let prio = tcb_ref.prio;

        let mut current = self.head;
        let mut prev: Option<NonNull<OsTcb>> = None;

        while let Some(cur_ptr) = current {
            let cur_ref = unsafe { cur_ptr.as_ref() };
            if prio < cur_ref.prio {
                break;
            }
            prev = current;
            current = cur_ref.pend_next_ptr;
        }

        let tcb_mut = unsafe { &mut *tcb.as_ptr() };
        tcb_mut.pend_prev_ptr = prev;
        tcb_mut.pend_next_ptr = current;

        match prev {
            Some(p) => {
                unsafe { (*p.as_ptr()).pend_next_ptr = Some(tcb) };
            }
            None => {
                self.head = Some(tcb);
            }
        }

        match current {
            Some(c) => {
                unsafe { (*c.as_ptr()).pend_prev_ptr = Some(tcb) };
            }
            None => {
                self.tail = Some(tcb);
            }
        }

        #[cfg(feature = "defmt")]
        {
            self.count += 1;
        }
    }

    /// Remove specific TCB from list
    pub fn remove(&mut self, tcb: NonNull<OsTcb>) {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };

        match tcb_ref.pend_prev_ptr {
            Some(prev) => {
                unsafe { (*prev.as_ptr()).pend_next_ptr = tcb_ref.pend_next_ptr };
            }
            None => {
                self.head = tcb_ref.pend_next_ptr;
            }
        }

        match tcb_ref.pend_next_ptr {
            Some(next) => {
                unsafe { (*next.as_ptr()).pend_prev_ptr = tcb_ref.pend_prev_ptr };
            }
            None => {
                self.tail = tcb_ref.pend_prev_ptr;
            }
        }

        tcb_ref.pend_prev_ptr = None;
        tcb_ref.pend_next_ptr = None;

        #[cfg(feature = "defmt")]
        {
            self.count = self.count.saturating_sub(1);
        }
    }
}

impl Default for PendList {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for PendList {}
unsafe impl Sync for PendList {}

/// Counting semaphore
pub struct OsSem {
    /// Object type marker
    obj_type: OsObjType,
    /// List of tasks waiting on this semaphore
    pend_list: PendList,
    /// Current count
    count: OsSemCtr,
    /// Name for debugging
    #[cfg(feature = "defmt")]
    name: &'static str,
}

impl OsSem {
    /// Create a new semaphore
    ///
    /// # Arguments
    /// * `count` - Initial count value
    /// * `name` - Semaphore name
    pub const fn new(count: OsSemCtr) -> Self {
        OsSem {
            obj_type: OsObjType::Sem,
            pend_list: PendList::new(),
            count,
            #[cfg(feature = "defmt")]
            name: "",
        }
    }

    /// Initialize/create the semaphore
    pub fn create(&mut self, count: OsSemCtr, _name: &'static str) -> OsResult<()> {
        if is_isr_context() {
            return Err(OsError::CreateIsr);
        }

        critical_section(|_cs| {
            self.obj_type = OsObjType::Sem;
            self.pend_list.init();
            self.count = count;
            #[cfg(feature = "defmt")]
            {
                self.name = _name;
            }
            Ok(())
        })
    }

    /// Wait on (pend) the semaphore
    ///
    /// # Arguments
    /// * `timeout` - Maximum ticks to wait (0 = forever)
    /// * `opt` - Pend options
    ///
    /// # Returns
    /// * `Ok(count)` - Semaphore acquired, returns current count
    /// * `Err(OsError::Timeout)` - Timeout expired
    /// * `Err(OsError::PendWouldBlock)` - Non-blocking and not available
    pub fn pend(&mut self, timeout: OsTick, pend_opt: OsOpt) -> OsResult<OsSemCtr> {
        if is_isr_context() {
            return Err(OsError::PendIsr);
        }

        if !kernel::KERNEL.is_running() {
            return Err(OsError::OsNotRunning);
        }

        if self.obj_type != OsObjType::Sem {
            return Err(OsError::ObjType);
        }

        critical_section(|_cs| {
            if self.count > 0 {
                self.count -= 1;
                return Ok(self.count);
            }

            if pend_opt & opt::PEND_NON_BLOCKING != 0 {
                return Err(OsError::PendWouldBlock);
            }

            if kernel::KERNEL.sched_lock_nesting() > 0 {
                return Err(OsError::SchedLocked);
            }

            // Block current task
            unsafe {
                if let Some(cur_tcb_ptr) = kernel::tcb_cur_ptr() {
                    let cur_tcb = &mut *cur_tcb_ptr.as_ptr();

                    sched::os_rdy_list_remove(cur_tcb_ptr);

                    cur_tcb.pend_on = OsPendOn::Semaphore;
                    cur_tcb.pend_status = OsPendStatus::Ok;
                    cur_tcb.pend_obj_ptr = self as *const _ as *const ();
                    cur_tcb.tick_remain = timeout;
                    
                    if timeout > 0 {
                        cur_tcb.task_state = OsTaskState::PendTimeout;
                    } else {
                        cur_tcb.task_state = OsTaskState::Pend;
                    }

                    self.pend_list.insert_by_prio(cur_tcb_ptr);
                }
            }

            sched::os_sched();

            unsafe {
                if let Some(cur_tcb_ptr) = kernel::tcb_cur_ptr() {
                    let cur_tcb = cur_tcb_ptr.as_ref();
                    
                    match cur_tcb.pend_status {
                        OsPendStatus::Ok => Ok(self.count),
                        OsPendStatus::Timeout => Err(OsError::Timeout),
                        OsPendStatus::Abort => Err(OsError::PendAbort),
                        OsPendStatus::Del => Err(OsError::ObjDel),
                    }
                } else {
                    Err(OsError::TcbInvalid)
                }
            }
        })
    }

    /// Signal (post) the semaphore
    ///
    /// # Arguments
    /// * `opt` - Post options
    ///
    /// # Returns
    /// * `Ok(count)` - New count after post
    /// * `Err(OsError::SemOvf)` - Counter overflow
    pub fn post(&mut self, post_opt: OsOpt) -> OsResult<OsSemCtr> {
        if self.obj_type != OsObjType::Sem {
            return Err(OsError::ObjType);
        }

        critical_section(|_cs| {
            if let Some(tcb_ptr) = self.pend_list.head() {
                let tcb = unsafe { &mut *tcb_ptr.as_ptr() };

                self.pend_list.remove(tcb_ptr);

                tcb.pend_on = OsPendOn::Nothing;
                tcb.pend_status = OsPendStatus::Ok;
                tcb.pend_obj_ptr = core::ptr::null();
                tcb.tick_remain = 0;
                tcb.task_state = OsTaskState::Ready;

                unsafe { sched::os_rdy_list_insert(tcb_ptr) };

                if post_opt & opt::POST_NO_SCHED == 0 && !is_isr_context() {
                    sched::os_sched();
                }

                Ok(self.count)
            } else {
                if self.count == OsSemCtr::MAX {
                    return Err(OsError::SemOvf);
                }
                self.count += 1;
                Ok(self.count)
            }
        })
    }

    /// Get current semaphore count
    #[inline(always)]
    pub fn count(&self) -> OsSemCtr {
        self.count
    }

    /// Set semaphore count
    pub fn set(&mut self, count: OsSemCtr) -> OsResult<()> {
        if is_isr_context() {
            return Err(OsError::AcceptIsr);
        }

        critical_section(|_cs| {
            self.count = count;
            Ok(())
        })
    }
}

impl Default for OsSem {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============ Safe Wrapper ============

use core::cell::UnsafeCell;

pub struct Semaphore {
    inner: UnsafeCell<OsSem>,
}

unsafe impl Sync for Semaphore {}
unsafe impl Send for Semaphore {}

impl Semaphore {
    pub const fn new(count: OsSemCtr) -> Self {
        Semaphore {
            inner: UnsafeCell::new(OsSem::new(count)),
        }
    }

    pub fn create(&self, count: OsSemCtr, name: &'static str) -> OsResult<()> {
        unsafe { (*self.inner.get()).create(count, name) }
    }

    pub fn wait(&self, timeout: OsTick, opt: OsOpt) -> OsResult<OsSemCtr> {
        unsafe { (*self.inner.get()).pend(timeout, opt) }
    }

    pub fn signal(&self, opt: OsOpt) -> OsResult<OsSemCtr> {
        unsafe { (*self.inner.get()).post(opt) }
    }

    #[inline]
    pub fn count(&self) -> OsSemCtr {
        unsafe { (*self.inner.get()).count() }
    }
}

impl Default for Semaphore {
    fn default() -> Self {
        Self::new(0)
    }
}
