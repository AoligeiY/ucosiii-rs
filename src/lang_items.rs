//! Language items and default exception handlers

// When defmt feature is enabled on ARM targets, use defmt_rtt and panic_probe
#[cfg(all(feature = "defmt", target_arch = "arm"))]
use defmt_rtt as _;

#[cfg(all(feature = "defmt", target_arch = "arm"))]
use panic_probe as _;

// Defmt panic handler
#[cfg(all(feature = "defmt", target_arch = "arm"))]
#[defmt::panic_handler]
fn defmt_panic() -> ! {
    cortex_m::asm::udf()
}

// Panic handler when defmt is disabled
#[cfg(all(not(feature = "defmt"), target_arch = "arm"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { cortex_m::asm::udf(); }
}

// Default HardFault handler
#[cfg(target_arch = "arm")]
#[cortex_m_rt::exception]
unsafe fn HardFault(_ef: &cortex_m_rt::ExceptionFrame) -> ! {
    loop { cortex_m::asm::udf(); }
}

// Defmt timestamp
#[cfg(all(feature = "defmt", target_arch = "arm"))]
defmt::timestamp!("{=u32}", crate::core::kernel::KERNEL.tick_get());
