//! Compile-time configuration for μC/OS-III
//!
//! These constants control the behavior and resource limits of the RTOS.

/// Maximum number of priority levels
pub const CFG_PRIO_MAX: usize = 64;

/// System tick rate in Hz
pub const CFG_TICK_RATE_HZ: u32 = 1000;

/// Default time quanta for round-robin scheduling
pub const CFG_TIME_QUANTA_DEFAULT: u32 = 10;

/// Minimum task stack size
pub const CFG_STK_SIZE_MIN: usize = 64;

/// Number of entries in tick wheel
pub const CFG_TICK_WHEEL_SIZE: usize = 16;

/// Maximum message queue size
pub const CFG_MSG_POOL_SIZE: usize = 32;

/// Enable round-robin scheduling for same-priority tasks
pub const CFG_SCHED_ROUND_ROBIN_EN: bool = true;

/// Idle task priority
pub const CFG_PRIO_IDLE: u8 = (CFG_PRIO_MAX - 1) as u8;
