use optimistic_lock_coupling::*;
fn main() {
    let lock = OptimisticLockCoupling::from(1);
    *lock.write().unwrap() += 1;
    assert_eq!(*lock.write().unwrap(), 2);
}
