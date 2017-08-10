use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::usize;
use std::rc::Rc;

use soio::tcp::TcpStream;
use soio::channel::{channel, Receiver, Sender};
use soio::{Poll, Ready, PollOpt, Events, Token};

use rustls;

use threading::Pool;

use error::Result;

use super::Handle;
use super::Connection;

const CHANNEL: Token = Token(usize::MAX - 1);
const CHANNEL2: Token = Token(usize::MAX - 2);

pub enum Event {
    Write(Token),
    WriteTls(Token)
}

pub struct Worker {
    rx: Receiver<TcpStream>,
    sockets: Vec<Option<Connection>>,
    active: Arc<AtomicUsize>,
    handle: Arc<Handle>,
    tls_config: Option<Arc<rustls::ServerConfig>>,
    pool: Rc<Pool>,
    event_rx: Receiver<Event>,
    event_tx: Sender<Event>
}

impl Worker {
    pub fn new(
        rx: Receiver<TcpStream>,
        active: Arc<AtomicUsize>,
        handle: Arc<Handle>,
        tls_config: Option<Arc<rustls::ServerConfig>>
    ) -> Worker {
        let mut sockets = Vec::new();

        for _ in 0..1024 {
            sockets.push(None);
        }

        let (event_tx, event_rx) = channel();

        Worker {
            rx: rx,
            sockets: sockets,
            active: active,
            handle: handle,
            tls_config: tls_config,
            pool: Rc::new(Pool::with_capacity(1, 8)),
            event_rx: event_rx,
            event_tx: event_tx
        }
    }

    pub fn run(&mut self) -> Result<()> {

        let poll = Poll::new()?;

        poll.register(&self.rx, CHANNEL, Ready::readable(), PollOpt::level())?;
        poll.register(&self.event_rx, CHANNEL2, Ready::readable(), PollOpt::level())?;

        let mut events = Events::with_capacity(1024);

        loop {
            poll.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    CHANNEL => {

                        let socket = self.rx.try_recv()?;

                        let mut index = 0;

                        for (k, v) in self.sockets.iter().enumerate() {
                            if v.is_none() {
                                index = k;
                                break;
                            }
                        }

                        poll.register(
                            &socket,
                            Token(index),
                            Ready::readable(),
                            PollOpt::edge() | PollOpt::oneshot()
                        )?;

                        let conn = match self.tls_config {
                            Some(ref tls_config) => {
                                let tls_session = rustls::ServerSession::new(tls_config);
                                let pool = self.pool.clone();
                                let event_tx = self.event_tx.clone();
                                Connection::new(socket, self.handle.clone(), Some(tls_session), pool, event_tx, Token(index))
                            }
                            None => {
                                let pool = self.pool.clone();
                                let event_tx = self.event_tx.clone();
                                Connection::new(socket, self.handle.clone(), None, pool, event_tx, Token(index))
                            }
                        };

                        if let Some(find) = self.sockets.get_mut(index) {
                            if find.is_some() {
                                panic!("bug");
                            }

                            *find = Some(conn);
                            self.active.fetch_add(1, Ordering::Release);
                        } else {
                            panic!("bug");
                        }

                    }
                    CHANNEL2 => {

                        let event = self.event_rx.try_recv()?;

                        let mut close = false;

                        let token = match event {
                            Event::Write(token) => {
                                if let &mut Some(ref mut conn) = self.sockets.get_mut::<usize>(token.into()).unwrap() {
                                    if self.tls_config.is_some() {
                                        panic!("bug");
                                    } else {
                                        conn.write();
                                    }

                                    close = conn.close;

                                    if !close {
                                        poll.reregister(
                                            &conn.socket,
                                            token,
                                            conn.interest,
                                            PollOpt::edge() | PollOpt::oneshot()
                                        )?;
                                    }
                                }

                                token
                            }
                            Event::WriteTls(token) => {
                                if let &mut Some(ref mut conn) = self.sockets.get_mut::<usize>(token.into()).unwrap() {
                                    if self.tls_config.is_some() {
                                        conn.write_to_tls();
                                    } else {
                                        panic!("bug");
                                    }

                                    close = conn.close;

                                    if !close {
                                        poll.reregister(
                                            &conn.socket,
                                            token,
                                            conn.interest,
                                            PollOpt::edge() | PollOpt::oneshot()
                                        )?;
                                    }
                                }

                                token
                            }
                        };

                        if close {
                            if let Some(find) = self.sockets.get_mut::<usize>(token.into()) {
                                if let Some(ref conn) = *find {
                                    poll.deregister(&conn.socket)?;
                                }

                                *find = None;
                                self.active.fetch_sub(1, Ordering::Release);
                            }
                        }
                    }
                    token => {
                        let mut close = false;

                        if event.readiness().is_error() {
                            close = true;
                        }
                            
                        if event.readiness().is_readable() {
                            if let &mut Some(ref mut conn) = self.sockets.get_mut::<usize>(token.into()).unwrap() {
                                if self.tls_config.is_some() {
                                    conn.read_tls();
                                } else {
                                    conn.read();
                                }
                                    
                                close = conn.close;

                                if !close && conn.handshake {
                                    poll.reregister(
                                        &conn.socket,
                                        token,
                                        conn.interest,
                                        PollOpt::edge() | PollOpt::oneshot()
                                    )?;
                                }
                            }
                        }

                        if event.readiness().is_writable() {
                            if let &mut Some(ref mut conn) = self.sockets.get_mut::<usize>(token.into()).unwrap() {
                                if self.tls_config.is_some() {
                                    conn.write_tls();
                                } else {
                                    panic!("bug");
                                }

                                close = conn.close;

                                if !close && self.tls_config.is_some() {
                                    poll.reregister(
                                        &conn.socket,
                                        token,
                                        conn.interest,
                                        PollOpt::edge() | PollOpt::oneshot()
                                    )?;
                                }
                            }
                        }

                        if close {
                            if let Some(find) = self.sockets.get_mut::<usize>(token.into()) {
                                if let Some(ref conn) = *find {
                                    poll.deregister(&conn.socket)?;
                                }

                                *find = None;
                                self.active.fetch_sub(1, Ordering::Release);
                            }
                        }
                    }
                }
            }
        }
    }
}
