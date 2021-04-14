use std::{fmt::Display, ops::Deref, sync::atomic::AtomicU8};

use crate::{OptimisticLockCoupling, OptimisticLockCouplingResult};
use std::fmt::Debug;
// usage:
// after getting the guard you can do what ever you want with Deref
// but after usage you **MUST** call `try_sync`
// if fails you must redo the hole function or other sync method to ensure the data you read is correct.
pub struct OptimisticLockCouplingReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a OptimisticLockCoupling<T>,
    version: u64,
}
impl<'a, T: ?Sized> OptimisticLockCouplingReadGuard<'a, T> {
    #[inline(always)]
    pub fn new(lock: &'a OptimisticLockCoupling<T>) -> OptimisticLockCouplingResult<Self> {
        use crate::OptimisticLockCouplingErrorType::*;
        if lock.is_poisoned() {
            return Err(Poisoned);
        }
        let version = lock.try_lock()?;
        Ok(Self {
            lock: &lock,
            version,
        })
    }
}
impl<T: ?Sized> !Send for OptimisticLockCouplingReadGuard<'_, T> {}
impl<T: ?Sized> OptimisticLockCouplingReadGuard<'_, T> {
    // consume self return retry or not
    #[inline(always)]
    pub fn try_sync(self) -> OptimisticLockCouplingResult<()> {
        if self.version == self.lock.try_lock()? {
            drop(self);
            Ok(())
        } else {
            use crate::OptimisticLockCouplingErrorType::*;
            Err(VersionUpdated)
        }
    }
}
impl<T: ?Sized> Deref for OptimisticLockCouplingReadGuard<'_, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: Debug> Debug for OptimisticLockCouplingReadGuard<'_, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimisticLockCouplingReadGuard")
            .field("version", &(self.version >> 2))
            .field("data", self.deref())
            .finish()
    }
}
impl<T: Debug + Display> Display for OptimisticLockCouplingReadGuard<'_, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "OptimisticLockCouplingReadGuard (ver: {}) {}",
            self.version >> 2,
            self.deref()
        ))
    }
}
