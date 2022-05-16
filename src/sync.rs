use std::sync::{RwLock as StdRwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct RwLock<T>(StdRwLock<T>);

impl<T> RwLock<T> {
    pub fn new(t: T) -> Self {
        Self(StdRwLock::new(t))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        match self.0.read() {
            Ok(lock) => lock,
            Err(err) => panic!("Failed to get read lock: {}", err),
        }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        match self.0.write() {
            Ok(lock) => lock,
            Err(err) => panic!("Failed to get write lock: {}", err),
        }
    }
}
