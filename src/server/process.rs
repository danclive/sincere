use std::sync::Arc;
use std::thread;

use soio::channel::{self, Sender};
use soio::tcp::TcpStream;

use rustls;

use error::Result;

use super::Handle;
use super::Worker;

pub struct Process {
    tx: Sender<TcpStream>
}

impl Process {
    pub fn new(handle: Arc<Handle>, tls_config: Option<Arc<rustls::ServerConfig>>) -> Result<Process> {
        let (tx, rx) = channel::channel();

        thread::spawn(move || {
            match tls_config {
                Some(tls_config) => {
                    let mut worker = Worker::new(rx, handle, Some(tls_config));
                    worker.run().expect("thread painc");
                }
                None => {
                    let mut worker = Worker::new(rx, handle, None);
                    worker.run().expect("thread painc");
                }
            }
        });

        Ok(Process {
            tx: tx
        })
    }

    pub fn send(&self, socket: TcpStream) -> Result<()> {
        self.tx.send(socket)?;
        Ok(())
    }
}
