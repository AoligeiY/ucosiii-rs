//! Blink Example - LED blinking using RTOS on STM32F401

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use ucosiii::task::OsTcb;
use ucosiii::time::os_time_dly;
use ucosiii::types::OsStkElement;
use ucosiii::os_task_create;

#[cfg(feature = "pac")]
use stm32_metapac as pac;

// ============ Task Storage ============

static mut BLINK_STK: [OsStkElement; 512] = [0; 512];
static mut BLINK_TCB: OsTcb = OsTcb::new();

// ============ LED Control ============

#[cfg(feature = "pac")]
fn led_init() {
    pac::RCC.ahb1enr().modify(|w| w.set_gpioaen(true));
    pac::GPIOA.moder().modify(|w| w.set_moder(5, pac::gpio::vals::Moder::OUTPUT));
    pac::GPIOA.otyper().modify(|w| w.set_ot(5, pac::gpio::vals::Ot::PUSHPULL));
}

#[cfg(feature = "pac")]
fn led_on() { pac::GPIOA.bsrr().write(|w| w.set_bs(5, true)); }

#[cfg(feature = "pac")]
fn led_off() { pac::GPIOA.bsrr().write(|w| w.set_br(5, true)); }

#[cfg(not(feature = "pac"))]
fn led_init() {}
#[cfg(not(feature = "pac"))]
fn led_on() {}
#[cfg(not(feature = "pac"))]
fn led_off() {}

// ============ Task ============

fn blink_task(_: *mut ()) -> ! {
    ucosiii::info!("Blink task started");
    loop {
        led_on();
        ucosiii::info!("LED ON");
        let _ = os_time_dly(500);
        
        led_off();
        ucosiii::info!("LED OFF");
        let _ = os_time_dly(500);
    }
}

fn test_task(_: *mut ()) -> ! {
    ucosiii::info!("test task started");
    loop {
        ucosiii::info!("Test Task");
        let _ = os_time_dly(1000);
    }
}
static mut TEST_STK: [OsStkElement; 512] = [0; 512];
static mut TEST_TCB: OsTcb = OsTcb::new();

// ============ Main ============

#[entry]
fn main() -> ! {   
    led_init();
    
    ucosiii::os_init().expect("OS init failed");
    
    os_task_create(
        unsafe { &mut BLINK_TCB },
        unsafe { &mut BLINK_STK },
        "Blink",
        blink_task,
        5,
    ).expect("Blink task failed");
    
    os_task_create(
        unsafe { &mut TEST_TCB },
        unsafe { &mut TEST_STK },
        "test",
        test_task,
        5,
    ).expect("Testtask failed");
    
    ucosiii::info!("Starting RTOS");
    ucosiii::os_start().expect("OS start failed");
    
    loop { cortex_m::asm::nop(); }
}
