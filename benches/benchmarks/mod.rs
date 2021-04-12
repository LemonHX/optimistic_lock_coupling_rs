#![allow(deprecated)]
pub mod compare_heavy_read;
pub mod compare_heavy_write;
pub mod compare_read_only;
pub mod compare_write_only;

use std::sync::{Arc, Mutex, RwLock};

use criterion::{criterion_group, BenchmarkId, Criterion, Fun, ParameterizedBenchmark};
use optimistic_lock_coupling::OptimisticLockCoupling;

// start 0:0:0
// end 0:i:i
struct ListRwLock {
    head: i32,
    tail: Option<Arc<RwLock<ListRwLock>>>,
}
impl ListRwLock {
    fn new() -> Self {
        let node3 = Some(Arc::new(RwLock::new(ListRwLock {
            head: 0,
            tail: None,
        })));
        let node2 = Some(Arc::new(RwLock::new(ListRwLock {
            head: 0,
            tail: node3,
        })));
        ListRwLock {
            head: 0,
            tail: node2,
        }
    }
    // simulate select * from db;
    fn get_all(&self) -> (i32, i32, i32) {
        let h1 = self.head;
        let t1 = self.tail.as_ref().unwrap().as_ref().read().unwrap();
        let h2 = t1.head;
        let t2 = t1.tail.as_ref().unwrap().as_ref().read().unwrap();
        let h3 = t2.head;
        (h1, h2, h3)
    }
    // simutale select * from db where id = 2
    fn get2(&self) -> i32 {
        let t1 = self.tail.as_ref().unwrap().as_ref().read().unwrap();
        let h2 = t1.head;
        h2
    }
    // simulate update id = 2
    fn set2(&self) {
        let mut t1 = self.tail.as_ref().unwrap().as_ref().write().unwrap();
        t1.head += 1;
    }
    // simulate update id = 3
    fn set3(&self) {
        let t1 = self.tail.as_ref().unwrap().as_ref().read().unwrap();
        let mut t2 = t1.tail.as_ref().unwrap().as_ref().write().unwrap();
        t2.head += 1;
    }
}
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
            let t1 = self.tail.as_ref().unwrap().as_ref().read();
            match t1 {
                Ok(t1) => {
                    let h2 = t1.head;
                    let t2 = t1.tail.as_ref().unwrap().as_ref().read();
                    match t2 {
                        Ok(t2) => {
                            let h3 = t2.head;
                            match t2.try_sync() {
                                Ok(_) => {}
                                Err(_) => {
                                    continue;
                                }
                            }
                            match t1.try_sync() {
                                Ok(_) => {
                                    return (h1, h2, h3);
                                }
                                Err(_) => {
                                    continue;
                                }
                            }
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
    // simutale select * from db where id = 2
    fn get2(&self) -> i32 {
        loop {
            let t1 = self.tail.as_ref().unwrap().as_ref().read();
            match t1 {
                Ok(t1) => {
                    let h2 = t1.head;
                    match t1.try_sync() {
                        Ok(_) => return h2,
                        Err(_) => {
                            continue;
                        }
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
    // simulate update id = 2
    fn set2(&self) {
        let mut t1 = self.tail.as_ref().unwrap().as_ref().write().unwrap();
        t1.head += 1;
    }
    // simulate update id = 3
    fn set3(&self) {
        loop {
            let t1 = self.tail.as_ref().unwrap().as_ref().read();
            match t1 {
                Ok(t1) => {
                    let mut t2 = t1.tail.as_ref().unwrap().as_ref().write().unwrap();
                    t2.head += 1;
                    // thanks to rust saved my ass
                    drop(t2);
                    match t1.try_sync() {
                        Ok(_) => {
                            return;
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
}
