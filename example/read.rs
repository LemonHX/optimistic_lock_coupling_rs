use optimistic_lock_coupling::{OptimisticLockCoupling, OptimisticLockCouplingErrorType};

// a transactional read function
#[inline(always)]
fn read_txn(lock: &OptimisticLockCoupling<i32>) -> Result<(), OptimisticLockCouplingErrorType> {
    // acquire the read lock
    let read_guard = lock.read()?;
    // do your stuff
    println!("status: {}", read_guard);
    println!("\tmy operations: {} + 1 = {}", *read_guard, *read_guard + 1);
    // remember to sync before drop
    let res = read_guard.try_sync();
    println!("safely synced");
    res
}

fn main() {
    let lock = OptimisticLockCoupling::new(1);
    // retry steps
    'retry: loop{
        // function call
        let res = read_txn(&lock);
        // before retry logics
        if res.is_err() {
            continue 'retry;
        }else{
            break 'retry;
        }
    }
}