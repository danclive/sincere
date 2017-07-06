use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use soio::channel::{self, Sender};
use soio::tcp::TcpStream;

use rustls;

use error::Result;

use super::Handle;
use super::Worker;

pub struct Process {
    tx: Sender<TcpStream>,
    active: Arc<AtomicUsize>,
}

impl Process {
    pub fn new(handle: Arc<Handle>, tls_config: Option<Arc<rustls::ServerConfig>>) -> Result<Process> {
        let (tx, rx) = channel::channel();

        let active = Arc::new(AtomicUsize::new(0));

        let active2 = active.clone();

        thread::spawn(move || {
            match tls_config {
                Some(tls_config) => {
                    let mut worker = Worker::new(rx, active2, handle, Some(tls_config));
                    worker.run().expect("thread painc");
                }
                None => {
                    let mut worker = Worker::new(rx, active2, handle, None);
                    worker.run().expect("thread painc");
                }
            }
        });

        Ok(Process {
            tx: tx,
            active: active,
        })
    }

    pub fn send(&self, socket: TcpStream) -> Result<()> {
        self.tx.send(socket)?;
        Ok(())
    }

    pub fn active(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
}
