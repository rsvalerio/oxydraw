//! Shared synchronization helpers.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Lock `mutex`, recovering from poisoning instead of panicking.
///
/// A panic in one thread while it holds a guard poisons a `std::sync::Mutex`; plain
/// `lock().expect(...)` then panics on every later acquisition, turning a one-off fault
/// into a permanent outage for that subsystem. The maps guarded this way hold plain data
/// that stays structurally valid mid-update, so continuing with the recovered guard is
/// safe.
pub fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Acquire a shared read guard on `lock`, recovering from poisoning instead of panicking.
/// Same rationale as [`lock_unpoisoned`]: the guarded data stays structurally valid, so a
/// poisoned lock should degrade to a usable guard rather than a permanent outage.
pub fn read_unpoisoned<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// Acquire an exclusive write guard on `lock`, recovering from poisoning. See
/// [`read_unpoisoned`].
pub fn write_unpoisoned<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_and_returns_the_value_when_not_poisoned() {
        let mutex = Mutex::new(7);
        *lock_unpoisoned(&mutex) += 1;
        assert_eq!(*lock_unpoisoned(&mutex), 8);
    }

    #[test]
    fn recovers_a_usable_guard_from_a_poisoned_mutex() {
        let mutex = Mutex::new(7);
        // Poison the mutex: panic while holding the guard.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison the mutex");
        }));
        assert!(result.is_err());
        assert!(mutex.is_poisoned());
        assert_eq!(*lock_unpoisoned(&mutex), 7, "guard still usable");
    }

    #[test]
    fn read_recovers_a_usable_guard_from_a_poisoned_rwlock() {
        let lock = RwLock::new(7);
        // Poison the lock: panic while holding a write guard.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.write().unwrap();
            panic!("poison the rwlock");
        }));
        assert!(result.is_err());
        assert!(lock.is_poisoned());
        assert_eq!(*read_unpoisoned(&lock), 7, "read guard still usable");
    }

    #[test]
    fn write_recovers_a_usable_guard_from_a_poisoned_rwlock() {
        let lock = RwLock::new(7);
        // Poison the lock: panic while holding a write guard.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.write().unwrap();
            panic!("poison the rwlock");
        }));
        assert!(result.is_err());
        assert!(lock.is_poisoned());
        *write_unpoisoned(&lock) += 1;
        assert_eq!(*read_unpoisoned(&lock), 8, "write guard still writable");
    }
}
