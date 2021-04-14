#![feature(negative_impls)]
#![feature(unboxed_closures)]
#![feature(stmt_expr_attributes)]

use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU64},
};
use std::{fmt::Debug, sync::atomic::Ordering::*};

use read_guard::OptimisticLockCouplingReadGuard;
use write_guard::OptimisticLockCouplingWriteGuard;

pub mod read_guard;
#[cfg(test)]
mod test;
pub mod write_guard;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OptimisticLockCouplingErrorType {
    Poisoned,
    Outdated,
    Blocked,
    VersionUpdated,
}
type OptimisticLockCouplingResult<T> = Result<T, OptimisticLockCouplingErrorType>;

// our data structure, the usage is 'pretty much' same as RwLock
pub struct OptimisticLockCoupling<T: ?Sized> {
    // 60 bit for version | 1 bit for lock | 1 bit for outdate
    version_lock_outdate: AtomicU64,
    // guard thread paniced
    poisoned: AtomicBool,
    // well the data
    data: UnsafeCell<T>,
}
unsafe impl<T: ?Sized + Send> Send for OptimisticLockCoupling<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for OptimisticLockCoupling<T> {}

impl<T> OptimisticLockCoupling<T> {
    #[inline(always)]
    pub fn new(t: T) -> Self {
        Self {
            version_lock_outdate: AtomicU64::new(0),
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }
    // logic should be an inlined closure
    #[inline(always)]
    pub fn read_txn<F, R>(&self, mut logic: F) -> OptimisticLockCouplingResult<R>
    where
        F: FnMut(&OptimisticLockCouplingReadGuard<T>) -> OptimisticLockCouplingResult<R>,
    {
        'txn: loop {
            match self.read() {
                Ok(guard) => match logic(&guard) {
                    Ok(r) => match guard.try_sync() {
                        Ok(_) => {
                            return Ok(r);
                        }
                        Err(e) => match e {
                            OptimisticLockCouplingErrorType::Poisoned
                            | OptimisticLockCouplingErrorType::Outdated => {
                                return Err(e);
                            }
                            _ => {
                                continue 'txn;
                            }
                        },
                    },
                    Err(e) => match e {
                        OptimisticLockCouplingErrorType::Poisoned
                        | OptimisticLockCouplingErrorType::Outdated => {
                            return Err(e);
                        }
                        _ => {
                            continue 'txn;
                        }
                    },
                },
                Err(e) => match e {
                    OptimisticLockCouplingErrorType::Poisoned
                    | OptimisticLockCouplingErrorType::Outdated => {
                        return Err(e);
                    }
                    _ => {
                        continue 'txn;
                    }
                },
            }
        }
    }
}
impl<T: Sized> From<T> for OptimisticLockCoupling<T> {
    #[inline(always)]
    fn from(t: T) -> Self {
        Self::new(t)
    }
}
impl<T: ?Sized + Default> Default for OptimisticLockCoupling<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: ?Sized> OptimisticLockCoupling<T> {
    // make self outdate
    // usually used when the container grows and this pointer point to this structure is replaced
    #[inline]
    pub fn make_outdate(&self) {
        self.version_lock_outdate.fetch_or(0b1, Release);
    }
    // is writter thread dead?
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        // if fail then fail ~
        // no need extra sync
        self.poisoned.load(std::sync::atomic::Ordering::Acquire)
    }
    // try to aquire the lock but only internal use
    #[inline]
    fn try_lock(&self) -> OptimisticLockCouplingResult<u64> {
        use OptimisticLockCouplingErrorType::*;
        if self.is_poisoned() {
            return Err(Poisoned);
        }
        let version = self.version_lock_outdate.load(Acquire);
        if is_outdate(version) {
            return Err(Outdated);
        }
        if is_locked(version) {
            return Err(Blocked);
        }
        Ok(version)
    }
    // I suggest you redo the hole function when error occurs
    #[inline]
    pub fn read(&self) -> OptimisticLockCouplingResult<OptimisticLockCouplingReadGuard<'_, T>> {
        OptimisticLockCouplingReadGuard::new(self)
    }
    // get your RAII write guard
    #[inline]
    pub fn write(&self) -> OptimisticLockCouplingResult<OptimisticLockCouplingWriteGuard<'_, T>> {
        use OptimisticLockCouplingErrorType::*;
        let version = self.try_lock()?;
        match self
            .version_lock_outdate
            .compare_exchange(version, version + 0b10, Acquire, Acquire)
        {
            Ok(_) => Ok(OptimisticLockCouplingWriteGuard::new(self)),
            Err(_) => Err(VersionUpdated),
        }
    }
}

#[inline(always)]
fn is_locked(version: u64) -> bool {
    version & 0b10 != 0
}
#[inline(always)]
fn is_outdate(version: u64) -> bool {
    version & 0b1 != 0
}
