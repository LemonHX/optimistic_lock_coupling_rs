#![feature(negative_impls)]
#![feature(unboxed_closures)]
#![feature(stmt_expr_attributes)]
//! This crate provides a general optimistic lock.
//!
//! # Description
//!
//! In actual projects, there are some lock-free data structures, especially database-related ones such as `BwTree`, `Split-Ordered List` (also known as Lock Free HashTable) when there are many write conflicts.
//! The performance loss is very serious, and sometimes it is not as good as the brainless Mutex. However, the performance of the brainless Mutex is often very bad under normal circumstances, so is there an intermediate form that can solve this problem?
//!
//! This is the intermediate form. So this can be used everywhere as a general lock, and the performance is satisfactory.
//!
//! # Simple example for read
//!
//! ```
//! use optimistic_lock_coupling::{OptimisticLockCoupling, OptimisticLockCouplingErrorType};
//!
//! #[inline(always)]
//! fn read_txn(lock: &OptimisticLockCoupling<i32>) -> Result<(), OptimisticLockCouplingErrorType> {
//!     // acquire the read lock
//!     let read_guard = lock.read()?;
//!     // do your stuff
//!     println!("status: {}", read_guard);
//!     println!("\tmy operations: {} + 1 = {}", *read_guard, *read_guard + 1);
//!     // remember to sync before drop
//!     let res = read_guard.try_sync();
//!     println!("safely synced");
//!     res
//! }
//!
//! fn main() {
//!     let lock = OptimisticLockCoupling::new(1);
//!     // retry steps
//!     'retry: loop{
//!         // function call
//!         let res = read_txn(&lock);
//!         // before retry logics
//!         if res.is_err() {
//!             continue 'retry;
//!         }else{
//!             break 'retry;
//!         }
//!     }
//! }
//! ```
//! or in a much easy way~
//! ```
//! use optimistic_lock_coupling::{OptimisticLockCoupling, OptimisticLockCouplingErrorType};
//!
//! let lock = OptimisticLockCoupling::new(1);
//! lock.read_txn(
//!     // very important!
//!     #[inline(always)]
//!     |guard| {
//!         println!("{}", guard);
//!         Ok(())
//!     },
//! )
//! .unwrap();
//! ```
//! # Thread-safety example
//! will create a read thread and a write thread both holding the same lock.
//! ```
//! use optimistic_lock_coupling::*;
//! let i = 10000;
//! static mut LOCK: Option<OptimisticLockCoupling<i32>> = None;
//! unsafe { LOCK = Some(OptimisticLockCoupling::from(0)) };
//! let write_fn = move || unsafe {
//!     for _i in 0..i {
//!         // std::thread::sleep_ms(10);
//!         loop {
//!             match LOCK.as_ref().unwrap().write() {
//!                 Ok(mut guard) => {
//!                     *guard += 1;
//!                     break;
//!                 }
//!                 Err(_err) => {
//!                     continue;
//!                 }
//!             }
//!         }
//!     }
//! };
//! let read_fn = move || unsafe {
//!     for _i in 0..i {
//!         while let Ok(_) = LOCK.as_ref().unwrap().read_txn(
//!             #[inline(always)]
//!             |guard| {
//!                 println!("{}", guard);
//!                 Ok(())
//!             },
//!         ) {
//!             break;
//!         }
//!     }
//! };
//! use std::thread::spawn;
//! let thread1 = spawn(write_fn);
//! let thread2 = spawn(read_fn);

//! let _ = thread1.join();
//! let _ = thread2.join();
//! unsafe { assert_eq!(*(LOCK.as_ref().unwrap().write().unwrap()), i) }
//! ```

use std::{
    cell::UnsafeCell,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU64},
};
use std::{fmt::Debug, sync::atomic::Ordering::*};

#[cfg(test)]
mod test;

/// Error types
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OptimisticLockCouplingErrorType {
    /// writer thread panics without release the lock
    Poisoned,
    /// writer thread set this data is outdated
    Outdated,
    /// writer thread blocks the reader thread
    Blocked,
    /// reader thead try to sync after writer thread write things into lock
    VersionUpdated,
}
/// Result type~
pub type OptimisticLockCouplingResult<T> = Result<T, OptimisticLockCouplingErrorType>;

/// Our data structure, the usage is 'pretty much' same as RwLock
pub struct OptimisticLockCoupling<T: ?Sized> {
    /// 60 bit for version | 1 bit for lock | 1 bit for outdate
    version_lock_outdate: AtomicU64,
    /// guard thread paniced
    poisoned: AtomicBool,
    /// well the data
    data: UnsafeCell<T>,
}

/// Of course Lock could be Send
unsafe impl<T: ?Sized + Send> Send for OptimisticLockCoupling<T> {}
/// Of course Lock could be Sync
unsafe impl<T: ?Sized + Send + Sync> Sync for OptimisticLockCoupling<T> {}

impl<T> OptimisticLockCoupling<T> {
    /// create an instance of OLC
    #[inline(always)]
    pub const fn new(t: T) -> Self {
        Self {
            version_lock_outdate: AtomicU64::new(0),
            poisoned: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }
    /// read transaction
    /// logic should be an inlined closure
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
    /// make self outdate
    /// usually used when the container grows and this pointer point to this structure is replaced
    #[inline(always)]
    pub fn make_outdate(&self) {
        self.version_lock_outdate.fetch_or(0b1, Release);
    }
    /// is writter thread dead?
    /// if fail then fail ~
    /// no need extra sync
    #[inline(always)]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned.load(std::sync::atomic::Ordering::Acquire)
    }
    /// try to aquire the lock but only internal use
    #[inline(always)]
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
    /// I suggest you redo the hole function when error occurs
    /// Or just use `read_txn`
    #[inline(always)]
    pub fn read(&self) -> OptimisticLockCouplingResult<OptimisticLockCouplingReadGuard<'_, T>> {
        OptimisticLockCouplingReadGuard::new(self)
    }
    /// get your RAII write guard
    #[inline(always)]
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

#[inline]
const fn is_locked(version: u64) -> bool {
    version & 0b10 != 0
}

#[inline]
const fn is_outdate(version: u64) -> bool {
    version & 0b1 != 0
}

// ============= reader guard =============== //

/// Usage:
/// after getting the guard you can do what ever you want with Deref
/// but after usage you **MUST** call `try_sync`
/// if fails you must redo the hole function or other sync method to ensure the data you read is correct.
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
        Ok(Self { lock, version })
    }
}
impl<T: ?Sized> !Send for OptimisticLockCouplingReadGuard<'_, T> {}
impl<T: ?Sized> OptimisticLockCouplingReadGuard<'_, T> {
    /// Consume self return retry or not
    /// suggest to use `read_txn`
    #[inline(always)]
    pub fn try_sync(self) -> OptimisticLockCouplingResult<()> {
        if self.version == self.lock.try_lock()? {
            drop(self);
            Ok(())
        } else {
            Err(crate::OptimisticLockCouplingErrorType::VersionUpdated)
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

// ============= writer guard =============== //

/// Only one instance because the data is locked
/// implemented `Deref` and `DerefMut`
/// release the lock on drop
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
    #[inline]
    pub const fn new(lock: &'a OptimisticLockCoupling<T>) -> Self {
        Self { lock }
    }
}
