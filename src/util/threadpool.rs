use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

use num_cpus;

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Truck<'a> = Box<FnBox + Send + 'a>;

pub struct Pool {
    inner: Arc<Inner>,
}

struct Inner {
    queue: Mutex<VecDeque<Truck<'static>>>,
    condvar: Condvar,
    active: AtomicUsize,
    waiting: AtomicUsize,
    min_num: usize,
    max_num: usize,
}

struct Count<'a> {
    num: &'a AtomicUsize,
}

impl<'a> Count<'a> {
    fn add(num: &'a AtomicUsize) -> Count<'a> {
        num.fetch_add(1, Ordering::Release);
        
        Count {
            num: num,
        }
    }
}

impl<'a> Drop for Count<'a> {
    fn drop(&mut self) {
        self.num.fetch_sub(1, Ordering::Release);
    }
}

impl Pool {
    pub fn new() -> Pool {
        let min_num = num_cpus::get();
        let max_num = min_num * 16;

        let pool = Pool {
            inner: Arc::new(Inner {
                queue: Mutex::new(VecDeque::with_capacity(max_num * 16)),
                condvar: Condvar::new(),
                active: AtomicUsize::new(0),
                waiting: AtomicUsize::new(0),
                min_num: min_num,
                max_num: max_num,
            }),
        };

        for _ in 0..min_num {
            pool.thread();
        }

        pool
    }

    pub fn with_capacity(min: usize, max: usize) -> Pool {
        let pool = Pool {
            inner: Arc::new(Inner {
                queue: Mutex::new(VecDeque::with_capacity(max)),
                condvar: Condvar::new(),
                active: AtomicUsize::new(0),
                waiting: AtomicUsize::new(0),
                min_num: min,
                max_num: max,
            })
        };

        for _ in 0..min {
            pool.thread();
        }

        pool
    }
   
    pub fn execute<F>(&self, handle: F)
        where F: FnOnce() + Send + 'static
    {
        if self.inner.waiting.load(Ordering::Acquire) == 0 && self.inner.active.load(Ordering::Acquire) < self.inner.max_num + 1 {
            self.thread();
        }

        let mut queue = self.inner.queue.lock().unwrap();
            
        queue.push_back(Box::new(handle));
        self.inner.condvar.notify_one();
    }

    fn thread(&self) {
        let inner = self.inner.clone();

        thread::spawn(move || {

            let _active = Count::add(&inner.active);

            loop {
                let handle = {
                    let mut queue = inner.queue.lock().unwrap();

                    let handle;

                    loop {
                        if let Some(front) = queue.pop_front() {
                            handle = front;
                            break;
                        }

                        let _waiting = Count::add(&inner.waiting);

                        if inner.active.load(Ordering::Acquire) <= inner.min_num {
                            queue = inner.condvar.wait(queue).unwrap();
                        } else {
                            let (q, wait) = inner.condvar.wait_timeout(queue, Duration::from_secs(60)).unwrap();
                            queue = q;

                            if wait.timed_out() && queue.is_empty() && inner.active.load(Ordering::Acquire) > inner.min_num {
                                return;
                            }
                        }
                    }

                    handle
                };

                handle.call_box();
            }
        });
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.inner.active.store(usize::max_value(), Ordering::Release);
        self.inner.condvar.notify_all();
    }
}
