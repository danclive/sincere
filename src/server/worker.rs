use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::usize;

use soio::tcp::TcpStream;
use soio::channel::Receiver;
use soio::{Poll, Ready, PollOpt, Events, Token};

use rustls;

use error::Result;

use super::Handle;
use super::Connection;

const CHANNEL: Token = Token(usize::MAX - 1);

pub struct Worker {
    rx: Receiver<TcpStream>,
    sockets: Vec<Option<Connection>>,
    active: Arc<AtomicUsize>,
    handle: Arc<Handle>,
    tls_config: Option<Arc<rustls::ServerConfig>>,
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

        Worker {
            rx: rx,
            sockets: sockets,
            active: active,
            handle: handle,
            tls_config: tls_config,
        }
    }

    pub fn run(&mut self) -> Result<()> {

        let poll = Poll::new()?;

        poll.register(&self.rx, CHANNEL, Ready::readable(), PollOpt::level())?;

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
                                Connection::new(socket, self.handle.clone(), Some(tls_session))
                            }
                            None => {
                                Connection::new(socket, self.handle.clone(), None)
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

                                if !close {
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
