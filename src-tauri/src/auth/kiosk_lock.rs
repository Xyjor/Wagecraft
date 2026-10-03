//! Kiosk PIN lockout (plan §6.3). After 5 wrong PINs for one employee number, the kiosk
//! refuses that number for 15 minutes. Held in memory like the session, so it is per app run.
//!
//! The lock follows the number that was typed, whether or not an employee has it, so a
//! locked-out message never reveals which employee numbers exist.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

const MAX_FAILED_PINS: u32 = 5;
const LOCK_FOR: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
struct Failures {
    count: u32,
    locked_until: Option<Instant>,
}

#[derive(Default)]
pub struct KioskLock {
    by_number: Mutex<HashMap<String, Failures>>,
}

impl KioskLock {
    /// The lock key for a typed employee number: trimmed, upper-cased (numbers are
    /// case-insensitive) and cut to a sane length so junk input can't grow the map much.
    pub fn key(employee_no: &str) -> String {
        employee_no
            .trim()
            .chars()
            .take(32)
            .collect::<String>()
            .to_uppercase()
    }

    pub fn is_locked(&self, key: &str, now: Instant) -> bool {
        let map = self.by_number.lock().expect("kiosk lock");
        map.get(key)
            .and_then(|f| f.locked_until)
            .is_some_and(|until| until > now)
    }

    /// Counts a wrong PIN. Returns true when this one locks the number.
    pub fn fail(&self, key: &str, now: Instant) -> bool {
        let mut map = self.by_number.lock().expect("kiosk lock");
        let f = map.entry(key.to_string()).or_default();
        if f.locked_until.is_some_and(|until| until <= now) {
            *f = Failures::default();
        }
        f.count += 1;
        if f.count >= MAX_FAILED_PINS {
            f.count = 0;
            f.locked_until = Some(now + LOCK_FOR);
            true
        } else {
            false
        }
    }

    /// Forgets the failures for a number: after a correct PIN, or when HR sets a new PIN.
    pub fn clear(&self, key: &str) {
        self.by_number.lock().expect("kiosk lock").remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_wrong_pins_lock_the_number_for_15_minutes() {
        let lock = KioskLock::default();
        let t0 = Instant::now();
        for _ in 0..4 {
            assert!(!lock.fail("EMP-0001", t0));
        }
        assert!(!lock.is_locked("EMP-0001", t0));
        assert!(lock.fail("EMP-0001", t0));
        assert!(lock.is_locked("EMP-0001", t0 + Duration::from_secs(14 * 60)));
        assert!(!lock.is_locked("EMP-0001", t0 + LOCK_FOR));
        // Other numbers are not affected.
        assert!(!lock.is_locked("EMP-0002", t0));
    }

    #[test]
    fn after_the_lock_ends_the_count_starts_over() {
        let lock = KioskLock::default();
        let t0 = Instant::now();
        for _ in 0..5 {
            lock.fail("EMP-0001", t0);
        }
        let later = t0 + LOCK_FOR;
        assert!(!lock.fail("EMP-0001", later));
        assert!(!lock.is_locked("EMP-0001", later));
    }

    #[test]
    fn clearing_forgets_earlier_failures() {
        let lock = KioskLock::default();
        let t0 = Instant::now();
        for _ in 0..4 {
            lock.fail("EMP-0001", t0);
        }
        lock.clear("EMP-0001");
        assert!(!lock.fail("EMP-0001", t0));
    }

    #[test]
    fn keys_ignore_case_and_spaces() {
        assert_eq!(KioskLock::key("  emp-0001 "), "EMP-0001");
        assert_eq!(KioskLock::key(&"9".repeat(100)).len(), 32);
    }
}
