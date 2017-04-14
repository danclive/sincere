use std::sync::{Arc, Mutex};
use std::io::{self, Read, Write};
use std::io::ErrorKind::WouldBlock;
use std::net::Shutdown;
use std::rc::Rc;

use rustls;
use rustls::Session;

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
    Read(Token),
    WriteTls(Token),
}

pub struct Connection {
    socket: TcpStream,
    token: Token,
    thread_pool: Rc<Pool>,
    tx: channel::Sender<Event>,
    stream: Arc<Mutex<Stream>>,
    pub closing: bool,
    handle: Arc<Handle>,
    tls_session: Option<rustls::ServerSession>,
}

impl Connection {
    pub fn new(socket: TcpStream, token: Token, thread_pool: Rc<Pool>, tx: channel::Sender<Event>, handle: Arc<Handle>, tls_session: Option<rustls::ServerSession>) -> Connection {
        Connection {
            socket: socket,
            token: token,
            thread_pool: thread_pool,
            tx: tx,
            stream: Arc::new(Mutex::new(Stream::new(Vec::with_capacity(1024), Vec::with_capacity(1024)))),
            closing: false,
            handle: handle,
            tls_session: tls_session,
        }
    }

    pub fn reader(&mut self) {
        match self.tls_session {
            Some(ref mut tls_session) => {

                match tls_session.read_tls(&mut self.socket) {
                    Ok(size) => {
                        if size == 0 {
                            self.closing = true;
                            return;
                        }
                    },
                    Err(err) => {
                        if let WouldBlock = err.kind() {
                            return;
                        }

                        self.closing = true;
                        return;
                    }
                }

                if let Err(_) = tls_session.process_new_packets() {
                    self.closing = true;
                    return;
                }

                let mut stream = self.stream.lock().unwrap();

                stream.remote_addr = self.socket.peer_addr().unwrap();

                if let Err(_) = tls_session.read_to_end(&mut stream.reader) {
                    self.closing = true;
                    return;
                }

                if stream.len() != 0 {

                    let tx = self.tx.clone();
                    let token = self.token.clone();

                    let handle = self.handle.clone();

                    let stream = self.stream.clone();

                    self.thread_pool.spawn(move || {

                        handle(stream);

                        tx.send(Event::WriteTls(token)).is_ok();

                    });

                } else {
                    let rd = tls_session.wants_read();
                    let wr = tls_session.wants_write();

                    if rd && wr {
                        self.tx.send(Event::Read(self.token)).is_ok();
                        self.tx.send(Event::Write(self.token)).is_ok();
                    } else if wr {
                        self.tx.send(Event::Write(self.token)).is_ok();
                    } else {
                        self.tx.send(Event::Read(self.token)).is_ok();
                    }
                }


            },
            None => {

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


            },
        }
    }

    pub fn writer(&mut self) {
        match self.tls_session {
            Some(ref mut tls_session) => {
                match tls_session.write_tls(&mut self.socket) {
                    Ok(size) => {
                        if size == 0 {
                            self.closing = true;
                            return;
                        }
                    },
                    Err(_) => {
                        self.closing = true;
                        return;
                    }
                }

                let rd = tls_session.wants_read();
                let wr = tls_session.wants_write();

                if rd && wr {
                    self.tx.send(Event::Read(self.token)).is_ok();
                    self.tx.send(Event::Write(self.token)).is_ok();
                } else if wr {
                    self.tx.send(Event::Write(self.token)).is_ok();
                } else {
                    self.tx.send(Event::Read(self.token)).is_ok();
                }
            },
            None => {
                let ref mut writer = self.stream.lock().unwrap().writer;
        
                match self.socket.write(writer) {
                    Ok(size) => {
                        if size == 0 {
                            self.closing = true;
                            return;
                        }

                        writer.clear();
                    },
                    Err(_) => {
                        self.closing = true;
                        return;
                    }
                }

                self.tx.send(Event::Read(self.token)).is_ok();
            }
        }
    }

    pub fn write_to_tls(&mut self) {
        let ref mut writer = self.stream.lock().unwrap().writer;

        match self.tls_session {
            Some(ref mut tls_session) => {
                tls_session.write_all(writer).unwrap();

                let rd = tls_session.wants_read();
                let wr = tls_session.wants_write();

                if rd && wr {
                    self.tx.send(Event::Read(self.token)).is_ok();
                    self.tx.send(Event::Write(self.token)).is_ok();
                } else if wr {
                    self.tx.send(Event::Write(self.token)).is_ok();
                } else {
                    self.tx.send(Event::Read(self.token)).is_ok();
                }
            },
            None => ()
        }

        writer.clear();

    }

    pub fn shutdown(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
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
