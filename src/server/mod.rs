use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::usize;

use soio::tcp::TcpListener;
use soio::tcp::TcpStream;
use soio::{Events, Poll, Token, Ready, PollOpt};

use rustls;

use error::Result;

pub use self::stream::Stream;
use self::process::Process;
use self::worker::Worker;
use self::connection::Connection;

pub type Handle = Box<Fn(Arc<Mutex<Stream>>) + Send + Sync + 'static>;

mod stream;
mod process;
mod worker;
mod connection;

const SERVER: Token = Token(0);

pub struct Server {
    listener: TcpListener,
    events: Events,
    poll: Poll,
    process: Vec<Process>,
    run: bool,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Server> {
        Ok(Server {
            listener: TcpListener::bind(addr)?,
            events: Events::with_capacity(1024),
            poll: Poll::new()?,
            process: Vec::new(),
            run: true,
        })
    }

    pub fn run(&mut self, handle: Handle, process_num: usize) -> Result<()> {

        let handle = Arc::new(handle);

        for _ in 0..process_num {
            self.process.push(Process::new(handle.clone(), None)?);
        }

        self.poll.register(
            &self.listener,
            SERVER,
            Ready::readable(),
            PollOpt::level()
        )?;

        while self.run {
            self.run_once()?;
        }

        Ok(())
    }

    pub fn run_tls(&mut self, handle: Handle, process_num: usize, cert: &str, private_key: &str) -> Result<()> {
        let tls_config = make_config(cert, private_key);

        let handle = Arc::new(handle);

        for _ in 0..process_num {
            self.process.push(Process::new(handle.clone(), Some(tls_config.clone()))?);
        }

        self.poll.register(
            &self.listener,
            SERVER,
            Ready::readable(),
            PollOpt::level()
        )?;

        while self.run {
            self.run_once()?;
        }

        Ok(())
    }

    pub fn run_once(&mut self) -> Result<()> {
        self.poll.poll(&mut self.events, None)?;

        for event in self.events.iter() {
            if event.token() == SERVER && event.readiness() == Ready::readable() {
                let (socket, _) = self.listener.accept()?;
                self.dispense(socket)?;
            }
        }

        Ok(())
    }

    fn dispense(&self, socket: TcpStream) -> Result<()> {
        let mut pro: Vec<(usize, usize)> = self.process.iter().enumerate().map(|(n, ref p)| { (n, p.active()) }).collect();
        pro.sort_by(|&(_, a), &(_, b)| a.cmp(&b));

        let (nim, _) = pro[0];

        self.process.get(nim).expect("don't send").send(socket)?;

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
