use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::ToSocketAddrs;
use std::io::{self, ErrorKind};
use std::sync::mpsc::TryRecvError;
use std::rc::Rc;

use rustls;

use soio::Token;
use soio::Poll;
use soio::Ready;
use soio::PollOpt;
use soio::channel::{self, Receiver, Sender};
use soio::Events;
use soio::Event;
use soio::tcp::TcpListener;
use soio::Evented;

use threading::Pool;

use self::connection::Connection;
pub use self::stream::Stream;
use error::Result;

pub mod connection;
pub mod stream;

const SERVER: Token = Token(0);
const CHANNEL: Token = Token(1);

pub type Handle = Box<Fn(Arc<Mutex<Stream>>) + Send + Sync + 'static>;

pub struct Server {
    listener: TcpListener,
    conns: HashMap<Token, Connection>,
    token: usize,
    handle: Arc<Handle>,
    poll: Poll,
    events: Events,
    thread_pool: Rc<Pool>,
    tx: Sender<connection::Event>,
    rx: Receiver<connection::Event>,
    run: bool,
    tls_config: Option<Arc<rustls::ServerConfig>>,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Server> {
        let (tx, rx) = channel::channel::<connection::Event>();

        Ok(Server {
            listener: TcpListener::bind(addr)?,
            conns: HashMap::with_capacity(1024),
            token: 4,
            handle: Arc::new(Box::new(|_| {})),
            poll: Poll::new().unwrap(),
            events: Events::with_capacity(1024),
            thread_pool: Rc::new(Pool::new()),
            tx: tx,
            rx: rx,
            run: true,
            tls_config: None,
        })
    }

    fn token(&mut self) -> Token {
        self.token += 1;
        Token(self.token)
    }

    fn accept(&mut self) -> Result<()> {
        let (socket, _) = self.listener.accept()?;

        let thread_pool = self.thread_pool.clone();
        let tx = self.tx.clone();
        let token = self.token();

        let handle = self.handle.clone();

        self.poll.register(
            &socket, token,
            Ready::readable() | Ready::hup(),
            PollOpt::edge() | PollOpt::oneshot()
        )?;

        match self.tls_config {
            Some(ref tls_config) => {
                let tls_session = rustls::ServerSession::new(tls_config);
                self.conns.insert(
                    token,
                    Connection::new(socket, token, thread_pool, tx, handle, Some(tls_session))
                );
            },
            None => {
                self.conns.insert(
                    token,
                    Connection::new(socket, token, thread_pool, tx, handle, None)
                );
            },
        }

        Ok(())
    }

    fn channel(&mut self) -> Result<()> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => {
                    match event {
                        connection::Event::Write(token) => {
                            if let Some(conn) = self.conns.get(&token) {
                                conn.reregister(
                                    &self.poll, token,
                                    Ready::writable() | Ready::hup(),
                                    PollOpt::edge() | PollOpt::oneshot()
                                )?;
                            }
                        },
                        connection::Event::Read(token) => {
                            if let Some(conn) = self.conns.get(&token) {
                                conn.reregister(
                                    &self.poll, token,
                                    Ready::readable() | Ready::hup(),
                                    PollOpt::edge() | PollOpt::oneshot()
                                )?;
                            }
                        },
                        connection::Event::WriteTls(token) => {
                            if let Some(conn) = self.conns.get_mut(&token) {
                                conn.write_to_tls();
                            }
                        },
                    }
                },
                Err(err) => {
                    match err {
                        TryRecvError::Empty => break,
                        TryRecvError::Disconnected => return Err(
                            io::Error::new(ErrorKind::ConnectionAborted, err).into()
                        ),
                    }
                }
            }
        }

        Ok(())
    }

    fn connect(&mut self, event: Event ,token: Token) -> Result<()> {

        if event.readiness().is_error() {
           if let Some(conn) = self.conns.remove(&token) {
               conn.deregister(&self.poll)?;
                return Ok(())
            }
        }

        if event.readiness().is_readable() {
            let mut close = false;

            if let Some(mut conn) = self.conns.get_mut(&token) {
                conn.reader();
                close = conn.closing;
            }

            if close {             
                if let Some(conn) = self.conns.remove(&token) {
                    conn.deregister(&self.poll)?;
                    conn.shutdown();
                }             
            } 
        }

        if event.readiness().is_writable() {
            let mut close = false;

            if let Some(mut conn) = self.conns.get_mut(&token) {
                conn.writer();
                close =  conn.closing;
            }

            if close {
                if let Some(conn) = self.conns.remove(&token) {
                    conn.deregister(&self.poll)?;
                    conn.shutdown();
                }
            }
        }

        Ok(())
    }

    pub fn event(&mut self, event: Event) -> Result<()> {
        match event.token() {
            SERVER => {
                self.accept()
            },
            CHANNEL => {
                self.channel()
            },
            token => {
                self.connect(event, token)
            }
        }
    }

    pub fn run_once(&mut self) -> Result<()> {
        let size = self.poll.poll(&mut self.events, None)?;

        for i in 0..size {
            let event = self.events.get(i).unwrap();
            self.event(event)?;
        }

        Ok(())
    }

    pub fn run(&mut self, handle: Handle) -> Result<()> {
        self.handle = Arc::new(handle);

        self.poll.register(&self.listener, SERVER, Ready::readable(), PollOpt::level())?;

        self.poll.register(&self.rx, CHANNEL, Ready::readable(), PollOpt::level())?;

        self.run = true;

        while self.run {
            self.run_once()?;
        }

        Ok(())
    }

    pub fn run_tls(&mut self, handle: Handle, cert: &str, private_key: &str) -> Result<()> {
        self.tls_config = Some(make_config(cert, private_key));

        self.handle = Arc::new(handle);

        self.poll.register(&self.listener, SERVER, Ready::readable(), PollOpt::level())?;

        self.poll.register(&self.rx, CHANNEL, Ready::readable(), PollOpt::level())?;

        self.run = true;

        while self.run {
            self.run_once()?;
        }

        Ok(())
    }
}


use std::fs;
use std::io::BufReader;

fn load_certs(filename: &str) -> Vec<rustls::Certificate> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls::internal::pemfile::certs(&mut reader).unwrap()
}

fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let keyfile = fs::File::open(filename).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);
    let keys = rustls::internal::pemfile::rsa_private_keys(&mut reader).unwrap();
    assert!(keys.len() == 1);
    keys[0].clone()
}

fn make_config(cert: &str, private_key: &str) -> Arc<rustls::ServerConfig> {
    let cert = load_certs(cert);
    let privkey = load_private_key(private_key);

    let mut config = rustls::ServerConfig::new();
    config.set_single_cert(cert, privkey);


    Arc::new(config)
}
