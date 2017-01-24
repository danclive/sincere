use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use mio::Token;
use mio::Poll;
use mio::Ready;
use mio::PollOpt;
use mio::channel;
use mio::Events;
use mio::tcp::TcpListener;

use util::TaskPool;

use self::connection::Connention;
pub use self::stream::Stream;

pub mod connection;
pub mod stream;

pub type Handle = Box<Fn(Stream) + Send + Sync + 'static>;

pub struct Server {
    conns: HashMap<Token, Connention>,
    token: usize,
    handle: Arc<Handle>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            conns: HashMap::new(),
            token: 3,
            handle: Arc::new(Box::new(|_| {})),
        }
    }

    fn token(&mut self) -> Token {
        if self.token >= usize::max_value() - 4 {
            self.token = 4;
        } else {
            self.token += 1;
        }

        Token(self.token)
    }

    pub fn event_loop(&mut self, listener: TcpListener) -> Result<(), Box<Error + Send + Sync>> {
        const SERVER: Token = Token(0);
        const CHANNEL: Token = Token(1);

        let poll = Poll::new()?;

        poll.register(&listener, SERVER, Ready::readable(), PollOpt::edge())?;

        let (tx, rx) = channel::channel::<connection::Event>();

        poll.register(&rx, CHANNEL, Ready::readable(), PollOpt::edge())?;
        
        let mut events = Events::with_capacity(1024);

        let task_pool = TaskPool::new();

        loop {
            poll.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    SERVER => {
                        let (socket, _) = listener.accept()?;

                        let task_pool = task_pool.clone();
                        let tx = tx.clone();
                        let token = self.token();

                        let handle = self.handle.clone();

                        poll.register(&socket, token, Ready::readable(), PollOpt::edge() | PollOpt::oneshot())?;
                        self.conns.insert(token, Connention::new(socket, token, task_pool, tx, handle));
                    },
                    CHANNEL => {
                        loop {
                            match rx.try_recv() {
                                Ok(event) => {
                                    match event {
                                        connection::Event::Close(token) => {
                                            if let Some(conn) = self.conns.remove(&token) {
                                                poll.deregister(&conn.socket)?;
                                            }
                                        }, 
                                        connection::Event::Write(token) => {
                                            if let Some(conn) = self.conns.get(&token) {
                                                poll.reregister(&conn.socket, token, Ready::writable(), PollOpt::edge() | PollOpt::oneshot())?;
                                            }
                                        },
                                        connection::Event::Read(token) => {
                                            if let Some(conn) = self.conns.get(&token) {
                                                poll.reregister(&conn.socket, token, Ready::readable(), PollOpt::edge() | PollOpt::oneshot())?;
                                            }
                                        }
                                    }
                                },
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                    },
                    token => {
                        if event.kind().is_hup() || event.kind().is_error() {
                            if let Some(conn) = self.conns.remove(&token) {
                                poll.deregister(&conn.socket)?;
                            }
                        }

                        if event.kind().is_readable() {
                            let mut close = false;

                            if let Some(mut conn) = self.conns.get_mut(&token) {
                                conn.read();

                                close = conn.closing;

                            }

                            if close {
                                if let Some(conn) = self.conns.remove(&token) {
                                    poll.deregister(&conn.socket)?;
                                }
                            }
                        }

                        if event.kind().is_writable() {
                            let mut close = false;

                            if let Some(mut conn) = self.conns.get_mut(&token) {
                                conn.write();

                                close =  conn.closing;
                            }

                            if close {
                                if let Some(conn) = self.conns.remove(&token) {
                                    poll.deregister(&conn.socket)?;
                                }
                            } else {
                                if let Some(conn) = self.conns.get(&token) {
                                    poll.reregister(&conn.socket, token, Ready::readable(), PollOpt::edge() | PollOpt::oneshot())?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle(&mut self, handle: Handle) {
        self.handle = Arc::new(handle);
    }

    pub fn run(&mut self, addr: &str) -> Result<(), Box<Error + Send + Sync>> {

        let addr = addr.parse().expect("cannot paser addr");
        let listener = TcpListener::bind(&addr).expect("cannot listen on port");

        self.event_loop(listener)?;

        Ok(())
    }
}
