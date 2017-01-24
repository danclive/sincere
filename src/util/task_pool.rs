use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

use num_cpus;

#[derive(Clone)]
pub struct TaskPool {
    inner: Arc<Inner>,
}

struct Inner {
    queue: Mutex<VecDeque<Box<FnMut() + Send>>>,
    condvar: Condvar,
    active: AtomicUsize,
    waiting: AtomicUsize,
    min_num: usize,
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

impl TaskPool {
    pub fn new() -> TaskPool {
        let cpu_num = num_cpus::get();

        let pool = TaskPool {
            inner: Arc::new(Inner {
                queue: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                active: AtomicUsize::new(0),
                waiting: AtomicUsize::new(0),
                min_num: cpu_num,
            }),
        };

        for _ in 0..cpu_num {
            pool.add_thread(None);
        }

        pool
    }

    pub fn with_capacity(n: usize) -> TaskPool {
        let pool = TaskPool {
            inner: Arc::new(Inner {
                queue: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                active: AtomicUsize::new(0),
                waiting: AtomicUsize::new(0),
                min_num: n,
            })
        };

        for _ in 0..n {
            pool.add_thread(None);
        }

        pool
    }
   
    pub fn spawn(&self, handle: Box<FnMut() + Send>) {
        let mut queue = self.inner.queue.lock().unwrap();

        if self.inner.waiting.load(Ordering::Acquire) == 0 {
            self.add_thread(Some(handle));
        } else {
            queue.push_back(handle);
            self.inner.condvar.notify_one();
        }
    }

    fn add_thread(&self, handle: Option<Box<FnMut() + Send>>) {
        let inner = self.inner.clone();

        thread::spawn(move || {
            let inner = inner;
            let _active = Count::add(&inner.active);

            if let Some(mut h) = handle {
                h();
            }

            loop {
                let mut handle = {
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
                            let (q, wait) = inner.condvar.wait_timeout(queue, Duration::from_secs(10)).unwrap();
                            queue = q;

                            if wait.timed_out() && queue.is_empty() {
                                return;
                            }
                        }
                    }

                    handle
                };

                handle();
            }
        });
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.inner.active.store(usize::max_value(), Ordering::Release);
        self.inner.condvar.notify_all();
    }
}
