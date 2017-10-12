use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::usize;

use soio::tcp::TcpListener;
use soio::tcp::TcpStream;
use soio::{Events, Poll, Token, Ready, PollOpt};

use error::Result;

pub use self::stream::Stream;
use self::process::Process;
use self::worker::Worker;
use self::connection::Connection;

pub type Handle = Box<Fn(&mut Stream) + Send + Sync>;

mod stream;
mod process;
mod worker;
mod connection;
mod tlsconfig;

const SERVER: Token = Token(0);

pub struct Server {
    listener: TcpListener,
    events: Events,
    poll: Poll,
    process: Vec<Process>,
    process_num: usize,
    process_pos: usize,
    run: bool,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Server> {
        Ok(Server {
            listener: TcpListener::bind(addr)?,
            events: Events::with_capacity(1024),
            poll: Poll::new()?,
            process: Vec::new(),
            process_num: 0,
            process_pos: 0,
            run: true,
        })
    }

    pub fn run(&mut self, handle: Handle, process_num: usize) -> Result<()> {
        self.process_num = process_num;

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
        let tls_config = tlsconfig::TlsConfig::new(cert, private_key).make_config();
        self.process_num = process_num;

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
        let events_num = self.poll.poll(&mut self.events, None)?;

        for i in 0..events_num {
            let event = self.events.get(i).unwrap();

            if event.token() == SERVER && event.readiness() == Ready::readable() {
                let (socket, _) = self.listener.accept()?;
                self.dispense(socket)?;
            }
        }

        Ok(())
    }

    fn dispense(&mut self, socket: TcpStream) -> Result<()> {

        self.process.get(self.process_pos).expect("bug!").send(socket)?;

        self.process_pos += 1;

        if self.process_pos >= self.process_num {
            self.process_pos = 0;
        }

        Ok(())
    }
}
