use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write, Cursor, BufRead};
use std::marker::{Send, Sync};
use std::mem;
use std::net::TcpListener;
use std::rc::Rc;
use std::sync::Arc;
use std::{u16, u32};
use std::cmp;
use std::env;

use util::thread_pool::ThreadPool;
use self::sys::{Transport, Socket};

mod sys;

const HEADER_LEN: usize = 8;

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Responder,
    Authorizer,
    Filter,
}

#[derive(Debug)]
enum ProtocolStatus {
    RequestComplete,
    CantMpxConn,
    #[allow(dead_code)] Overloaded,
    UnknownRole,
}

#[derive(Debug)]
enum Record {
    BeginRequest { request_id: u16, role: Result<Role, u16>, keep_conn: bool },
    AbortRequest { request_id: u16 },
    EndRequest { request_id: u16, app_status: i32, protocol_status: ProtocolStatus },
    Params { request_id: u16, content: Vec<u8> },
    Stdin { request_id: u16, content: Vec<u8> },
    Stdout { request_id: u16, content: Vec<u8> },
    Stderr { request_id: u16, content: Vec<u8> },
    Data { request_id: u16, content: Vec<u8> },
    GetValues(Vec<String>),
    GetValuesResult(Vec<(String, String)>),
    UnknownType(u8),
}

fn read_len<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf: Vec<u8> = Vec::with_capacity(4);

    r.take(1).read_to_end(&mut buf)?;

    if buf.len() == 1 {
        if buf[0] >> 7 == 1 {
            assert!(r.take(3).read_to_end(&mut buf)? == 3);

            Ok(
                (((buf[0] & 0x7f) as u32) << 24)
                + ((buf[1] as u32) << 16)
                + ((buf[2] as u32) << 8)
                + (buf[3] as u32)
            )
        } else {
            Ok(buf[0] as u32)
        }
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "EOF"))
    }
}

fn read_pair<R: Read>(r: &mut R) -> io::Result<(String, String)> {
    let key_len = read_len(r)?;
    let value_len = read_len(r)?;

    let mut key = String::with_capacity(key_len as usize);

    assert!(r.take(key_len as u64).read_to_string(&mut key)? == key_len as usize);

    let mut value = String::with_capacity(value_len as usize);

    assert!(r.take(value_len as u64).read_to_string(&mut value)? == value_len as usize);

    Ok((key, value))
}

fn read_pairs<R: Read>(r: &mut R) -> io::Result<Vec<(String, String)>> {
    let mut params = Vec::new();

    match read_pair(r) {
        Ok(param) => {
            params.push(param);
            params.extend(read_pairs(r)?.into_iter());

            Ok(params)
        },
        Err(_) => Ok(params),
    }
}

fn write_len<W: Write>(w: &mut W, n: u32) -> io::Result<()> {
    if n < 0x80 {
        w.write_all(&[n as u8])?;
    } else {
        assert!(n < 0x80000000);

        let buf = unsafe {
            mem::transmute::<u32, [u8; 4]>((0x80000000 & n).to_be())
        };

        w.write_all(&buf)?;
    }
    Ok(())
}

fn write_pairs<W: Write>(w: &mut W, pairs: Vec<(String, String)>) -> io::Result<()> {
    for (key, value) in pairs {
        write_len(w, key.len() as u32)?;
        write_len(w, value.len() as u32)?;
        write!(w, "{}{}", key, value)?;
    }
    Ok(())
}

#[inline]
fn write_record<W: Write>(w: &mut W, record_type: u8, request_id: u16, content: &[u8]) -> io::Result<()> {
    assert!(content.len() <= u32::MAX as usize);

    let request_id = unsafe {
        mem::transmute::<_, [u8; 2]>(request_id.to_be())
    };

    let content_length = unsafe {
        mem::transmute::<_, [u8; 2]>((content.len() as u16).to_be())
    };

    w.write_all(&[
        1, record_type, request_id[0], request_id[1],
        content_length[0], content_length[1], 0, 0,
    ])?; // TODO: Padding

    w.write_all(content)?;

    Ok(())
}

#[inline]
fn read_record<R: Read>(r: &mut R) -> io::Result<(u8, u16, Vec<u8>)> {
    let mut header: Vec<u8> = Vec::with_capacity(HEADER_LEN);

    assert!(r.take(HEADER_LEN as u64).read_to_end(&mut header)? == HEADER_LEN);

    assert!(header[0] == 1);

    let record_type = header[1];
    let request_id = unsafe { u16::from_be(mem::transmute([header[2], header[3]])) };
    let content_length = unsafe { u16::from_be(mem::transmute([header[4], header[5]])) };
    let padding_length = header[6];

    let mut content: Vec<u8> = Vec::with_capacity(content_length as usize);

    assert!(r.take(content_length as u64).read_to_end(&mut content)? == content_length as usize);
    assert!(r.take(padding_length as u64).read_to_end(&mut Vec::with_capacity(padding_length as usize))? == padding_length as usize);

    Ok((record_type, request_id, content))
}

fn get_values(keys: Vec<String>) -> Vec<(String, String)> {
    keys.into_iter().filter_map(|key|
        match key.as_ref() {
            "FCGI_MAX_CONNS" => Some((key, "1".to_owned())),
            "FCGI_MAX_REQS" => Some((key, "1".to_owned())),
            "FCGI_MPXS_CONNS" => Some((key, "0".to_owned())),
            _ => None,
        }
    ).collect()
}

impl Record {
    fn send<W: Write>(self, w: &mut W) -> io::Result<()> {
        match self {
            Record::EndRequest { request_id, app_status, protocol_status } => {
                let app_status = unsafe {
                    mem::transmute::<_, [u8; 4]>(app_status.to_be())
                };

                let protocol_status = match protocol_status {
                    ProtocolStatus::RequestComplete => 0,
                    ProtocolStatus::CantMpxConn => 1,
                    ProtocolStatus::Overloaded => 2,
                    ProtocolStatus::UnknownRole => 3,
                };

                let content = [
                    app_status[0], app_status[1], app_status[2], app_status[3],
                    protocol_status, 0, 0, 0,
                ];

                write_record(w, 3, request_id, &content)?;
            }
            Record::Stdout { request_id, content } => {
                write_record(w, 6, request_id, &content)?;
            }
            Record::Stderr { request_id, content } => {
                write_record(w, 7, request_id, &content)?;
            }
            Record::GetValuesResult(items) => {
                let mut content = Cursor::new(Vec::new());
                write_pairs(&mut content, items)?;
                write_record(w, 10, 0, &content.into_inner())?;
            }
            Record::UnknownType(record_type) => {
                let content = [record_type, 0, 0, 0, 0, 0, 0, 0];
                write_record(w, 11, 0, &content)?;
            }
            _ => panic!("Record not sendable"),
        }
        Ok(())
    }

    fn receive<R: Read>(r: &mut R) -> io::Result<Self> {
        let (record_type, request_id, content) = read_record(r)?;
        let rec = match record_type {
            1 => {
                let role = unsafe {
                    u16::from_be(mem::transmute([content[0], content[1]]))
                };
                let role = match role {
                    1 => Ok(Role::Responder),
                    2 => Ok(Role::Authorizer),
                    3 => Ok(Role::Filter),
                    _ => Err(role),
                };
                let keep_conn = content[2] & 1 == 1;
                Record::BeginRequest {
                    request_id: request_id,
                    role: role,
                    keep_conn: keep_conn
                }
            }
            2 => Record::AbortRequest { request_id: request_id },
            4 => Record::Params { request_id: request_id, content: content },
            5 => Record::Stdin { request_id: request_id, content: content },
            8 => Record::Data { request_id: request_id, content: content },
            9 => {
                let items = try!(read_pairs(&mut Cursor::new(content)));
                Record::GetValues(items.into_iter().map(|(key, _)| key).collect())
            }
            _ if record_type >= 11 => Record::UnknownType(record_type),
            _ => panic!("Record not receivable"),
        };

        Ok(rec)
    }
}

pub struct Stdin<'a> {
    req: &'a mut Request,
}

impl<'a> Stdin<'a> {
    /// Begin reading the second stream of the request, for FastCGI Filter
    /// applications.
    ///
    /// May only be called after all contents of stdin has been read. Panics
    /// if stdin has not reached EOF yet.
    pub fn start_filter_data(&mut self) {
        if !self.req.filter_data {
            assert!(self.req.is_eof);
            self.req.is_eof = false;
            self.req.filter_data = true;
        }
    }
}

impl<'a> BufRead for Stdin<'a> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.req.aborted {
            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Request aborted"));
        }
        if self.req.pos == self.req.buf.len() && !self.req.is_eof {
            let mut sock = &*self.req.sock;
            loop {
                match (try!(Record::receive(&mut sock)), self.req.filter_data) {
                    (Record::UnknownType(rec_type), _) => {
                        try!(Record::UnknownType(rec_type).send(&mut sock));
                    },
                    (Record::GetValues(keys), _) => {
                        try!(
                            Record::GetValuesResult(get_values(keys))
                            .send(&mut sock)
                        );
                    },
                    (Record::BeginRequest { request_id, .. }, _) => {
                        try!(Record::EndRequest {
                                request_id: request_id,
                                app_status: 0,
                                protocol_status: ProtocolStatus::CantMpxConn,
                            }
                            .send(&mut sock)
                        );
                    },
                    (Record::AbortRequest { request_id }, _) => {
                        if request_id != self.req.id {
                            continue;
                        }
                        self.req.aborted = true;
                        return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Request aborted"));
                    }
                    (Record::Stdin { request_id, content }, false)
                    | (Record::Data { request_id, content }, true) => {
                        if request_id != self.req.id {
                            continue;
                        }
                        if content.is_empty() {
                            self.req.is_eof = true;
                        }
                        self.req.buf = content;
                        self.req.pos = 0;
                        break;
                    },
                    _ => (),
                }
            }
        }
        Ok(&self.req.buf[self.req.pos..])
    }

    fn consume(&mut self, amount: usize) {
        self.req.pos = cmp::min(self.req.pos + amount, self.req.buf.len());
    }
}

impl<'a> Read for Stdin<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = {
            let mut chunk = try!(self.fill_buf());
            try!(chunk.read(buf))
        };
        self.consume(n);
        Ok(n)
    }
}

macro_rules! writer {
    ($Writer:ident) => (
        pub struct $Writer<'a> {
            req: &'a mut Request,
        }

        impl<'a> Write for $Writer<'a> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                if self.req.aborted {
                    return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Request aborted"));
                }
                if buf.is_empty() {
                    Ok(0)
                } else {
                    for chunk in buf.chunks(u16::MAX as usize) {
                        let rec = Record::$Writer {
                            request_id: self.req.id,
                            content: chunk.to_owned(),
                        };
                        rec.send(&mut &*self.req.sock)?;
                    }
                    Ok(buf.len())
                }
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
    );
}

writer!(Stdout);

writer!(Stderr);



/// Request objects are what a FastCGI application will primarily deal with
/// throughout its lifetime.
///
/// The Request API is designed to be an abstraction of the traditional CGI
/// process model. Note that this API is low level. Dealing with things like
/// GET/POST parameters or cookies is outside the scope of this library.
pub struct Request {
    sock: Rc<Socket>,
    id: u16,
    role: Role,
    params: HashMap<String, String>,
    aborted: bool,
    status: i32,
    buf: Vec<u8>,
    pos: usize,
    is_eof: bool,
    filter_data: bool,
}

pub type Params<'a> = Box<Iterator<Item = (String, String)> + 'a>;

impl Request {
    fn begin(mut sock: &Socket) -> io::Result<(u16, Role, bool)> {
        loop {
            match Record::receive(&mut sock)? {
                Record::UnknownType(rec_type) => {
                    Record::UnknownType(rec_type).send(&mut sock)?;
                },
                Record::GetValues(keys) => {
                    Record::GetValuesResult(get_values(keys)).send(&mut sock)?;
                },
                Record::BeginRequest { request_id, role: Ok(role), keep_conn } => {
                    return Ok((request_id, role, keep_conn));
                },
                Record::BeginRequest { request_id, role: Err(_), .. } => {
                    Record::EndRequest {
                        request_id: request_id,
                        app_status: 0,
                        protocol_status: ProtocolStatus::UnknownRole
                    }.send(&mut sock)?;
                },
                _ => (),
            }
        }
    }

    fn new(sock: Rc<Socket>, id: u16, role: Role) -> io::Result<Self> {
        let mut buf = Vec::new();
        let mut params = HashMap::new();
        let mut aborted = false;
        loop {
            match Record::receive(&mut &*sock)? {
                Record::UnknownType(rec_type) => {
                    Record::UnknownType(rec_type).send(&mut &*sock)?;
                },
                Record::GetValues(keys) => {
                    Record::GetValuesResult(get_values(keys)).send(&mut &*sock)?;

                },
                Record::BeginRequest { request_id, .. } => {
                    Record::EndRequest {
                        request_id: request_id,
                        app_status: 0,
                        protocol_status: ProtocolStatus::CantMpxConn,
                    }.send(&mut &*sock)?;
                }
                Record::AbortRequest { request_id } => {
                    if id != request_id {
                        continue;
                    }
                    aborted = true;
                    break;
                }
                Record::Params { request_id, content } => {
                    if id != request_id {
                        continue;
                    }
                    if content.is_empty() {
                        params.extend(read_pairs(&mut Cursor::new(&buf))?);
                        break;
                    } else {
                        buf.extend(content);
                    }
                },
                _ => (),
            }
        }
        Ok(Request {
            sock: sock,
            id: id,
            role: role,
            params: params,
            aborted: aborted,
            status: 0,
            buf: Vec::new(),
            pos: 0,
            is_eof: false,
            filter_data: false,
        })
    }

    pub fn role(&self) -> Role {
        self.role
    }

    /// Retrieves the value of the given parameter name.
    pub fn param(&self, key: &str) -> Option<String> {
        self.params.get(key).map(|s| s.to_owned() )
    }

    /// FastCGI parameters.
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// Standard input stream of the request.
    pub fn stdin(&mut self) -> Stdin {
        Stdin { req: self }
    }

    /// Standard output stream of the request.
    pub fn stdout(&mut self) -> Stdout {
        Stdout { req: self }
    }

    /// Standard error stream of the request.
    pub fn stderr(&mut self) -> Stderr {
        Stderr { req: self }
    }

    /// Checks if the client has closed the connection prematurely.
    ///
    /// The reliability of this method depends on whether the web server
    /// notifies such event (by sending the `FCGI_REQUEST_ABORTED` record) to
    /// the FastCGI application. This value is updated synchronously; the
    /// update may only be triggered by reading from stdin.
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// Reports the specified exit code to the web server.
    ///
    /// This will consume the Request object. If you finish processing the
    /// Request object without calling `exit`, it is assumed that the exit code
    /// is 0.
    pub fn exit(mut self, code: i32) {
        self.status = code;
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        Record::Stdout {
            request_id: self.id,
            content: Vec::new(),
        }.send(&mut &*self.sock).unwrap_or(());
        Record::Stderr {
            request_id: self.id,
            content: Vec::new()
        }.send(&mut &*self.sock).unwrap_or(());
        Record::EndRequest {
            request_id: self.id,
            app_status: self.status,
            protocol_status: ProtocolStatus::RequestComplete,
        }.send(&mut &*self.sock).unwrap_or(());
    }
}

fn run_transport<F>(handler: F, transport: &mut Transport, thread_size: usize) -> io::Result<()>
    where F: Fn(Request) + Send + Sync + 'static
{
    let addrs: Option<HashSet<String>> = match env::var("FCGI_WEB_SERVER_ADDRS") {
        Ok(value) => Some(value.split(',').map(|s| s.to_owned()).collect()),
        Err(env::VarError::NotPresent) => None,
        Err(e) => Err(e).unwrap(),
    };

    let thread_pool = Rc::new(ThreadPool::new(thread_size));

    let handler = Arc::new(handler);

    loop {
        let sock = match transport.accept() {
            Ok(sock) => sock,
            Err(e) => panic!(e.to_string()),
        };
        let allow = match addrs {
            Some(ref addrs) => match sock.peer() {
                Ok(ref addr) => addrs.contains(addr),
                Err(_) => false,
            },
            None => true,
        };

        if allow {
            let thread_pool = thread_pool.clone();
            let handler = handler.clone();

            thread_pool.execute(move || {
                let sock = Rc::new(sock);
                loop {
                    let (request_id, role, keep_conn) = Request::begin(&sock).unwrap();
                    handler(Request::new(sock.clone(), request_id, role).unwrap());
                    if !keep_conn { break; }
                }
            });
        }
    }
}

#[cfg(unix)]
/// Runs as a FastCGI process with the given handler.
///
/// Available under Unix only. If you are using Windows, use `run_tcp` instead.
pub fn run<F>(handler: F, thread_size: usize) -> io::Result<()>
    where F: Fn(Request) + Send + Sync + 'static
{
    run_transport(handler, &mut Transport::new(), thread_size)
}

#[cfg(unix)]
/// Accepts requests from a user-supplied raw file descriptor. IPv4, IPv6, and
/// Unix domain sockets are supported.
///
/// Available under Unix only.
pub fn run_raw<F>(handler: F, raw_fd: ::std::os::unix::io::RawFd, thread_size: usize) -> io::Result<()>
    where F: Fn(Request) + Send + Sync + 'static
{
    run_transport(handler, &mut Transport::from_raw_fd(raw_fd), thread_size)
}

#[cfg(unix)]
/// Accepts requests from a user-supplied TCP listener.
pub fn run_tcp<F>(handler: F, listener: &TcpListener, thread_size: usize) -> io::Result<()>
    where F: Fn(Request) + Send + Sync + 'static
{
    use std::os::unix::io::AsRawFd;
    run_transport(handler, &mut Transport::from_raw_fd(listener.as_raw_fd()), thread_size)
}

#[cfg(windows)]
/// Accepts requests from a user-supplied TCP listener.
pub fn run_tcp<F>(handler: F, listener: &TcpListener, thread_size: usize) -> io::Result<()>
    where F: Fn(Request) + Send + Sync + 'static
{
    run_transport(handler, &mut Transport::from_tcp(&listener), thread_size)
}
