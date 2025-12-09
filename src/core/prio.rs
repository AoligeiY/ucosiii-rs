//! Priority bitmap management for O(1) highest-ready lookup
//!
//! This module implements the priority table using a bitmap approach.
//! Leverages the CLZ (Count Leading Zeros) instruction for efficient
//! highest-priority determination.

use crate::config::CFG_PRIO_MAX;
use crate::types::OsPrio;

/// Number of words needed for the priority bitmap
const PRIO_TBL_SIZE: usize = (CFG_PRIO_MAX + 31) / 32;

/// Priority bitmap table
///
/// Each bit represents a priority level. A set bit means there's at least
/// one ready task at that priority. Bit 0 of word 0 is highest priority (0),
/// with priorities increasing toward lower significance and higher word indices.
pub struct PrioTable {
    bitmap: [u32; PRIO_TBL_SIZE],
}

impl PrioTable {
    pub const fn new() -> Self {
        PrioTable {
            bitmap: [0; PRIO_TBL_SIZE],
        }
    }

    pub fn init(&mut self) {
        for word in self.bitmap.iter_mut() {
            *word = 0;
        }
    }

    /// Insert a priority into the bitmap 
    #[inline]
    pub fn insert(&mut self, prio: OsPrio) {
        debug_assert!((prio as usize) < CFG_PRIO_MAX);
        
        let word_idx = (prio / 32) as usize;
        let bit_pos = 31 - (prio % 32);
        
        self.bitmap[word_idx] |= 1 << bit_pos;
    }

    /// Remove a priority from the bitmap
    #[inline]
    pub fn remove(&mut self, prio: OsPrio) {
        debug_assert!((prio as usize) < CFG_PRIO_MAX);
        
        let word_idx = (prio / 32) as usize;
        let bit_pos = 31 - (prio % 32);
        
        self.bitmap[word_idx] &= !(1 << bit_pos);
    }

    /// Get the highest priority
    #[inline]
    pub fn get_highest(&self) -> OsPrio {
        #[cfg(any())]
        {
            // Single word optimization (up to 32 priorities)
            if PRIO_TBL_SIZE == 1 {
                return Self::clz(self.bitmap[0]);
            }
            
            // Two word optimization (up to 64 priorities)
            if PRIO_TBL_SIZE == 2 {
                if self.bitmap[0] != 0 {
                    return Self::clz(self.bitmap[0]);
                } else {
                    return 32 + Self::clz(self.bitmap[1]);
                }
            }
        }

        let mut prio: OsPrio = 0;
        for &word in self.bitmap.iter() {
            if word != 0 {
                prio += Self::clz(word);
                return prio;
            }
            prio += 32;
        }

        // return lowest priority
        (CFG_PRIO_MAX - 1) as OsPrio
    }

    /// Check if a specific priority has any ready tasks
    #[inline]
    pub fn is_set(&self, prio: OsPrio) -> bool {
        let word_idx = (prio / 32) as usize;
        let bit_pos = 31 - (prio % 32);
        
        (self.bitmap[word_idx] & (1 << bit_pos)) != 0
    }

    /// Check if the priority table is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bitmap.iter().all(|&w| w == 0)
    }

    /// Count leading zeros
    #[inline]
    fn clz(value: u32) -> OsPrio {
        if value == 0 {
            32
        } else {
            value.leading_zeros() as OsPrio
        }
    }
}

impl Default for PrioTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_table() {
        let table = PrioTable::new();
        assert!(table.is_empty());
        assert_eq!(table.get_highest(), (CFG_PRIO_MAX - 1) as OsPrio);
    }

    #[test]
    fn test_insert_remove() {
        let mut table = PrioTable::new();
        
        table.insert(5);
        assert!(table.is_set(5));
        assert!(!table.is_set(4));
        assert_eq!(table.get_highest(), 5);
        
        table.insert(3);
        assert_eq!(table.get_highest(), 3);
        
        table.remove(3);
        assert_eq!(table.get_highest(), 5);
        
        table.remove(5);
        assert!(table.is_empty());
    }

    #[test]
    fn test_priority_order() {
        let mut table = PrioTable::new();
        
        table.insert(10);
        table.insert(5);
        table.insert(20);
        table.insert(0);
        table.insert(15);
        
        assert_eq!(table.get_highest(), 0);
        
        table.remove(0);
        assert_eq!(table.get_highest(), 5);
        
        table.remove(5);
        assert_eq!(table.get_highest(), 10);
    }

    #[test]
    fn test_boundary_priorities() {
        let mut table = PrioTable::new();
        
        table.insert(31);
        assert_eq!(table.get_highest(), 31);
        
        table.insert(32);
        assert_eq!(table.get_highest(), 31);
        
        table.remove(31);
        assert_eq!(table.get_highest(), 32);
    }
}
