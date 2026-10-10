use std::sync::{Mutex, MutexGuard, PoisonError};

pub trait LockExt<T: ?Sized> {
    fn lock_or_recover(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> LockExt<T> for Mutex<T> {
    fn lock_or_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn recovers_from_poisoned_mutex() {
        let mutex = Arc::new(Mutex::new(1));
        let poisoner = mutex.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock_or_recover();
            panic!("poison");
        })
        .join();
        assert!(mutex.is_poisoned());
        *mutex.lock_or_recover() += 1;
        assert_eq!(*mutex.lock_or_recover(), 2);
    }
}
