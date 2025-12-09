//! Priority Inversion Demo - mutex priority inheritance
//!
//! Three tasks: High(5), Med(10), Low(15)
//! Low holds mutex -> High waits -> Low boosted to prio 5

#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use core::sync::atomic::{AtomicU32, Ordering};

use cortex_m_rt::entry;
use defmt::info;
use ucosiii::task::OsTcb;
use ucosiii::time::os_time_dly;
use ucosiii::types::OsStkElement;
use ucosiii::mutex::Mutex;
use ucosiii::os_task_create;

static HIGH_RUNS: AtomicU32 = AtomicU32::new(0);
static LOW_RUNS: AtomicU32 = AtomicU32::new(0);

static MTX: Mutex = Mutex::new();

static mut HIGH_STK: [OsStkElement; 256] = [0; 256];
static mut HIGH_TCB: OsTcb = OsTcb::new();
static mut MED_STK: [OsStkElement; 256] = [0; 256];
static mut MED_TCB: OsTcb = OsTcb::new();
static mut LOW_STK: [OsStkElement; 256] = [0; 256];
static mut LOW_TCB: OsTcb = OsTcb::new();

/// High priority task (prio=5)
fn high_task_fn(_arg: *mut ()) -> ! {
    let _ = os_time_dly(50);
    
    loop {
        let n = HIGH_RUNS.fetch_add(1, Ordering::Relaxed) + 1;
        
        let _ = MTX.lock(0, 0);
        info!("[HIGH] acquired #{}", n);
        
        for _ in 0..1_000 { cortex_m::asm::nop(); }
        
        let _ = MTX.unlock(0);
        let _ = os_time_dly(100);
    }
}

/// Medium priority task (prio=10) - CPU bound
fn med_task_fn(_arg: *mut ()) -> ! {
    loop {
        for _ in 0..50_000 { cortex_m::asm::nop(); }
        let _ = os_time_dly(10);
    }
}

/// Low priority task (prio=15) - holds mutex long
fn low_task_fn(_arg: *mut ()) -> ! {
    loop {
        let n = LOW_RUNS.fetch_add(1, Ordering::Relaxed) + 1;
        
        let _ = MTX.lock(0, 0);
        info!("[LOW] holding #{}", n);
        
        for _ in 0..100_000 { cortex_m::asm::nop(); }
        
        let _ = MTX.unlock(0);
        let _ = os_time_dly(200);
    }
}

#[entry]
fn main() -> ! {
    info!("Priority Inversion Demo: H(5) M(10) L(15)");
    
    ucosiii::os_init().expect("OS init failed");
    MTX.create("Mtx").unwrap();

    unsafe {
        os_task_create(&mut LOW_TCB, &mut LOW_STK, "L", low_task_fn, 15).unwrap();
        os_task_create(&mut MED_TCB, &mut MED_STK, "M", med_task_fn, 10).unwrap();
        os_task_create(&mut HIGH_TCB, &mut HIGH_STK, "H", high_task_fn, 5).unwrap();
    }

    info!("Starting...");
    ucosiii::os_start().expect("OS start failed");

    loop { cortex_m::asm::wfi(); }
}
