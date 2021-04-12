use crate::OptimisticLockCoupling;
use std::{fmt::Debug, sync::atomic::Ordering::*};
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

// only one instance because the data is locked
// implemented `Deref` and `DerefMut`
// release the lock on drop
pub struct OptimisticLockCouplingWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a OptimisticLockCoupling<T>,
}
unsafe impl<T: ?Sized + Sync> Sync for OptimisticLockCouplingWriteGuard<'_, T> {}
impl<T: ?Sized> Deref for OptimisticLockCouplingWriteGuard<'_, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}
impl<T: ?Sized> DerefMut for OptimisticLockCouplingWriteGuard<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}
impl<T: ?Sized> Drop for OptimisticLockCouplingWriteGuard<'_, T> {
    #[inline(always)]
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.lock.poisoned.fetch_or(true, Release);
        } else {
            self.lock.version_lock_outdate.fetch_add(0b10, Release);
        }
    }
}
impl<T: Debug> Debug for OptimisticLockCouplingWriteGuard<'_, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimisticLockCouplingWriteGuard")
            .field(
                "version",
                &(self.lock.version_lock_outdate.load(Relaxed) >> 2),
            )
            .field("data", self.deref())
            .finish()
    }
}
impl<T: Debug + Display> Display for OptimisticLockCouplingWriteGuard<'_, T> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "OptimisticLockCouplingWriteGuard (ver: {}) {}",
            self.lock.version_lock_outdate.load(Relaxed) >> 2,
            self.deref()
        ))
    }
}
impl<'a, T: ?Sized> OptimisticLockCouplingWriteGuard<'a, T> {
    #[inline(always)]
    pub fn new(lock: &'a OptimisticLockCoupling<T>) -> Self {
        Self { lock }
    }
}
