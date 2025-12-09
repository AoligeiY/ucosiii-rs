//! Producer-Consumer example with semaphores

#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use core::sync::atomic::{AtomicU32, Ordering};

use cortex_m_rt::entry;
use defmt::info;
use ucosiii::task::OsTcb;
use ucosiii::time::os_time_dly;
use ucosiii::types::OsStkElement;
use ucosiii::sem::Semaphore;
use ucosiii::os_task_create;

static PRODUCED: AtomicU32 = AtomicU32::new(0);
static CONSUMED: AtomicU32 = AtomicU32::new(0);

static SEM: Semaphore = Semaphore::new(0);

static mut PRODUCER_STK: [OsStkElement; 256] = [0; 256];
static mut PRODUCER_TCB: OsTcb = OsTcb::new();
static mut CONSUMER_STK: [OsStkElement; 256] = [0; 256];
static mut CONSUMER_TCB: OsTcb = OsTcb::new();

fn producer_task(_arg: *mut ()) -> ! {
    loop {
        let n = PRODUCED.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = SEM.signal(0);
        info!("[P] produced #{}", n);
        let _ = os_time_dly(200);
    }
}

fn consumer_task(_arg: *mut ()) -> ! {
    loop {
        let _ = SEM.wait(0, 0);
        let n = CONSUMED.fetch_add(1, Ordering::Relaxed) + 1;
        info!("[C] consumed #{}", n);
        for _ in 0..10_000 { cortex_m::asm::nop(); }
    }
}

#[entry]
fn main() -> ! {
    info!("Producer-Consumer Demo");
    
    ucosiii::os_init().expect("OS init failed");
    SEM.create(0, "Sem").unwrap();

    unsafe {
        os_task_create(&mut PRODUCER_TCB, &mut PRODUCER_STK, "P", producer_task, 15).unwrap();
        os_task_create(&mut CONSUMER_TCB, &mut CONSUMER_STK, "C", consumer_task, 10).unwrap();
    }

    info!("Starting...");
    ucosiii::os_start().expect("OS start failed");

    loop { cortex_m::asm::wfi(); }
}
