use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use server::Stream;

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

pub struct Http {
    stream: Arc<Mutex<Stream>>,
}

impl Http {
    pub fn new(stream: Arc<Mutex<Stream>>) -> Http {
        Http {
            stream: stream,
        }
    }

    pub fn decode(&mut self) -> Request {
        let mut stream = self.stream.lock().unwrap();  
        
        let line = read_next_line(&mut stream);
        let mut words = line.trim().split(' ');

        let method = words.next();
        let path = words.next();
        let version = words.next();

        let method = method.unwrap();
        let path = path.unwrap();
        let version = version.unwrap();

        let mut headers: HashMap<String, String> = HashMap::new();

        loop {
            let line = read_next_line(&mut stream);

            if line.len() == 0 {
                break;
            }

            let mut header = line.trim().split(':');

            let key = header.next();
            let value = header.next();

            let key = key.unwrap();
            let value = value.unwrap().trim();

            headers.insert(key.to_owned(), value.to_owned());
        }

        let remote_addr = stream.remote_addr();

        Request::new(
            method.parse().unwrap(),
            path.to_owned(),
            version.to_owned(),
            headers,
            remote_addr,
            stream.to_vec()
        )
    }

    pub fn encode(&mut self, response: Response) {
        let mut stream = self.stream.lock().unwrap();

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



fn read_next_line(stream: &mut Stream) -> String {
    let mut buf = Vec::new();
    let mut prev_byte_was_cr = false;
    let mut index: usize = 0;

    for i in 0.. {
        let byte = stream.get(i).unwrap();

        if *byte == b'\n' && prev_byte_was_cr {
            buf.pop();
            index = i;
            break;
        }

        prev_byte_was_cr = *byte == b'\r';

        buf.push(*byte);
    }

    stream.split_off(index + 1);

    String::from_utf8(buf).unwrap()
}
