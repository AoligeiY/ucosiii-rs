//! Global kernel state and initialization
//!
//! This module manages the global OS state including initialization,
//! starting the scheduler, and tracking kernel status.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use crate::config::{CFG_PRIO_MAX, CFG_TICK_WHEEL_SIZE};
use crate::critical::{critical_section, CriticalSection};
use crate::core::cs_cell::CsCell;
use crate::error::{OsError, OsResult};
use crate::prio::PrioTable;
use crate::sched::ReadyList;
use crate::task::OsTcb;
use crate::types::{OsNestingCtr, OsPrio, OsTick};

// ============ Kernel State Structures ============

/// Atomic kernel flags
pub struct KernelFlags {
    initialized: AtomicBool,
    running: AtomicBool,
    int_nesting: AtomicU8,
    sched_lock_nesting: AtomicU8,
    tick_counter: AtomicU32,
    time: AtomicU32,
}

impl KernelFlags {
    const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            running: AtomicBool::new(false),
            int_nesting: AtomicU8::new(0),
            sched_lock_nesting: AtomicU8::new(0),
            tick_counter: AtomicU32::new(0),
            time: AtomicU32::new(0),
        }
    }

    pub(crate) fn reset(&self) {
        self.initialized.store(false, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);
        self.int_nesting.store(0, Ordering::SeqCst);
        self.sched_lock_nesting.store(0, Ordering::SeqCst);
        self.tick_counter.store(0, Ordering::SeqCst);
    }

    /// Check if the OS is running
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Check if OS is initialized
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Get current tick count
    #[inline(always)]
    pub fn tick_get(&self) -> OsTick {
        self.tick_counter.load(Ordering::Relaxed)
    }

    /// Get interrupt nesting level
    #[inline(always)]
    pub fn int_nesting(&self) -> OsNestingCtr {
        self.int_nesting.load(Ordering::Relaxed)
    }

    /// Get scheduler lock nesting level
    #[inline(always)]
    pub fn sched_lock_nesting(&self) -> OsNestingCtr {
        self.sched_lock_nesting.load(Ordering::SeqCst)
    }

    /// Increment and return tick count
    #[inline(always)]
    pub(crate) fn tick_increment(&self) -> OsTick {
        self.tick_counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Enter ISR
    #[inline(always)]
    pub(crate) fn int_enter(&self) {
        if self.is_running() {
            let nesting = self.int_nesting.fetch_add(1, Ordering::Relaxed);
            if nesting == 254 {
                self.int_nesting.store(254, Ordering::Relaxed);
            }
        }
    }

    /// Set initialized flag
    #[inline(always)]
    pub(crate) fn set_initialized(&self, val: bool) {
        self.initialized.store(val, Ordering::SeqCst);
    }

    /// Set running flag
    #[inline(always)]
    pub(crate) fn set_running(&self, val: bool) {
        self.running.store(val, Ordering::SeqCst);
    }

    /// Decrement int nesting
    #[inline(always)]
    pub(crate) fn int_nesting_dec(&self) -> OsNestingCtr {
        let nesting = self.int_nesting.load(Ordering::Relaxed);
        if nesting > 0 {
            self.int_nesting.store(nesting - 1, Ordering::Relaxed);
        }
        nesting.saturating_sub(1)
    }

    /// Lock scheduler 
    pub(crate) fn try_sched_lock(&self) -> OsResult<()> {
        let nesting = self.sched_lock_nesting.load(Ordering::SeqCst);
        if nesting == 255 {
            return Err(OsError::LockNestingOvf);
        }
        self.sched_lock_nesting.store(nesting + 1, Ordering::SeqCst);
        Ok(())
    }

    /// Unlock scheduler
    pub(crate) fn try_sched_unlock(&self) -> OsResult<OsNestingCtr> {
        let nesting = self.sched_lock_nesting.load(Ordering::SeqCst);
        if nesting == 0 {
            return Err(OsError::SchedNotLocked);
        }
        self.sched_lock_nesting.store(nesting - 1, Ordering::SeqCst);
        Ok(nesting - 1)
    }
}

// ============ Global Instances ============

/// Global kernel state instance
pub(crate) static KERNEL: KernelFlags = KernelFlags::new();

/// Scheduler state
pub struct SchedState {
    pub(crate) prio_tbl: PrioTable,
    pub(crate) rdy_list: [ReadyList; CFG_PRIO_MAX],
    pub(crate) tick_wheel: [Option<NonNull<OsTcb>>; CFG_TICK_WHEEL_SIZE],
}

impl SchedState {
    const fn new() -> Self {
        Self {
            prio_tbl: PrioTable::new(),
            rdy_list: [ReadyList::new(); CFG_PRIO_MAX],
            tick_wheel: [None; CFG_TICK_WHEEL_SIZE],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.prio_tbl = PrioTable::new();
        self.rdy_list = [ReadyList::new(); CFG_PRIO_MAX];
        self.tick_wheel = [None; CFG_TICK_WHEEL_SIZE];
    }

    /// Get mutable reference to priority table
    #[inline(always)]
    pub fn prio_table(&mut self) -> &mut PrioTable {
        &mut self.prio_tbl
    }

    /// Get reference to ready list
    #[inline(always)]
    pub fn rdy_list(&mut self, prio: OsPrio) -> &mut ReadyList {
        &mut self.rdy_list[prio as usize]
    }

    /// Get the tick wheel slot
    #[inline(always)]
    fn tick_wheel_slot(tick: u32) -> usize {
        (tick as usize) % CFG_TICK_WHEEL_SIZE
    }

    /// Get head of tick wheel at current slot
    #[inline(always)]
    pub fn tick_wheel_head(&self, slot: usize) -> Option<NonNull<OsTcb>> {
        self.tick_wheel[slot]
    }

    /// Add task to tick wheel
    pub unsafe fn tick_wheel_insert(&mut self, tcb: NonNull<OsTcb>, expiry_tick: u32) {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };
        let slot = Self::tick_wheel_slot(expiry_tick);
        
        tcb_ref.tick_wheel_slot = slot as u8;
        
        // Insert at head of slot
        tcb_ref.tick_next_ptr = self.tick_wheel[slot];
        tcb_ref.tick_prev_ptr = None;
        
        if let Some(mut old_head) = self.tick_wheel[slot] {
            unsafe { old_head.as_mut().tick_prev_ptr = Some(tcb) };
        }
        
        self.tick_wheel[slot] = Some(tcb);
    }

    /// Remove task from tick wheel
    pub unsafe fn tick_wheel_remove(&mut self, tcb: NonNull<OsTcb>) {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };
        let slot = tcb_ref.tick_wheel_slot as usize;
        
        if let Some(mut prev) = tcb_ref.tick_prev_ptr {
            unsafe { prev.as_mut().tick_next_ptr = tcb_ref.tick_next_ptr };
        } else {
            self.tick_wheel[slot] = tcb_ref.tick_next_ptr;
        }
        
        if let Some(mut next) = tcb_ref.tick_next_ptr {
            unsafe { next.as_mut().tick_prev_ptr = tcb_ref.tick_prev_ptr };
        }
        
        tcb_ref.tick_next_ptr = None;
        tcb_ref.tick_prev_ptr = None;
    }
}

/// Global scheduler state instance  
pub(crate) static SCHED: CsCell<SchedState> = CsCell::new(SchedState::new());

/// IDLE task TCB
static mut IDLE_TCB: OsTcb = OsTcb::new();

/// IDLE task stack
static mut IDLE_STK: [crate::types::OsStkElement; 128] = [0; 128];

// ============ CPU/Context Switch State ============

/// CPU context switch state
#[repr(C)]
pub struct CpuState {
    /// Current running task's TCB pointer
    pub tcb_cur: *mut OsTcb,
    /// Highest priority ready task's TCB pointer
    pub tcb_high_rdy: *mut OsTcb,
    /// Current running task's priority
    pub prio_cur: OsPrio,
    /// Highest ready priority
    pub prio_high_rdy: OsPrio,
    /// Exception stack base
    pub except_stk_base: u32,
}

impl CpuState {
    pub const fn new() -> Self {
        Self {
            tcb_cur: core::ptr::null_mut(),
            tcb_high_rdy: core::ptr::null_mut(),
            prio_cur: 0,
            prio_high_rdy: 0,
            except_stk_base: 0,
        }
    }
    
    pub fn reset(&mut self) {
        self.tcb_cur = core::ptr::null_mut();
        self.tcb_high_rdy = core::ptr::null_mut();
        self.prio_cur = 0;
        self.prio_high_rdy = 0;
    }

    // ============ TCB Accessor Methods ============

    /// Get current TCB pointer
    #[inline(always)]
    pub unsafe fn tcb_cur_ptr(&self) -> Option<NonNull<OsTcb>> {
        NonNull::new(self.tcb_cur)
    }

    /// Set current TCB pointer
    #[inline(always)]
    pub unsafe fn set_tcb_cur(&mut self, tcb: Option<NonNull<OsTcb>>) {
        self.tcb_cur = tcb.map_or(core::ptr::null_mut(), |p| p.as_ptr());
    }

    /// Get high ready TCB pointer
    #[inline(always)]
    pub unsafe fn tcb_high_rdy_ptr(&self) -> Option<NonNull<OsTcb>> {
        NonNull::new(self.tcb_high_rdy)
    }

    /// Set high ready TCB pointer
    #[inline(always)]
    pub unsafe fn set_tcb_high_rdy(&mut self, tcb: Option<NonNull<OsTcb>>) {
        self.tcb_high_rdy = tcb.map_or(core::ptr::null_mut(), |p| p.as_ptr());
    }

    // ============ Priority Accessor Methods ============

    /// Get current priority
    #[inline(always)]
    pub unsafe fn get_prio_cur(&self) -> OsPrio {
        self.prio_cur
    }

    /// Set current priority
    #[inline(always)]
    pub unsafe fn set_prio_cur(&mut self, prio: OsPrio) {
        self.prio_cur = prio;
    }

    /// Get high ready priority
    #[inline(always)]
    pub unsafe fn get_prio_high_rdy(&self) -> OsPrio {
        self.prio_high_rdy
    }

    /// Set high ready priority
    #[inline(always)]
    pub unsafe fn set_prio_high_rdy(&mut self, prio: OsPrio) {
        self.prio_high_rdy = prio;
    }
}

/// Global CPU state instance
#[no_mangle]
#[used]
pub static mut CPU_STATE: CpuState = CpuState::new();

/// BASEPRI boundary
#[no_mangle]
pub static OS_KA_BASEPRI_Boundary: u32 = 0;

// ============ Initialization ============

/// Internal IDLE task function
fn os_idle_task(_: *mut ()) -> ! {
    loop {
        cortex_m::asm::nop();
    }
}

/// Reset global kernel state
unsafe fn os_reset_globals() {
    KERNEL.reset();
    
    unsafe {
        CPU_STATE.tcb_cur = core::ptr::null_mut();
        CPU_STATE.tcb_high_rdy = core::ptr::null_mut();
        CPU_STATE.prio_cur = 0;
        CPU_STATE.prio_high_rdy = 0;
    }
    
    unsafe {
        SCHED.get_unchecked().reset();
    }
}

// ============ Public API ============

/// Initialize the RTOS kernel
///
/// This must be called before any other OS function.
/// It initializes the priority table, ready lists, and internal state.
/// IDLE task is automatically created.
///
/// # Returns
/// * `Ok(())` - Initialization successful
/// * `Err(OsError::OsRunning)` - OS is already running
#[allow(static_mut_refs)]
pub fn os_init() -> OsResult<()> {
    unsafe { os_reset_globals(); }
    
    if KERNEL.is_running() {
        return Err(OsError::OsRunning);
    }
    
    critical_section(|cs| {
        let sched = SCHED.get(cs);
        
        // Initialize priority table
        sched.prio_tbl.init();

        // Initialize ready lists
        for list in sched.rdy_list.iter_mut() {
            list.init();
        }

        // Create IDLE task
        unsafe {
            crate::task::os_task_create_internal(
                &raw mut IDLE_TCB,
                "Idle",
                os_idle_task,
                core::ptr::null_mut(),
                crate::config::CFG_PRIO_IDLE,
                IDLE_STK.as_mut_ptr(),
                IDLE_STK.len(),
                0,
                0,
            ).expect("IDLE task creation failed");
        }

        KERNEL.set_initialized(true);
    });

    Ok(())
}

/// Start multitasking
///
/// This function starts the highest priority ready task. It never returns.
/// Before calling this, at least one application task must be created.
///
/// # Returns
/// This function does not return under normal operation.
/// * `Err(OsError::OsNotInit)` - OS not initialized
/// * `Err(OsError::OsRunning)` - OS is already running
/// * `Err(OsError::OsNoAppTask)` - No application task created
pub fn os_start() -> OsResult<()> {
    if !KERNEL.is_initialized() {
        return Err(OsError::OsNotInit);
    }
    
    if KERNEL.is_running() {
        return Err(OsError::OsRunning);
    }
    
    critical_section(|cs| {
        let sched = SCHED.get(cs);
        
        let high_prio = sched.prio_tbl.get_highest();

        unsafe {
            CPU_STATE.prio_high_rdy = high_prio;
            CPU_STATE.prio_cur = high_prio;

            if let Some(head) = sched.rdy_list[high_prio as usize].head() {
                CPU_STATE.tcb_high_rdy = head.as_ptr();
                CPU_STATE.tcb_cur = head.as_ptr();
            } else {
                return;
            }
        }

        KERNEL.set_running(true);
    });

    // Initialize SysTick
    crate::port::os_cpu_systick_init(16_000_000 / crate::config::CFG_TICK_RATE_HZ);

    unsafe { 
        CPU_STATE.tcb_cur = CPU_STATE.tcb_high_rdy;
        crate::port::os_start_high_rdy() 
    };
    
    Ok(())
}

/// Exit ISR
pub fn os_int_exit() {
    if !KERNEL.is_running() {
        return;
    }

    let _cs = CriticalSection::enter();

    let old_nesting = KERNEL.int_nesting();
    if old_nesting == 0 {
        return;
    }

    let new_nesting = KERNEL.int_nesting_dec();

    if new_nesting == 0 && KERNEL.sched_lock_nesting() == 0 {
        // Check whether need to switch tasks
        let high_prio = unsafe { SCHED.get_unchecked().prio_tbl.get_highest() };
        
        unsafe {
            if high_prio < CPU_STATE.prio_cur {
                CPU_STATE.prio_high_rdy = high_prio;
                
                if let Some(head) = SCHED.get_unchecked().rdy_list[high_prio as usize].head() {
                    CPU_STATE.tcb_high_rdy = head.as_ptr();
                    crate::port::os_int_ctx_sw();
                }
            }
        }
    }
}

/// Lock the scheduler
pub fn os_sched_lock() -> OsResult<()> {
    if !KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }

    if KERNEL.int_nesting() > 0 {
        return Err(OsError::SchedLockIsr);
    }

    critical_section(|_cs| {
        KERNEL.try_sched_lock()
    })
}

/// Unlock the scheduler
pub fn os_sched_unlock() -> OsResult<()> {
    if !KERNEL.is_running() {
        return Err(OsError::OsNotRunning);
    }

    if KERNEL.int_nesting() > 0 {
        return Err(OsError::SchedUnlockIsr);
    }

    critical_section(|_cs| {
        let remaining = KERNEL.try_sched_unlock()?;
        if remaining == 0 {
            crate::sched::os_sched();
        }
        Ok(())
    })
}

// ============ Internal accessors for other modules ============

/// Get mutable reference to priority table
#[inline(always)]
pub(crate) unsafe fn prio_table() -> &'static mut PrioTable {
    unsafe { &mut SCHED.get_unchecked().prio_tbl }
}

/// Get reference to ready list for a priority
#[inline(always)]
pub(crate) unsafe fn rdy_list(prio: OsPrio) -> &'static mut ReadyList {
    unsafe { &mut SCHED.get_unchecked().rdy_list[prio as usize] }
}

/// Get current TCB pointer as Option<NonNull>
#[inline]
#[allow(static_mut_refs)]
pub(crate) unsafe fn tcb_cur_ptr() -> Option<NonNull<OsTcb>> {
    unsafe { CPU_STATE.tcb_cur_ptr() }
}

/// Set current TCB pointer
#[inline]
#[allow(dead_code, static_mut_refs)]
pub(crate) unsafe fn set_tcb_cur_ptr(tcb: Option<NonNull<OsTcb>>) {
    unsafe { CPU_STATE.set_tcb_cur(tcb) }
}

/// Get high ready TCB pointer as Option<NonNull>
#[inline]
#[allow(dead_code, static_mut_refs)]
pub(crate) unsafe fn tcb_high_rdy_ptr() -> Option<NonNull<OsTcb>> {
    unsafe { CPU_STATE.tcb_high_rdy_ptr() }
}

/// Set high ready TCB pointer
#[inline]
#[allow(static_mut_refs)]
pub(crate) unsafe fn set_tcb_high_rdy_ptr(tcb: Option<NonNull<OsTcb>>) {
    unsafe { CPU_STATE.set_tcb_high_rdy(tcb) }
}

/// Get current priority
#[inline]
#[allow(dead_code, static_mut_refs)]
pub(crate) unsafe fn prio_cur() -> OsPrio {
    unsafe { CPU_STATE.get_prio_cur() }
}

/// Set current priority
#[inline]
#[allow(dead_code, static_mut_refs)]
pub(crate) unsafe fn set_prio_cur(prio: OsPrio) {
    unsafe { CPU_STATE.set_prio_cur(prio) }
}

/// Get high ready priority
#[inline]
#[allow(dead_code, static_mut_refs)]
pub(crate) unsafe fn prio_high_rdy() -> OsPrio {
    unsafe { CPU_STATE.get_prio_high_rdy() }
}

/// Set high ready priority
#[inline]
#[allow(static_mut_refs)]
pub(crate) unsafe fn set_prio_high_rdy(prio: OsPrio) {
    unsafe { CPU_STATE.set_prio_high_rdy(prio) }
}

// ============ Tick Wheel Management ============

/// Add task to tick wheel based on expiry tick
pub(crate) unsafe fn tick_wheel_insert(tcb: NonNull<OsTcb>, expiry_tick: u32) {
    unsafe {
        SCHED.get_unchecked().tick_wheel_insert(tcb, expiry_tick);
    }
}

/// Remove task from tick wheel
pub(crate) unsafe fn tick_wheel_remove(tcb: NonNull<OsTcb>) {
    unsafe {
        SCHED.get_unchecked().tick_wheel_remove(tcb);
    }
}

/// Get head of tick wheel at specified slot
#[inline]
pub(crate) unsafe fn tick_wheel_head(slot: usize) -> Option<NonNull<OsTcb>> {
    unsafe { SCHED.get_unchecked().tick_wheel_head(slot) }
}

