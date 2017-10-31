use std::io;
use std::fmt;
use std::fmt::Write;

use httparse;

use bytes::BytesMut;
use bytes::BufMut;
use tokio_io::codec::{Encoder, Decoder, Framed};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_proto::pipeline::ServerProto;

pub use self::request::Request;
pub use self::response::Response;
pub use self::http_code::StatusCode;
pub use self::http_method::Method;
use self::http_date::HTTPDate;

mod http_code;
mod http_date;
mod http_method;
mod request;
mod response;

pub struct Http;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for Http {
    type Request = Request;
    type Response = Response;
    type Transport = Framed<T, HttpCodec>;
    type BindTransport = io::Result<Framed<T, HttpCodec>>;

    fn bind_transport(&self, io: T) -> io::Result<Framed<T, HttpCodec>> {
        Ok(io.framed(HttpCodec))
    }
}

pub struct HttpCodec;

impl Decoder for HttpCodec {
    type Item = Request;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Request>> {
        decode(buf)
    }
}

impl Encoder for HttpCodec {
    type Item = Response;
    type Error = io::Error;

    fn encode(&mut self, response: Response, buf: &mut BytesMut) -> io::Result<()> {
        encode(response, buf)
    }
}

pub fn decode(buf: &mut BytesMut) -> io::Result<Option<Request>> {
    let (method, path, headers, amt) = {
        let mut headers = [httparse::EMPTY_HEADER; 24];
        let mut req = httparse::Request::new(&mut headers);
        let res = req.parse(buf).map_err(|e| {
            let msg = format!("failed to parse http request: {:?}", e);
            io::Error::new(io::ErrorKind::Other, msg)
        })?;

        let amt = match res {
            httparse::Status::Complete(amt) => amt,
            httparse::Status::Partial => return Ok(None)
        };

        let method = req.method.unwrap().to_owned();
        let path = req.path.unwrap().to_owned();
        let headers = req.headers.iter().map(|h| (h.name.to_owned(), String::from_utf8_lossy(h.value).to_string())).collect();

        (method, path, headers, amt)
    };

    Ok(Request::new(
        method.parse().unwrap(),
        path,
        headers,
        buf.split_to(amt).to_vec()
    ).into())
}

pub fn encode(response: Response, buf: &mut BytesMut) -> io::Result<()> {
    write!(FastWrite(buf), "\
        HTTP/1.1 {} {}\r\n\
        Server: Sincere\r\n\
        Data: {}\r\n\
    ", response.status_code.0,
        response.status_code.default_reason_phrase(),
        HTTPDate::new().to_string()
    ).unwrap();

    if let Some(data_length) = response.data_length {
        write!(FastWrite(buf), "Content-Length: {}\r\n", data_length).unwrap();
    }

    for (key, value) in response.headers {
        write!(FastWrite(buf), "{}: {}\r\n", key, value).unwrap();
    }

    push(buf, "\r\n".as_bytes());
    push(buf, &response.data);
    
    Ok(())
}

fn push(buf: &mut BytesMut, data: &[u8]) {
    buf.reserve(data.len());
    unsafe {
        buf.bytes_mut()[..data.len()].copy_from_slice(data);
        buf.advance_mut(data.len());
    }
}

struct FastWrite<'a>(&'a mut BytesMut);

impl<'a> fmt::Write for FastWrite<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        push(&mut *self.0, s.as_bytes());
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        fmt::write(self, args)
    }
}
