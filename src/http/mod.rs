use std::io::Write;
use std::sync::{Arc, Mutex};

use httparse;

use server::Stream;
use error::Result;
use error::Error;

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

pub struct Http<'a> {
    stream: &'a mut Stream,
}

impl<'a> Http<'a> {
    pub fn new(stream: &mut Stream) -> Http {
        Http {
            stream: stream,
        }
    }

    pub fn decode(&mut self) -> Result<Request> {
        //let mut stream = self.stream.lock().unwrap();
        let ref mut stream = self.stream;

        let (method, path, headers, amt) = {
            let mut headers = [httparse::EMPTY_HEADER; 24];
            let mut req = httparse::Request::new(&mut headers);
            let res = req.parse(&stream.reader)?;

            let amt = match res {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return Err(Error::Error("Http paser error".to_owned()))
            };

            let method = req.method.unwrap().to_owned();
            let path = req.path.unwrap().to_owned();
            let headers = req.headers.iter().map(|h| (h.name.to_owned(), String::from_utf8_lossy(h.value).to_string())).collect();

            (method, path, headers, amt)
        };

        let remote_addr = stream.remote_addr();

        Ok(Request::new(
            method.parse().unwrap(),
            path,
            headers,
            remote_addr,
            stream.reader.split_off(amt)
        ))
    }

    pub fn encode(&mut self, response: Response) {
        //let mut stream = self.stream.lock().unwrap();

        let ref mut stream = self.stream;

        let mut data = Vec::new();

        write!(data, "HTTP/1.1 {} {}\r\n", response.status_code.0, response.status_code.default_reason_phrase()).unwrap();
        write!(data, "Data: {}\r\n", HTTPDate::new().to_string()).unwrap();
        write!(data, "Server: Sincere\r\n").unwrap();

        if let Some(data_length) = response.data_length {
            write!(data, "Content-Length: {}\r\n", data_length).unwrap();
        }

        for (key, value) in response.headers {
            write!(data, "{}: {}\r\n", key, value).unwrap();
        }

        write!(data, "\r\n").unwrap();

        stream.write(&data).unwrap();
        stream.write(&response.data).unwrap();
    }
}
