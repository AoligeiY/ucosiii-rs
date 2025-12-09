//! Ready list - doubly linked list of TCBs at a given priority
//!
//! Each priority level has its own ready list. Tasks are added to the
//! tail (FIFO for round-robin) and scheduled from the head.

use core::ptr::NonNull;

use crate::task::OsTcb;

/// Ready list for a single priority level
///
/// Doubly-linked list of tasks ready to run at this priority.
/// Tasks are inserted at the tail for FIFO ordering and
/// scheduled from the head.
#[derive(Debug)]
pub struct ReadyList {
    head: Option<NonNull<OsTcb>>,
    tail: Option<NonNull<OsTcb>>,
    #[cfg(feature = "defmt")]
    count: usize,
}

impl ReadyList {
    /// Create a new empty ready list
    pub const fn new() -> Self {
        ReadyList {
            head: None,
            tail: None,
            #[cfg(feature = "defmt")]
            count: 0,
        }
    }

    /// Initialize/reset the ready list
    pub fn init(&mut self) {
        self.head = None;
        self.tail = None;
        #[cfg(feature = "defmt")]
        {
            self.count = 0;
        }
    }

    /// Get head of list (first to be scheduled)
    #[inline]
    pub fn head(&self) -> Option<NonNull<OsTcb>> {
        self.head
    }

    /// Get tail of list
    #[inline]
    pub fn tail(&self) -> Option<NonNull<OsTcb>> {
        self.tail
    }

    /// Check if list is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Get number of entries (only in debug mode)
    #[cfg(feature = "defmt")]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Insert TCB at the tail of the list (FIFO order)
    ///
    /// # Safety
    /// Caller must ensure tcb is valid and not already in any list.
    pub fn insert_tail(&mut self, tcb: NonNull<OsTcb>) {
        // SAFETY: We have exclusive access via critical section
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };

        tcb_ref.next_ptr = None;
        tcb_ref.prev_ptr = self.tail;

        match self.tail {
            Some(tail) => {
                // List not empty - link from current tail
                unsafe { (*tail.as_ptr()).next_ptr = Some(tcb) };
            }
            None => {
                // List is empty - this becomes head
                self.head = Some(tcb);
            }
        }

        self.tail = Some(tcb);

        #[cfg(feature = "defmt")]
        {
            self.count += 1;
        }
    }

    /// Insert TCB at the head of the list (LIFO order)
    ///
    /// Used for priority boosting when a task should run next.
    ///
    /// # Safety
    /// Caller must ensure tcb is valid and not already in any list.
    pub fn insert_head(&mut self, tcb: NonNull<OsTcb>) {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };

        tcb_ref.prev_ptr = None;
        tcb_ref.next_ptr = self.head;

        match self.head {
            Some(head) => {
                // List not empty - link to current head
                unsafe { (*head.as_ptr()).prev_ptr = Some(tcb) };
            }
            None => {
                // List is empty - this becomes tail
                self.tail = Some(tcb);
            }
        }

        self.head = Some(tcb);

        #[cfg(feature = "defmt")]
        {
            self.count += 1;
        }
    }

    /// Remove a TCB from the list
    ///
    /// # Safety
    /// Caller must ensure tcb is valid and is in this list.
    pub fn remove(&mut self, tcb: NonNull<OsTcb>) {
        let tcb_ref = unsafe { &mut *tcb.as_ptr() };

        // Update previous node's next pointer
        match tcb_ref.prev_ptr {
            Some(prev) => {
                unsafe { (*prev.as_ptr()).next_ptr = tcb_ref.next_ptr };
            }
            None => {
                // This was the head
                self.head = tcb_ref.next_ptr;
            }
        }

        // Update next node's prev pointer
        match tcb_ref.next_ptr {
            Some(next) => {
                unsafe { (*next.as_ptr()).prev_ptr = tcb_ref.prev_ptr };
            }
            None => {
                // This was the tail
                self.tail = tcb_ref.prev_ptr;
            }
        }

        // Clear TCB's list pointers
        tcb_ref.prev_ptr = None;
        tcb_ref.next_ptr = None;

        #[cfg(feature = "defmt")]
        {
            self.count = self.count.saturating_sub(1);
        }
    }
}

impl Default for ReadyList {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: ReadyList is only modified within critical sections
unsafe impl Send for ReadyList {}
unsafe impl Sync for ReadyList {}

impl Copy for ReadyList {}

impl Clone for ReadyList {
    fn clone(&self) -> Self {
        *self
    }
}
