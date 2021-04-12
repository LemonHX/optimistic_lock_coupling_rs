use optimistic_lock_coupling::*;
fn main() {
    let i = 10000;
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
                        println!("{:?}", guard);
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
    unsafe { assert_eq!(*(lock.as_ref().unwrap().write().unwrap()), i) }
}
