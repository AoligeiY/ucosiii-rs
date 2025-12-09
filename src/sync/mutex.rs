//! Mutex implementation with priority inheritance
//!
//! Mutexes provide mutual exclusion with automatic priority boosting
//! to prevent priority inversion.

use core::ptr::NonNull;

use crate::critical::{critical_section, is_isr_context};
use crate::error::{OsError, OsResult};
use crate::kernel;
use crate::sched;
use crate::sem::PendList;
use crate::task::OsTcb;
use crate::types::{OsNestingCtr, OsObjType, OsOpt, OsPendOn, OsPendStatus, OsPrio, OsTaskState, OsTick, opt};

/// Mutex with priority inheritance
pub struct OsMutex {
    /// Object type marker
    obj_type: OsObjType,
    /// List of tasks waiting on this mutex
    pend_list: PendList,
    /// Task that owns the mutex
    owner: Option<NonNull<OsTcb>>,
    /// Nesting counter
    nesting_ctr: OsNestingCtr,
    /// Name for debugging
    #[cfg(feature = "defmt")]
    name: &'static str,
}

impl OsMutex {
    /// Create a new mutex
    pub const fn new() -> Self {
        OsMutex {
            obj_type: OsObjType::Mutex,
            pend_list: PendList::new(),
            owner: None,
            nesting_ctr: 0,
            #[cfg(feature = "defmt")]
            name: "",
        }
    }

    /// Initialize the mutex
    pub fn create(&mut self, _name: &'static str) -> OsResult<()> {
        if is_isr_context() {
            return Err(OsError::CreateIsr);
        }

        critical_section(|_cs| {
            self.obj_type = OsObjType::Mutex;
            self.pend_list.init();
            self.owner = None;
            self.nesting_ctr = 0;
            #[cfg(feature = "defmt")]
            {
                self.name = _name;
            }
            Ok(())
        })
    }

    /// Acquire the mutex
    ///
    /// If the mutex is owned by a lower-priority task, the owner's priority
    /// is temporarily boosted to prevent priority inversion.
    ///
    /// # Arguments
    /// * `timeout` - Maximum ticks to wait
    /// * `opt` - Pend options
    pub fn pend(&mut self, timeout: OsTick, pend_opt: OsOpt) -> OsResult<()> {
        if is_isr_context() {
            return Err(OsError::PendIsr);
        }

        if !kernel::KERNEL.is_running() {
            return Err(OsError::OsNotRunning);
        }

        if self.obj_type != OsObjType::Mutex {
            return Err(OsError::ObjType);
        }

        critical_section(|_cs| {
            let cur_tcb_ptr = unsafe { kernel::tcb_cur_ptr() }.ok_or(OsError::TcbInvalid)?;
            
            if self.owner.is_none() {
                self.owner = Some(cur_tcb_ptr);
                self.nesting_ctr = 1;
                return Ok(());
            }

            // Check if current task already owns it
            if self.owner == Some(cur_tcb_ptr) {
                if self.nesting_ctr == OsNestingCtr::MAX {
                    return Err(OsError::MutexOvf);
                }
                self.nesting_ctr += 1;
                return Ok(());
            }

            // Mutex is owned by another task
            if pend_opt & opt::PEND_NON_BLOCKING != 0 {
                return Err(OsError::PendWouldBlock);
            }

            if kernel::KERNEL.sched_lock_nesting() > 0 {
                return Err(OsError::SchedLocked);
            }

            // Priority inheritance
            let cur_tcb = unsafe { cur_tcb_ptr.as_ref() };
            let cur_prio = cur_tcb.prio;

            if let Some(owner_ptr) = self.owner {
                let owner = unsafe { &mut *owner_ptr.as_ptr() };
                if cur_prio < owner.prio {
                    if owner.task_state == OsTaskState::Ready {
                        unsafe { sched::os_rdy_list_change_prio(owner_ptr, cur_prio) };
                    } else {
                        owner.prio = cur_prio;
                    }
                }
            }

            // Block current task
            unsafe {
                let cur_tcb = &mut *cur_tcb_ptr.as_ptr();

                sched::os_rdy_list_remove(cur_tcb_ptr);

                cur_tcb.pend_on = OsPendOn::Mutex;
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

            sched::os_sched();

            unsafe {
                let cur_tcb = cur_tcb_ptr.as_ref();
                match cur_tcb.pend_status {
                    OsPendStatus::Ok => Ok(()),
                    OsPendStatus::Timeout => Err(OsError::Timeout),
                    OsPendStatus::Abort => Err(OsError::PendAbort),
                    OsPendStatus::Del => Err(OsError::ObjDel),
                }
            }
        })
    }

    /// Release the mutex
    ///
    /// If the current task's priority was boosted due to priority inheritance,
    /// it is restored to its base priority.
    pub fn post(&mut self, post_opt: OsOpt) -> OsResult<()> {
        if is_isr_context() {
            return Err(OsError::AcceptIsr);
        }

        if !kernel::KERNEL.is_running() {
            return Err(OsError::OsNotRunning);
        }

        if self.obj_type != OsObjType::Mutex {
            return Err(OsError::ObjType);
        }

        critical_section(|_cs| {
            let cur_tcb_ptr = unsafe { kernel::tcb_cur_ptr() }.ok_or(OsError::TcbInvalid)?;

            if self.owner != Some(cur_tcb_ptr) {
                return Err(OsError::MutexNotOwner);
            }

            if self.nesting_ctr > 1 {
                self.nesting_ctr -= 1;
                return Ok(());
            }

            // Unlock completely
            self.nesting_ctr = 0;

            // Restore owner's priority if it was boosted
            let cur_tcb = unsafe { &mut *cur_tcb_ptr.as_ptr() };
            if cur_tcb.prio != cur_tcb.base_prio {
                if cur_tcb.task_state == OsTaskState::Ready {
                    unsafe { sched::os_rdy_list_change_prio(cur_tcb_ptr, cur_tcb.base_prio) };
                }
                cur_tcb.prio = cur_tcb.base_prio;
            }

            if let Some(waiter_ptr) = self.pend_list.head() {
                let waiter = unsafe { &mut *waiter_ptr.as_ptr() };

                self.pend_list.remove(waiter_ptr);

                waiter.pend_on = OsPendOn::Nothing;
                waiter.pend_status = OsPendStatus::Ok;
                waiter.pend_obj_ptr = core::ptr::null();
                waiter.tick_remain = 0;
                waiter.task_state = OsTaskState::Ready;

                self.owner = Some(waiter_ptr);
                self.nesting_ctr = 1;

                unsafe { sched::os_rdy_list_insert(waiter_ptr) };

                if post_opt & opt::POST_NO_SCHED == 0 {
                    sched::os_sched();
                }
            } else {
                self.owner = None;
            }

            Ok(())
        })
    }

    /// Check if mutex is owned
    #[inline]
    pub fn is_owned(&self) -> bool {
        self.owner.is_some()
    }

    /// Get owner's priority
    pub fn owner_prio(&self) -> Option<OsPrio> {
        self.owner.map(|ptr| unsafe { ptr.as_ref().prio })
    }
}

impl Default for OsMutex {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Safe Wrapper ============

use core::cell::UnsafeCell;
pub struct Mutex {
    inner: UnsafeCell<OsMutex>,
}

unsafe impl Sync for Mutex {}
unsafe impl Send for Mutex {}

impl Mutex {
    pub const fn new() -> Self {
        Mutex {
            inner: UnsafeCell::new(OsMutex::new()),
        }
    }

    pub fn create(&self, name: &'static str) -> OsResult<()> {
        unsafe { (*self.inner.get()).create(name) }
    }

    pub fn lock(&self, timeout: OsTick, opt: OsOpt) -> OsResult<()> {
        unsafe { (*self.inner.get()).pend(timeout, opt) }
    }

    pub fn unlock(&self, opt: OsOpt) -> OsResult<()> {
        unsafe { (*self.inner.get()).post(opt) }
    }

    #[inline]
    pub fn is_owned(&self) -> bool {
        unsafe { (*self.inner.get()).is_owned() }
    }
}

impl Default for Mutex {
    fn default() -> Self {
        Self::new()
    }
}
