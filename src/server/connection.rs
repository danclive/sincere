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
    Write(Token),
}

pub struct Connection {
    socket: TcpStream,
    token: Token,
    thread_pool: Pool,
    tx: channel::Sender<Event>,
    stream: Arc<Mutex<Stream>>,
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
            stream: Arc::new(Mutex::new(Stream::new(Vec::with_capacity(1024), Vec::with_capacity(1024)))),
            closing: false,
            handle: handle,
        }
    }

    pub fn read(&mut self) {

        let mut stream = self.stream.lock().unwrap();
        
        loop {

            let mut buf = [0; 1024];

            match self.socket.read(&mut buf) {
                Ok(size) => {
                    if size == 0 {
                        self.closing = true;
                        return;
                    } else {
                        stream.reader.extend_from_slice(&buf[0..size]);
                    }
                },
                Err(err) => {
                    if let WouldBlock = err.kind() {
                        break;
                    } else {
                        self.closing = true;
                        return;
                    }
                }
            }

        }

        stream.remote_addr = self.socket.peer_addr().unwrap();

        let tx = self.tx.clone();
        let token = self.token.clone();

        let handle = self.handle.clone();

        let stream = self.stream.clone();

        self.thread_pool.spawn(move || {

            handle(stream);

            tx.send(Event::Write(token)).is_ok();
        });
    }

    pub fn write(&mut self) {
        let ref mut writer = self.stream.lock().unwrap().writer;
        
        match self.socket.write(writer) {
            Ok(_) => {
                writer.clear();
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
