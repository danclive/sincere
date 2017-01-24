use std::sync::{Arc, Mutex};
use std::io::Read;
use std::io::Write;
use std::io::ErrorKind::WouldBlock;
use std::mem;

use mio::Token;
use mio::tcp::TcpStream;
use mio::channel;

use util::TaskPool;

use super::Handle;
use super::stream::Stream;

pub enum Event {
    Close(Token),
    Write(Token),
    Read(Token),
}

pub struct Connention {
    pub socket: TcpStream,
    token: Token,
    task_pool: TaskPool,
    tx: channel::Sender<Event>,
    writer: Arc<Mutex<Vec<u8>>>,
    pub closing: bool,
    handle: Arc<Handle>,
}

impl Connention {
    pub fn new(socket: TcpStream, token: Token, task_pool: TaskPool, tx: channel::Sender<Event>, handle: Arc<Handle>) -> Connention {
        Connention {
            socket: socket,
            token: token,
            task_pool: task_pool,
            tx: tx,
            writer: Arc::new(Mutex::new(Vec::new())),
            closing: false,
            handle: handle,
        }
    }

    pub fn read(&mut self) {
        let mut reader = Vec::new();

        loop {
            let mut buf = [0; 1024];

            match self.socket.read(&mut buf) {
                Ok(size) => {
                    if size == 0 {
                        self.closing = true;
                        return;
                    } else {
                        reader.extend_from_slice(&buf[0..size]);
                    }
                },
                Err(err) => {
                    if let WouldBlock = err.kind() {
                        break;
                    }

                    self.closing = true;
                    break;
                }
            }
        }

        let writer = self.writer.clone();

        let tx = self.tx.clone();
        let token = self.token.clone();

        let handle = self.handle.clone();

        let remote_addr = self.socket.peer_addr().unwrap();

        self.task_pool.spawn(Box::new(move || {

            handle(Stream::new(mem::replace(&mut reader, Vec::new()), writer.clone(), remote_addr));

            tx.send(Event::Write(token)).is_ok();
        }));
    }

    pub fn write(&mut self) {
        let mut data = self.writer.lock().unwrap();

        match self.socket.write(&data) {
            Ok(_) => {
                data.clear();
            },
            Err(_) => {
                self.closing = true;
                return;
            }
        }
    }
}
