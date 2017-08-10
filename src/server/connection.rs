use std::io::{Read, Write};
use std::io::ErrorKind::WouldBlock;
use std::sync::{Arc, Mutex};

use soio::tcp::TcpStream;
use soio::Ready;

use rustls::{self, Session};

use super::Handle;
use super::Stream;

pub struct Connection {
    pub socket: TcpStream,
    handle: Arc<Handle>,
    stream: Arc<Mutex<Stream>>,
    pub interest: Ready,
    tls_session: Option<rustls::ServerSession>,
    pub close: bool,
}

impl Connection {
    pub fn new(
        socket: TcpStream,
        handle: Arc<Handle>,
        tls_session: Option<rustls::ServerSession>
    ) -> Connection {
        Connection {
            socket: socket,
            handle: handle,
            stream: Arc::new(Mutex::new(
                Stream::new(Vec::with_capacity(1024), Vec::with_capacity(1024))
                )),
            interest: Ready::empty(),
            tls_session: tls_session,
            close: false,
        }
    }

    pub fn read(&mut self) {
        {
            let mut stream = self.stream.lock().unwrap();

            if !stream.reader.is_empty() {
                stream.reader.clear();
            }

            loop {
                let mut buf = [0; 1024];

                match self.socket.read(&mut buf) {
                    Ok(size) => {
                        if size == 0 {
                            self.close = true;
                            return;
                        }

                        stream.reader.extend_from_slice(&buf[0..size]);

                        if size < 1024 {
                            break;
                        }
                    }
                    Err(err) => {
                        if let WouldBlock = err.kind() {
                            break;
                        } else {
                            self.close = true;
                            return;
                        }
                    }
                }
            }

            stream.remote_addr = self.socket.peer_addr().unwrap();
        }

        let stream = self.stream.clone();

        (self.handle)(stream);

        self.interest.remove(Ready::readable());
        self.interest.insert(Ready::writable());
    }

    pub fn read_tls(&mut self) {
        if let Some(ref mut tls_session) = self.tls_session {
            match tls_session.read_tls(&mut self.socket) {
                Ok(size) => {
                    if size == 0 {
                        self.close = true;
                    }
                }
                Err(err) => {
                    if let WouldBlock = err.kind() {
                        //return;
                    } else {
                        self.close = true;
                        return;
                    }
                }
            }

            if tls_session.process_new_packets().is_err() {
                self.close = true;
                return;
            }

            let next = {
                let mut stream = self.stream.lock().unwrap();

                stream.remote_addr = self.socket.peer_addr().unwrap();

                if !stream.reader.is_empty() {
                    stream.reader.clear();
                }

                if tls_session.read_to_end(&mut stream.reader).is_err() {
                    self.close = true;
                    return;
                }

                !stream.reader.is_empty()
            };

            if next {
                let stream = self.stream.clone();
                (self.handle)(stream);
            }

            if next {
                let ref mut writer = self.stream.lock().unwrap().writer;

                tls_session.write_all(writer).unwrap();
            }

            let rd = tls_session.wants_read();
            let wr = tls_session.wants_write();

            self.interest.remove(Ready::readable() | Ready::writable());

            if rd && wr {
                self.interest.insert(Ready::readable());
                self.interest.insert(Ready::writable());
            } else if wr {
                self.interest.insert(Ready::writable());
            } else {
                self.interest.insert(Ready::readable());
            }

            if self.interest.is_empty() {
                panic!("bug");
            }

        } else {
            panic!("bug");
        }
    }

    pub fn write(&mut self) {
        let ref mut writer = self.stream.lock().unwrap().writer;

        match self.socket.write(writer) {
            Ok(size) => {
                if size == 0 {
                    self.close = true;
                    return;
                }

                writer.clear();
            }
            Err(_) => {
                self.close = true;
                return;
            }
        }

        self.interest.remove(Ready::writable());
        self.interest.insert(Ready::readable());
    }

    pub fn write_tls(&mut self) {
        if let Some(ref mut tls_session) = self.tls_session {
            match tls_session.write_tls(&mut self.socket) {
                Ok(size) => {
                    if size == 0 {
                        self.close = true;
                        return;
                    }
                }
                Err(_) => {
                    self.close = true;
                    return;
                }
            }

            let rd = tls_session.wants_read();
            let wr = tls_session.wants_write();

            self.interest.remove(Ready::readable() | Ready::writable());

            if rd && wr {
                self.interest.insert(Ready::readable());
                self.interest.insert(Ready::writable());
            } else if wr {
                self.interest.insert(Ready::writable());
            } else {
                self.interest.insert(Ready::readable());
            }

            if self.interest.is_empty() {
                panic!("bug");
            }

        } else {
            panic!("bug");
        }
    }
}
