use std::sync::{Arc, Mutex};
use std::io::{self, Read, Write};
use std::io::ErrorKind::WouldBlock;

use soio::Token;
use soio::tcp::TcpStream;
use soio::channel;
use soio::Evented;
use soio::Poll;
use soio::Ready;
use soio::PollOpt;

use threading::Pool;

use super::Handle;
use super::stream::Stream;

pub enum Event {
    Close(Token),
    Write(Token),
    Read(Token),
}

pub struct Connection {
    socket: TcpStream,
    token: Token,
    thread_pool: Pool,
    tx: channel::Sender<Event>,
    writer: Arc<Mutex<Vec<u8>>>,
    pub closing: bool,
    handle: Arc<Handle>,
}

impl Connection {
    pub fn new(socket: TcpStream, token: Token, thread_pool: Pool, tx: channel::Sender<Event>, handle: Arc<Handle>) -> Connection {
        Connection {
            socket: socket,
            token: token,
            thread_pool: thread_pool,
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

        self.thread_pool.spawn(move || {
            handle(Stream::new(reader, writer, remote_addr));
            tx.send(Event::Write(token)).is_ok();
        });
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

impl Evented for Connection {
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
         -> io::Result<()>
    {
        self.socket.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
         -> io::Result<()>
    {
        self.socket.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.socket.deregister(poll)
    }
}
