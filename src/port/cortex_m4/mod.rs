//! Cortex-M4 port implementation
//!
//! Provides context switching via PendSV exception handler.

#![allow(named_asm_labels)]

use core::arch::{asm, naked_asm};

use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::peripheral::syst::SystClkSource;
use crate::task::OsTaskFn;
use crate::types::{OsOpt, OsStkElement};

/// Interrupt stack for MSP
#[no_mangle]
static mut INTERRUPT_STACK: [u64; 256] = [0xDEADBEEF_DEADBEEF; 256];

/// Initialize SysTick timer for system tick generation
///
/// # Arguments
/// * `cnts` - Reload value
///
/// # Example
/// For 16MHz clock with 1000Hz tick rate: cnts = 16_000_000 / 1000 = 16_000
pub fn os_cpu_systick_init(cnts: u32) {
    let mut p = unsafe { cortex_m::Peripherals::steal() };
    
    // Configure SysTick timer
    p.SYST.set_reload(cnts - 1);
    p.SYST.clear_current();
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();
}

/// Start the highest priority ready task
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn os_start_high_rdy() {
    unsafe {
        let mut scb = cortex_m::Peripherals::steal().SCB;
        
        // Set PendSV and SysTick priority to lowest
        scb.set_priority(SystemHandler::PendSV, 0xF0);
        scb.set_priority(SystemHandler::SysTick, 0xF0);

        // Switch MSP to dedicated interrupt stack
        let msp_top = &INTERRUPT_STACK as *const _ as u32 + core::mem::size_of_val(&INTERRUPT_STACK) as u32;
        
        asm!("msr msp, {0}", in(reg) msp_top,);
        asm!("msr psp, {0}", in(reg) 0);

        crate::kernel::CPU_STATE.tcb_cur = core::ptr::null_mut();

        cortex_m::interrupt::enable();
        cortex_m::peripheral::SCB::set_pendsv();
    }
}

/// Trigger context switch from task level
#[inline(always)]
pub fn os_ctx_sw() {
    cortex_m::peripheral::SCB::set_pendsv();
}

/// Trigger context switch from interrupt level
#[inline(always)]
pub fn os_int_ctx_sw() {
    cortex_m::peripheral::SCB::set_pendsv();
}

/// Context structure stored on stack
#[repr(C, align(4))]
struct UcStk {
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    exc_return: u32,  // LR value for exception return
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
}
const CONTEXT_STACK_SIZE: usize = 17;

/// Initialize task stack
pub unsafe fn os_task_stk_init(
    task_fn: OsTaskFn,
    arg: *mut (),
    stk_base: *mut OsStkElement,
    stk_size: usize,
    _opt: OsOpt,
) -> *mut OsStkElement {
    unsafe {
        let stk_top = stk_base.add(stk_size);
        let stk_aligned = ((stk_top as usize) & !7) as *mut u32;
        
        let frame_ptr = stk_aligned.sub(CONTEXT_STACK_SIZE) as *mut UcStk;
        
        (*frame_ptr) = UcStk {
            r4: 0x04040404,
            r5: 0x05050505,
            r6: 0x06060606,
            r7: 0x07070707,
            r8: 0x08080808,
            r9: 0x09090909,
            r10: 0x10101010,
            r11: 0x11111111,
            exc_return: 0xFFFF_FFFD,
            r0: arg as u32,
            r1: 0,
            r2: 0,
            r3: 0,
            r12: 0,
            lr: os_task_return as *const () as u32,
            pc: (task_fn as usize as u32) | 1,
            xpsr: 0x0100_0000,
        };
        
        // Return pointer 4 bytes before frame to match PendSV's "add r0, r0, #4"
        (frame_ptr as *mut u32).sub(1) as *mut OsStkElement
    }
}

/// Helper function called from PendSV to perform TCB switching
/// Returns new task's stack pointer
#[inline(never)]
#[no_mangle]
unsafe extern "C" fn pendsv_switch_context(cur_sp: *mut u32) -> *mut u32 {
    unsafe {
        let cur_tcb_ptr = crate::kernel::CPU_STATE.tcb_cur;
        
        if !cur_tcb_ptr.is_null() {
            (*cur_tcb_ptr).stk_ptr = cur_sp;
        }
        
        crate::kernel::CPU_STATE.tcb_cur = crate::kernel::CPU_STATE.tcb_high_rdy;
        crate::kernel::CPU_STATE.prio_cur = crate::kernel::CPU_STATE.prio_high_rdy;
        
        let new_tcb_ptr = crate::kernel::CPU_STATE.tcb_cur;
        
        if new_tcb_ptr.is_null() {
            core::ptr::null_mut()
        } else {
            (*new_tcb_ptr).stk_ptr
        }
    }
}

/// PendSV exception handler - performs full context switch
///
/// 1. Save R4-R11, LR to current task's PSP (skip if first task)
/// 2. Call switch_context to swap TCB pointers
/// 3. Restore R4-R11, LR from new task's stack
/// 4. Exception return
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() {
    use crate::kernel::CPU_STATE;

    naked_asm!(
        "cpsid i",
        "dsb",
        "isb",
        
        "mrs r0, psp",
        
        "ldr r1, ={cpu_state}",
        "ldr r1, [r1]",
        "cbz r1, 1f",
        
        "stmdb r0!, {{r4-r11, lr}}",
        
        "sub r0, r0, #4",
        
        "1:",
        "bl pendsv_switch_context",
        
        "cbz r0, 2f",
        "add r0, r0, #4",
        "ldmia r0!, {{r4-r11, lr}}",
        
        "msr psp, r0",
        
        "2:",
        "cpsie i",
        "dsb",
        "isb",
        
        "bx lr",
        
        cpu_state = sym CPU_STATE,
    );
}

/// Task switch hook
#[no_mangle]
fn os_task_sw_hook() {}

/// Task return handler
#[no_mangle]
fn os_task_return() -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}
