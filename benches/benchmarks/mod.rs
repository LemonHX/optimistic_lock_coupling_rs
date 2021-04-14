#![allow(deprecated)]
pub mod compare_heavy_read;
pub mod compare_heavy_write;
pub mod compare_read_only;
pub mod compare_write_only;

use std::sync::{Arc, Mutex, RwLock};

use optimistic_lock_coupling::OptimisticLockCoupling;

// start 0:0:0
// end 0:i:i
struct ListMutex {
    head: i32,
    tail: Option<Arc<Mutex<ListMutex>>>,
}
impl ListMutex {
    fn new() -> Self {
        let node3 = Some(Arc::new(Mutex::new(ListMutex {
            head: 0,
            tail: None,
        })));
        let node2 = Some(Arc::new(Mutex::new(ListMutex {
            head: 0,
            tail: node3,
        })));
        ListMutex {
            head: 0,
            tail: node2,
        }
    }
    // simulate select * from db;
    fn get_all(&self) -> (i32, i32, i32) {
        let h1 = self.head;
        let t1 = self.tail.as_ref().unwrap().as_ref().lock().unwrap();
        let h2 = t1.head;
        let t2 = t1.tail.as_ref().unwrap().as_ref().lock().unwrap();
        let h3 = t2.head;
        (h1, h2, h3)
    }
    // simutale select * from db where id = 2
    fn get2(&self) -> i32 {
        let t1 = self.tail.as_ref().unwrap().as_ref().lock().unwrap();
        let h2 = t1.head;
        h2
    }
    // simulate update id = 2
    fn set2(&self) {
        let mut t1 = self.tail.as_ref().unwrap().as_ref().lock().unwrap();
        t1.head += 1;
    }
    // simulate update id = 3
    fn set3(&self) {
        let t1 = self.tail.as_ref().unwrap().as_ref().lock().unwrap();
        let mut t2 = t1.tail.as_ref().unwrap().as_ref().lock().unwrap();
        t2.head += 1;
    }
}

struct ListOLock {
    head: i32,
    tail: Option<Arc<OptimisticLockCoupling<ListOLock>>>,
}
impl ListOLock {
    fn new() -> Self {
        let node3 = Some(Arc::new(OptimisticLockCoupling::new(ListOLock {
            head: 0,
            tail: None,
        })));
        let node2 = Some(Arc::new(OptimisticLockCoupling::new(ListOLock {
            head: 0,
            tail: node3,
        })));
        ListOLock {
            head: 0,
            tail: node2,
        }
    }
    // simulate select * from db;
    fn get_all(&self) -> (i32, i32, i32) {
        loop {
            let h1 = self.head;
            if let Ok((h2, h3)) = self.tail.as_ref().unwrap().read_txn(
                #[inline(always)]
                |rg| {
                    let h2 = rg.head;
                    let h3 = rg.tail.as_ref().unwrap().read_txn(
                        #[inline(always)]
                        |rg| {
                            let h3 = rg.head;
                            Ok(h3)
                        },
                    )?;
                    Ok((h2, h3))
                },
            ) {
                return (h1, h2, h3);
            } else {
                continue;
            }
        }
    }
    // simutale select * from db where id = 2
    fn get2(&self) -> i32 {
        loop {
            if let Ok(h2) = self.tail.as_ref().unwrap().read_txn(
                #[inline(always)]
                |rg| {
                    let h2 = rg.head;
                    Ok(h2)
                },
            ) {
                return h2;
            } else {
                continue;
            }
        }
    }
    // simulate update id = 2
    fn set2(&self) {
        loop {
            let t1 = self.tail.as_ref().unwrap().as_ref().write();
            match t1 {
                Ok(mut t1) => {
                    t1.head += 1;
                    break;
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
    // simulate update id = 3
    fn set3(&self) {
        loop {
            if let Ok(()) = self.tail.as_ref().unwrap().read_txn(
                #[inline(always)]
                |rg| {
                    // in 2
                    let mut g = rg.tail.as_ref().unwrap().write()?;
                    g.head += 1;
                    Ok(())
                },
            ) {
                return;
            } else {
                continue;
            }
        }
    }
}
