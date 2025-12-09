//! Unit tests for core RTOS modules
//!
//! These tests run on the host (not embedded target) to verify
//! the core algorithms work correctly.

#[cfg(test)]
mod prio_tests {
    use ucosiii::prio::PrioTable;
    use ucosiii::config::CFG_PRIO_MAX;

    #[test]
    fn test_empty_table() {
        let table = PrioTable::new();
        assert!(table.is_empty());
        assert_eq!(table.get_highest(), (CFG_PRIO_MAX - 1) as u8);
    }

    #[test]
    fn test_single_priority() {
        let mut table = PrioTable::new();
        
        table.insert(5);
        assert!(!table.is_empty());
        assert!(table.is_set(5));
        assert!(!table.is_set(4));
        assert_eq!(table.get_highest(), 5);
        
        table.remove(5);
        assert!(table.is_empty());
    }

    #[test]
    fn test_multiple_priorities() {
        let mut table = PrioTable::new();
        
        // Insert in random order
        table.insert(20);
        table.insert(5);
        table.insert(10);
        table.insert(0);
        table.insert(15);
        
        // Highest (lowest number) should be 0
        assert_eq!(table.get_highest(), 0);
        
        // Remove in order
        table.remove(0);
        assert_eq!(table.get_highest(), 5);
        
        table.remove(5);
        assert_eq!(table.get_highest(), 10);
        
        table.remove(10);
        assert_eq!(table.get_highest(), 15);
        
        table.remove(15);
        assert_eq!(table.get_highest(), 20);
        
        table.remove(20);
        assert!(table.is_empty());
    }

    #[test]
    fn test_boundary_priorities() {
        let mut table = PrioTable::new();
        
        // Test at word boundaries (31, 32, 33)
        table.insert(31);
        assert_eq!(table.get_highest(), 31);
        
        table.insert(32);
        assert_eq!(table.get_highest(), 31);
        
        table.remove(31);
        assert_eq!(table.get_highest(), 32);
        
        table.insert(0);
        assert_eq!(table.get_highest(), 0);
        
        table.insert(63);
        table.remove(0);
        table.remove(32);
        assert_eq!(table.get_highest(), 63);
    }

    #[test]
    fn test_all_priorities() {
        let mut table = PrioTable::new();
        
        // Insert all priorities
        for i in 0..CFG_PRIO_MAX {
            table.insert(i as u8);
        }
        
        // Highest should be 0
        assert_eq!(table.get_highest(), 0);
        
        // Remove from highest to lowest
        for i in 0..CFG_PRIO_MAX {
            assert_eq!(table.get_highest(), i as u8);
            table.remove(i as u8);
        }
        
        assert!(table.is_empty());
    }

    #[test]
    fn test_duplicate_insert_remove() {
        let mut table = PrioTable::new();
        
        // Insert same priority twice
        table.insert(10);
        table.insert(10);
        assert_eq!(table.get_highest(), 10);
        
        // First remove clears the bit
        table.remove(10);
        // Table should be empty now (bit is cleared)
        // Note: This tests that we don't track count per priority
    }
}

#[cfg(test)]
mod error_tests {
    use ucosiii::error::OsError;

    #[test]
    fn test_error_variants() {
        assert!(OsError::None.is_ok());
        assert!(!OsError::None.is_err());
        
        assert!(!OsError::Timeout.is_ok());
        assert!(OsError::Timeout.is_err());
        
        assert_eq!(OsError::None, OsError::None);
        assert_ne!(OsError::None, OsError::Timeout);
    }

    #[test]
    fn test_error_debug() {
        // Ensure errors can be formatted for debugging
        let err = OsError::PendIsr;
        let _ = format!("{:?}", err);
    }
}

#[cfg(test)]
mod types_tests {
    use ucosiii::types::*;

    #[test]
    fn test_task_state_enum() {
        let state = OsTaskState::Ready;
        assert_eq!(state, OsTaskState::Ready);
        assert_ne!(state, OsTaskState::Delayed);
    }

    #[test]
    fn test_pend_status_enum() {
        let status = OsPendStatus::Ok;
        assert_eq!(status, OsPendStatus::Ok);
        assert_ne!(status, OsPendStatus::Timeout);
    }

    #[test]
    fn test_option_flags() {
        use ucosiii::types::opt::*;
        
        assert_eq!(NONE, 0);
        assert_eq!(PEND_NON_BLOCKING, 0x8000);
        assert_eq!(POST_NO_SCHED, 0x8000);
        
        // Test combining flags
        let combined = POST_FIFO | POST_NO_SCHED;
        assert_eq!(combined & POST_NO_SCHED, POST_NO_SCHED);
    }
}

#[cfg(test)]
mod config_tests {
    use ucosiii::config::*;

    #[test]
    fn test_config_values() {
        assert!(CFG_PRIO_MAX >= 8, "Need at least 8 priority levels");
        assert!(CFG_PRIO_MAX <= 256, "Too many priority levels");
        
        assert!(CFG_STK_SIZE_MIN >= 32, "Stack too small");
        
        assert!(CFG_TICK_RATE_HZ >= 10, "Tick rate too slow");
        assert!(CFG_TICK_RATE_HZ <= 10000, "Tick rate too fast");
        
        // Idle priority should be lowest
        assert_eq!(CFG_PRIO_IDLE, (CFG_PRIO_MAX - 1) as u8);
    }
}
