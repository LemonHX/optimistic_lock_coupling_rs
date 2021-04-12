use criterion::{criterion_group, BenchmarkId, Criterion, Fun, ParameterizedBenchmark};

use super::ListMutex;
use super::ListOLock;
use super::ListRwLock;
use optimistic_lock_coupling::OptimisticLockCoupling;
use std::{
    collections::binary_heap::Iter,
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
    thread::spawn,
};

fn heavy_read_mutex(i: i32) {
    static mut lock: Option<Mutex<i32>> = None;
    unsafe { lock = Some(Mutex::from(0)) };

    let write_fn = move || unsafe {
        for _i in 0..i {
            std::thread::sleep_ms(10);
            match lock.as_ref().unwrap().lock() {
                Ok(mut guard) => {
                    *guard += 1;
                }
                Err(_poison) => {
                    panic!(" fuck me! ")
                }
            }
        }
    };
    let read_fn = move || unsafe {
        for _i in 0..i {
            std::thread::sleep_ms(8);
            match lock.as_ref().unwrap().lock() {
                Ok(guard) => {
                    let _ = *guard + 1;
                }
                Err(_poison) => {
                    panic!(" fuck me! ")
                }
            }
        }
    };
    let thread1 = spawn(write_fn);
    let thread2 = spawn(read_fn);
    let thread3 = spawn(read_fn);

    let _ = thread1.join();
    let _ = thread2.join();
    let _ = thread3.join();
}
fn heavy_read_rwlock(i: i32) {
    static mut lock: Option<RwLock<i32>> = None;
    unsafe { lock = Some(RwLock::from(0)) };

    let write_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(10);
            match lock.as_ref().unwrap().write() {
                Ok(mut guard) => {
                    *guard += 1;
                }
                Err(_poison) => {
                    panic!(" fuck me! ")
                }
            }
        }
    };
    let read_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            match lock.as_ref().unwrap().read() {
                Ok(guard) => {
                    let _ = *guard + 1;
                }
                Err(_poison) => {
                    panic!(" fuck me! ")
                }
            }
        }
    };
    let thread1 = spawn(write_fn);
    let thread2 = spawn(read_fn);
    let thread3 = spawn(read_fn);

    let _ = thread1.join();
    let _ = thread2.join();
    let _ = thread3.join();
    unsafe { assert_eq!(*(lock.as_ref().unwrap().write().unwrap().deref()), i) }
}
fn heavy_read_optimistic_lock_coupling(i: i32) {
    static mut lock: Option<OptimisticLockCoupling<i32>> = None;
    unsafe { lock = Some(OptimisticLockCoupling::from(0)) };
    let write_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(10);
            loop {
                match lock.as_ref().unwrap().write() {
                    Ok(mut guard) => {
                        *guard += 1;
                        break;
                    }
                    Err(_err) => {
                        continue;
                    }
                }
            }
        }
    };
    let read_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            loop {
                let g = lock.as_ref().unwrap();
                match g.read() {
                    Ok(guard) => {
                        let _ = *guard + 1;
                        match guard.try_sync() {
                            Ok(_) => {
                                break;
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
    };
    use std::thread::spawn;
    let thread1 = spawn(write_fn);
    let thread2 = spawn(read_fn);
    let thread3 = spawn(read_fn);

    let _ = thread1.join();
    let _ = thread2.join();
    let _ = thread3.join();
    unsafe { assert_eq!(*(lock.as_ref().unwrap().write().unwrap().deref()), i) }
}

fn heavy_read_list_rwlock(i: i32) {
    static mut lock: Option<ListRwLock> = None;
    unsafe { lock = Some(ListRwLock::new()) };
    let write_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(10);
            lock.as_ref().unwrap().set2();
            // std::thread::sleep_ms(12);
            lock.as_ref().unwrap().set3();
        }
    };
    let read_fn1 = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            lock.as_ref().unwrap().get_all();
        }
    };
    let read_fn2 = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            lock.as_ref().unwrap().get2();
        }
    };
    use std::thread::spawn;
    let thread1 = spawn(write_fn);
    let thread2 = spawn(read_fn1);
    let thread3 = spawn(read_fn2);

    let _ = thread1.join();
    let _ = thread2.join();
    let _ = thread3.join();
    unsafe { assert_eq!(lock.as_ref().unwrap().get2(), i) }
}
fn heavy_read_list_mutex(i: i32) {
    static mut lock: Option<ListMutex> = None;
    unsafe { lock = Some(ListMutex::new()) };
    let write_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(10);
            lock.as_ref().unwrap().set2();
            // std::thread::sleep_ms(12);
            lock.as_ref().unwrap().set3();
        }
    };
    let read_fn1 = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            lock.as_ref().unwrap().get_all();
        }
    };
    let read_fn2 = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            lock.as_ref().unwrap().get2();
        }
    };
    use std::thread::spawn;
    let thread1 = spawn(write_fn);
    let thread2 = spawn(read_fn1);
    let thread3 = spawn(read_fn2);

    let _ = thread1.join();
    let _ = thread2.join();
    let _ = thread3.join();
    unsafe { assert_eq!(lock.as_ref().unwrap().get2(), i) }
}

fn heavy_read_list_optimistic_lock_coupling(i: i32) {
    static mut lock: Option<ListOLock> = None;
    unsafe { lock = Some(ListOLock::new()) };
    let write_fn = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(10);
            lock.as_ref().unwrap().set2();
            // std::thread::sleep_ms(12);
            lock.as_ref().unwrap().set3();
        }
    };
    let read_fn1 = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            lock.as_ref().unwrap().get_all();
        }
    };
    let read_fn2 = move || unsafe {
        for _i in 0..i {
            // std::thread::sleep_ms(8);
            lock.as_ref().unwrap().get2();
        }
    };
    use std::thread::spawn;
    let thread1 = spawn(write_fn);
    let thread2 = spawn(read_fn1);
    let thread3 = spawn(read_fn2);

    let _ = thread1.join();
    let _ = thread2.join();
    let _ = thread3.join();
    unsafe { assert_eq!(lock.as_ref().unwrap().get2(), i) }
}

fn lock_heavy_read_i32(c: &mut Criterion) {
    // let mx = Fun::new("Mutex", |b, i| b.iter(|| heavy_read_mutex(*i)));
    let rw = Fun::new("RwLock", |b, i| b.iter(|| heavy_read_rwlock(*i)));
    let ol = Fun::new("OptimisticLockCoupling", |b, i| {
        b.iter(|| heavy_read_optimistic_lock_coupling(*i))
    });

    let functions = vec![ol, rw];

    c.bench_functions("Lock Heavy Read I32 Compare", functions, 100000);
}
fn lock_heavy_read_list(c: &mut Criterion) {
    let mx = Fun::new("Mutex", |b, i| b.iter(|| heavy_read_list_mutex(*i)));
    let rw = Fun::new("RwLock", |b, i| b.iter(|| heavy_read_list_rwlock(*i)));
    let ol = Fun::new("OptimisticLockCoupling", |b, i| {
        b.iter(|| heavy_read_list_optimistic_lock_coupling(*i))
    });

    let functions = vec![ol, rw, mx];

    c.bench_functions("Lock Heavy Read List Compare", functions, 100000);
}

criterion_group!(name = lock_heavy_read; config = Criterion::default().sample_size(100); targets = lock_heavy_read_list);
