//! Logging macros for uCOS-III
//!
//! Provides logging macros that work with or without the debug feature.

/// Debug message
#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { defmt::debug!($($arg)*) };
}

/// Info message
#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { defmt::info!($($arg)*) };
}

/// Error message
#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { defmt::error!($($arg)*) };
}

/// Trace message
#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { defmt::trace!($($arg)*) };
}

/// Warning message
#[cfg(feature = "defmt")]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { defmt::warn!($($arg)*) };
}

// No-op versions when debug is disabled
#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! debug { ($($arg:tt)*) => {}; }
#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! info { ($($arg:tt)*) => {}; }
#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! error { ($($arg:tt)*) => {}; }
#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! trace { ($($arg:tt)*) => {}; }
#[cfg(not(feature = "defmt"))]
#[macro_export]
macro_rules! warn { ($($arg:tt)*) => {}; }
